use crate::extension_tools;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({ path: "../extensions/api/wit", world: "squadron-extension" });

use squadron::api::types::{AgentState, Output, Request, Response};

pub const READ_AGENTS: &str = "agents:read";
pub const ARCHIVE_AGENTS: &str = "agents:archive";

const FUEL: u64 = 5_000_000_000;
const MEMORY: usize = 64 << 20;
const MAX_STATE: usize = 1 << 20;
const ANSWER: Duration = Duration::from_secs(150);
const FETCH: Duration = Duration::from_secs(30);
const MAX_BODY: usize = 8 << 20;
const WINDOW: Duration = Duration::from_secs(10);
const MIN_TIMER: u32 = 10;

pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;

type Answers = Arc<Mutex<HashMap<String, mpsc::Sender<Result<Value, String>>>>>;

#[derive(Deserialize, Clone, Debug)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub project: String,
    pub dir: String,
}

impl From<AgentInfo> for Agent {
    fn from(a: AgentInfo) -> Self {
        Agent {
            id: a.id,
            name: a.name,
            project: a.project,
            dir: a.dir,
        }
    }
}

#[derive(Deserialize)]
struct Listed {
    #[serde(flatten)]
    agent: AgentInfo,
    status: String,
    since: u64,
    pinned: bool,
    remote: bool,
    shown: bool,
    branch: Option<String>,
    archivable: bool,
}

impl From<Listed> for AgentState {
    fn from(l: Listed) -> Self {
        AgentState {
            agent: l.agent.into(),
            status: l.status,
            since: l.since,
            pinned: l.pinned,
            remote: l.remote,
            shown: l.shown,
            branch: l.branch,
            archivable: l.archivable,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Spec {
    pub id: String,
    pub dir: String,
    pub wasm: String,
    pub allowed: Vec<String>,
}

enum Job {
    Event(String, Option<Agent>, String),
    Run(String, Option<Agent>, mpsc::Sender<Result<(), String>>),
    Tool(
        String,
        String,
        Option<Agent>,
        mpsc::Sender<Result<String, String>>,
    ),
    Allow(Vec<String>),
    Timer,
}

struct Loaded {
    tx: mpsc::Sender<Job>,
    path: PathBuf,
    modified: Option<SystemTime>,
}

struct Host {
    id: String,
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    file: PathBuf,
    data: PathBuf,
    state: BTreeMap<String, String>,
    allowed: Vec<String>,
    emit: Emit,
    answers: Answers,
    timer: Option<Instant>,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl Host {
    fn tell(&self, call: &str, args: Value) {
        let mut message = json!({"extension": self.id, "call": call});
        if let (Some(m), Some(a)) = (message.as_object_mut(), args.as_object()) {
            m.extend(a.clone());
        }
        (self.emit)(message);
    }

    fn may(&self, access: &str) -> Result<(), String> {
        if self.allowed.iter().any(|a| a == access) {
            return Ok(());
        }
        let wanted = access.trim_start_matches("agents:");
        Err(format!(
            "{access} is not allowed. List \"{wanted}\" in permissions.agents; the user then allows it."
        ))
    }

    fn ask(&self, call: &str, mut args: Value) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();
        self.answers.lock().unwrap().insert(id.clone(), tx);
        args["reply"] = json!(id);
        self.tell(call, args);
        let answer = rx.recv_timeout(WINDOW);
        self.answers.lock().unwrap().remove(&id);
        answer.map_err(|_| "Squadron did not answer.".to_string())?
    }
}

impl squadron::api::types::Host for Host {}

impl squadron::api::host::Host for Host {
    fn log(&mut self, message: String) {
        log::info!("extension {}: {message}", self.id);
    }

    fn notify(&mut self, title: String, body: String) {
        self.tell("notify", json!({"title": title, "body": body}));
    }

    fn insert_prompt(&mut self, agent: String, text: String) {
        self.tell("insert-prompt", json!({"agent": agent, "text": text}));
    }

    fn set_badge(&mut self, item: String, badge: Option<String>) {
        self.tell("set-badge", json!({"item": item, "value": badge}));
    }

    fn set_label(&mut self, item: String, label: Option<String>) {
        self.tell("set-label", json!({"item": item, "value": label}));
    }

    fn state_get(&mut self, key: String) -> Option<String> {
        self.state.get(&key).cloned()
    }

    fn state_set(&mut self, key: String, value: Option<String>) {
        let mut next = self.state.clone();
        match value {
            Some(v) => next.insert(key, v),
            None => next.remove(&key),
        };
        let Ok(bytes) = serde_json::to_vec(&next) else {
            return;
        };
        if bytes.len() > MAX_STATE {
            log::warn!(
                "extension {}: its state would pass 1 MB, the change is dropped",
                self.id
            );
            return;
        }
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&self.file, bytes) {
            log::warn!("extension {}: state not saved: {e}", self.id);
        }
        self.state = next;
    }

    fn exec(
        &mut self,
        program: String,
        args: Vec<String>,
        dir: Option<String>,
    ) -> Result<Output, String> {
        if !self.allowed.contains(&format!("exec:{program}")) {
            return Err(format!(
                "{program} is not allowed. List it in permissions.exec; the user then allows it."
            ));
        }
        let mut cmd = crate::process::command(&program);
        cmd.args(&args);
        if let Some(dir) = dir
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        {
            cmd.current_dir(dir);
        }
        let out = extension_tools::capture(cmd)?;
        Ok(Output {
            code: out.status.and_then(|s| s.code()).unwrap_or(-1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    fn panel_post(&mut self, panel: String, message: String) {
        self.tell("panel-post", json!({"panel": panel, "message": message}));
    }

    fn fetch(&mut self, request: Request) -> Result<Response, String> {
        let id = self.id.clone();
        fetch(&self.allowed, request, |name| {
            crate::forge::read_secret(&secret_key(&id, name)?)
        })
    }

    fn agents(&mut self) -> Result<Vec<AgentState>, String> {
        self.may(READ_AGENTS)?;
        let list: Vec<Listed> =
            serde_json::from_value(self.ask("agents", json!({}))?).map_err(|e| e.to_string())?;
        Ok(list.into_iter().map(AgentState::from).collect())
    }

    fn archive_agent(&mut self, agent: String) -> Result<(), String> {
        self.may(ARCHIVE_AGENTS)?;
        self.ask("archive-agent", json!({"agent": agent}))
            .map(|_| ())
    }

    fn data_dir(&mut self) -> String {
        if let Err(e) = std::fs::create_dir_all(&self.data) {
            log::warn!("extension {}: no data folder: {e}", self.id);
        }
        self.data.to_string_lossy().into_owned()
    }

    fn set_timer(&mut self, seconds: u32) {
        self.timer = Some(Instant::now() + Duration::from_secs(seconds.max(MIN_TIMER).into()));
    }
}

fn wait(rx: &mpsc::Receiver<Job>, timer: Option<Instant>) -> Option<Job> {
    let Some(due) = timer else {
        return rx.recv().ok();
    };
    match rx.recv_timeout(due.saturating_duration_since(Instant::now())) {
        Ok(job) => Some(job),
        Err(RecvTimeoutError::Timeout) => Some(Job::Timer),
        Err(RecvTimeoutError::Disconnected) => None,
    }
}

pub fn secret_key(id: &str, name: &str) -> Result<String, String> {
    let fine = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    if fine(id) && fine(name) {
        Ok(format!("extension.{id}.{name}"))
    } else {
        Err(format!("{name} is not a secret name."))
    }
}

fn checked(raw: &str, allowed: &[String]) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw).map_err(|e| format!("{raw}: {e}"))?;
    if url.scheme() != "https" {
        return Err("Only https:// addresses are allowed.".into());
    }
    let host = url.host_str().unwrap_or_default();
    if !allowed.iter().any(|a| a.strip_prefix("net:") == Some(host)) {
        return Err(format!(
            "{host} is not allowed. List it in permissions.network; the user then allows it."
        ));
    }
    Ok(url)
}

fn fill(
    value: &str,
    secret: &impl Fn(&str) -> Result<Option<String>, String>,
    used: &mut Vec<String>,
) -> Result<String, String> {
    let (mut out, mut rest) = (String::new(), value);
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else {
            break;
        };
        let name = &rest[start + 2..start + 2 + len];
        let found = secret(name)?.ok_or_else(|| {
            format!("The secret {name} is not set. Set it in Settings › Extensions.")
        })?;
        out.push_str(&rest[..start]);
        out.push_str(&found);
        used.push(found);
        rest = &rest[start + 4 + len..];
    }
    out.push_str(rest);
    Ok(out)
}

fn scrub(text: String, used: &[String]) -> String {
    used.iter()
        .filter(|u| !u.is_empty())
        .fold(text, |text, u| text.replace(u.as_str(), "[secret]"))
}

fn why(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut text = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        text = format!("{text}: {cause}");
        source = cause.source();
    }
    text
}

fn fetch(
    allowed: &[String],
    request: Request,
    secret: impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<Response, String> {
    let url = checked(&request.url, allowed)?;
    let method = reqwest::Method::from_bytes(request.method.to_uppercase().as_bytes())
        .map_err(|_| format!("{} is not an HTTP method.", request.method))?;
    let mut used = Vec::new();
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &request.headers {
        let value = fill(value, &secret, &mut used)?;
        let bad = || format!("The header {name} is not valid.");
        headers.append(
            reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| bad())?,
            reqwest::header::HeaderValue::from_str(&value).map_err(|_| bad())?,
        );
    }
    let hosts = allowed.to_vec();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(FETCH)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("Too many redirects.")
            } else if checked(attempt.url().as_str(), &hosts).is_ok() {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(why)?;
    let mut send = client.request(method, url).headers(headers);
    if let Some(body) = request.body {
        send = send.body(body);
    }
    tauri::async_runtime::block_on(async {
        let mut answer = send.send().await.map_err(why)?;
        let headers = answer
            .headers()
            .iter()
            .map(|(k, v)| {
                let v = String::from_utf8_lossy(v.as_bytes()).into_owned();
                (k.to_string(), scrub(v, &used))
            })
            .collect();
        let mut body = Vec::new();
        while let Some(chunk) = answer.chunk().await.map_err(why)? {
            if body.len() + chunk.len() > MAX_BODY {
                return Err("The response is larger than 8 MB.".into());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Response {
            status: answer.status().as_u16(),
            headers,
            body: scrub(String::from_utf8_lossy(&body).into_owned(), &used),
        })
    })
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    host: Host,
) -> wasmtime::Result<(Store<Host>, SquadronExtension)> {
    let mut linker = Linker::<Host>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    SquadronExtension::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
    let mut store = Store::new(engine, host);
    store.limiter(|h| &mut h.limits);
    store.set_fuel(FUEL)?;
    let bindings = SquadronExtension::instantiate(&mut store, component, &linker)?;
    Ok((store, bindings))
}

pub struct Runtime {
    engine: Engine,
    loaded: Mutex<HashMap<String, Loaded>>,
    states: PathBuf,
    emit: Emit,
    answers: Answers,
}

fn resolve(dir: &str, wasm: &str) -> Result<PathBuf, String> {
    let root = Path::new(dir).canonicalize().map_err(|e| e.to_string())?;
    let path = root
        .join(wasm)
        .canonicalize()
        .map_err(|_| format!("{wasm} is missing. Build the extension first."))?;
    if !path.starts_with(&root) {
        return Err("The wasm file must stay inside the extension folder.".into());
    }
    Ok(path)
}

fn trapped(e: wasmtime::Error) -> String {
    format!("The extension stopped: {e:#}")
}

impl Runtime {
    pub fn new(
        states: PathBuf,
        emit: impl Fn(Value) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let mut config = Config::new();
        config.consume_fuel(true);
        Ok(Self {
            engine: Engine::new(&config).map_err(|e| e.to_string())?,
            loaded: Default::default(),
            states,
            emit: Arc::new(emit),
            answers: Default::default(),
        })
    }

    fn data_of(&self, id: &str) -> PathBuf {
        self.states.with_file_name("extensions-data").join(id)
    }

    pub fn data_dir(&self, id: &str) -> Result<PathBuf, String> {
        if !crate::extensions::valid_id(id) {
            return Err(format!("{id} is not an extension."));
        }
        let dir = self.data_of(id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }

    pub fn answer(&self, reply: &str, answer: Result<Value, String>) {
        if let Some(tx) = self.answers.lock().unwrap().remove(reply) {
            let _ = tx.send(answer);
        }
    }

    pub fn sync(&self, specs: Vec<Spec>) -> HashMap<String, String> {
        let mut errors = HashMap::new();
        let mut loaded = self.loaded.lock().unwrap();
        loaded.retain(|id, _| specs.iter().any(|s| &s.id == id));
        for spec in specs {
            let path = match resolve(&spec.dir, &spec.wasm) {
                Ok(path) => path,
                Err(e) => {
                    loaded.remove(&spec.id);
                    errors.insert(spec.id, e);
                    continue;
                }
            };
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if let Some(l) = loaded
                .get(&spec.id)
                .filter(|l| l.path == path && l.modified == modified)
            {
                let _ = l.tx.send(Job::Allow(spec.allowed));
                continue;
            }
            match self.start(&spec, &path) {
                Ok(tx) => {
                    loaded.insert(spec.id, Loaded { tx, path, modified });
                }
                Err(e) => {
                    loaded.remove(&spec.id);
                    errors.insert(spec.id, e);
                }
            }
        }
        errors
    }

    fn start(&self, spec: &Spec, path: &Path) -> Result<mpsc::Sender<Job>, String> {
        let component = Component::from_file(&self.engine, path).map_err(|e| format!("{e:#}"))?;
        let engine = self.engine.clone();
        let (id, emit, answers, file, data) = (
            spec.id.clone(),
            self.emit.clone(),
            self.answers.clone(),
            self.states.join(format!("{}.json", spec.id)),
            self.data_of(&spec.id),
        );
        let mut allowed = spec.allowed.clone();
        let host = move |allowed: &Vec<String>| Host {
            id: id.clone(),
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new().memory_size(MEMORY).build(),
            state: std::fs::read(&file)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default(),
            file: file.clone(),
            data: data.clone(),
            allowed: allowed.clone(),
            emit: emit.clone(),
            answers: answers.clone(),
            timer: None,
        };
        let (ready_tx, ready_rx) = mpsc::channel();
        let (tx, rx) = mpsc::channel::<Job>();
        let (report, id) = (self.emit.clone(), spec.id.clone());
        std::thread::spawn(move || {
            let mut current = match instantiate(&engine, &component, host(&allowed)) {
                Ok(current) => current,
                Err(e) => return ready_tx.send(Err(format!("{e:#}"))).unwrap_or(()),
            };
            let _ = ready_tx.send(Ok(()));
            let mut job = Some(Job::Event("started".into(), None, "{}".into()));
            loop {
                let timer = current.0.data().timer;
                let Some(next) = job.take().or_else(|| wait(&rx, timer)) else {
                    break;
                };
                let (store, bindings) = &mut current;
                let _ = store.set_fuel(FUEL);
                let failure = match next {
                    Job::Allow(list) => {
                        allowed = list.clone();
                        store.data_mut().allowed = list;
                        None
                    }
                    Job::Timer => {
                        store.data_mut().timer = None;
                        bindings
                            .call_on_event(&mut *store, "timer", None, "{}")
                            .err()
                            .map(trapped)
                    }
                    Job::Event(name, agent, data) => bindings
                        .call_on_event(&mut *store, &name, agent.as_ref(), &data)
                        .err()
                        .map(trapped),
                    Job::Run(action, agent, reply) => {
                        match bindings.call_run(&mut *store, &action, agent.as_ref()) {
                            Ok(result) => {
                                let _ = reply.send(result);
                                None
                            }
                            Err(e) => {
                                let e = trapped(e);
                                let _ = reply.send(Err(e.clone()));
                                Some(e)
                            }
                        }
                    }
                    Job::Tool(name, input, agent, reply) => {
                        match bindings.call_call_tool(&mut *store, &name, &input, agent.as_ref()) {
                            Ok(result) => {
                                let _ = reply.send(result);
                                None
                            }
                            Err(e) => {
                                let e = trapped(e);
                                let _ = reply.send(Err(e.clone()));
                                Some(e)
                            }
                        }
                    }
                };
                if let Some(error) = failure {
                    report(json!({"extension": id, "call": "error", "error": error}));
                    let timer = current.0.data().timer;
                    match instantiate(&engine, &component, host(&allowed)) {
                        Ok(mut fresh) => {
                            fresh.0.data_mut().timer = timer;
                            current = fresh;
                        }
                        Err(e) => {
                            report(
                                json!({"extension": id, "call": "error", "error": format!("{e:#}")}),
                            );
                            break;
                        }
                    }
                }
            }
        });
        ready_rx
            .recv()
            .map_err(|_| "The extension did not start.".to_string())??;
        Ok(tx)
    }

    pub fn event(&self, only: Option<&str>, name: &str, agent: Option<AgentInfo>, data: &str) {
        for (_, l) in self
            .loaded
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| only.is_none_or(|only| only == id.as_str()))
        {
            let _ = l.tx.send(Job::Event(
                name.into(),
                agent.clone().map(Agent::from),
                data.into(),
            ));
        }
    }

    fn sender(&self, id: &str) -> Result<mpsc::Sender<Job>, String> {
        self.loaded
            .lock()
            .unwrap()
            .get(id)
            .map(|l| l.tx.clone())
            .ok_or_else(|| format!("The extension {id} is not running."))
    }

    pub fn run(&self, id: &str, action: &str, agent: Option<AgentInfo>) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        self.sender(id)?
            .send(Job::Run(action.into(), agent.map(Agent::from), reply))
            .map_err(|_| "The extension stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The extension did not answer.".to_string())?
    }

    pub fn tool(
        &self,
        id: &str,
        name: &str,
        input: &str,
        agent: Option<AgentInfo>,
    ) -> Result<String, String> {
        let (reply, answer) = mpsc::channel();
        self.sender(id)?
            .send(Job::Tool(
                name.into(),
                input.into(),
                agent.map(Agent::from),
                reply,
            ))
            .map_err(|_| "The extension stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The extension did not answer.".to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn hello() -> Option<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../extensions/examples/hello");
        let built = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2", "--quiet"])
            .current_dir(&dir)
            .status()
            .is_ok_and(|s| s.success());
        built.then(|| dir.canonicalize().unwrap())
    }

    #[test]
    #[ignore = "builds the example extension, needs the wasm32-wasip2 target"]
    fn runs_the_example_mod() {
        let dir = hello().expect("the example extension builds");
        let states = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        let told = Arc::new(Mutex::new(Vec::<Value>::new()));
        let record = told.clone();
        let runtime =
            Runtime::new(states.clone(), move |v| record.lock().unwrap().push(v)).unwrap();
        let spec = |allowed: Vec<String>| Spec {
            id: "hello".into(),
            dir: dir.to_string_lossy().into(),
            wasm: "target/wasm32-wasip2/release/hello.wasm".into(),
            allowed,
        };
        assert!(runtime.sync(vec![spec(vec![])]).is_empty());
        let agent = AgentInfo {
            id: "a1".into(),
            name: "Ada".into(),
            project: dir.to_string_lossy().into(),
            dir: dir.to_string_lossy().into(),
        };

        runtime.run("hello", "count", None).unwrap();
        runtime.run("hello", "count", None).unwrap();
        assert_eq!(
            runtime.tool("hello", "clicks", "{}", None).unwrap(),
            "The Count button was clicked 2 times."
        );
        runtime.run("hello", "greet", Some(agent.clone())).unwrap();
        assert!(
            runtime
                .run("hello", "nope", None)
                .unwrap_err()
                .contains("Unknown action")
        );
        assert!(
            runtime
                .tool("hello", "changed_files", "{}", Some(agent.clone()))
                .unwrap_err()
                .contains("git is not allowed")
        );

        assert!(runtime.sync(vec![spec(vec!["exec:git".into()])]).is_empty());
        let changed = runtime.tool("hello", "changed_files", "{}", Some(agent.clone()));
        assert!(changed.is_ok(), "{changed:?}");
        runtime.event(Some("other"), "agent.done", Some(agent.clone()), "{}");
        runtime.event(None, "agent.done", Some(agent), "{}");
        runtime.run("hello", "count", None).unwrap();
        runtime.event(
            Some("hello"),
            "panel.message",
            None,
            r#"{"panel":"main","message":{"type":"count"}}"#,
        );
        runtime.run("hello", "nope", None).unwrap_err();

        let told = told.lock().unwrap().clone();
        let calls: Vec<_> = told
            .iter()
            .map(|v| v["call"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            calls,
            [
                "set-badge",
                "set-badge",
                "panel-post",
                "set-badge",
                "panel-post",
                "insert-prompt",
                "notify",
                "set-badge",
                "panel-post",
                "set-badge",
                "panel-post"
            ]
        );
        assert_eq!(told[0]["value"], "0");
        assert_eq!(told[5]["text"], "Hello Ada, what are you working on?");
        assert_eq!(told[10]["panel"], "main");
        assert_eq!(told[10]["message"], r#"{"count":4}"#);
        assert_eq!(
            std::fs::read_to_string(states.join("hello.json")).unwrap(),
            r#"{"count":"4"}"#
        );

        let restarted = Runtime::new(states, |_| {}).unwrap();
        assert!(restarted.sync(vec![spec(vec![])]).is_empty());
        assert_eq!(
            restarted.tool("hello", "clicks", "{}", None).unwrap(),
            "The Count button was clicked 4 times."
        );
    }

    fn host(
        allowed: &[&str],
        window: impl Fn(&str, &Value) -> Result<Value, String> + Send + Sync + 'static,
    ) -> (Host, Arc<Mutex<Vec<Value>>>) {
        let told = Arc::new(Mutex::new(Vec::<Value>::new()));
        let answers = Answers::default();
        let (record, pending) = (told.clone(), answers.clone());
        let host = Host {
            id: "jira".into(),
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new().build(),
            file: std::env::temp_dir().join(format!("squadron-wasm-{}.json", uuid::Uuid::new_v4())),
            data: std::env::temp_dir().join(format!("squadron-data-{}", uuid::Uuid::new_v4())),
            state: BTreeMap::new(),
            allowed: allowed.iter().map(|a| a.to_string()).collect(),
            emit: Arc::new(move |v: Value| {
                record.lock().unwrap().push(v.clone());
                let reply = v["reply"].as_str().unwrap_or_default();
                if let Some(tx) = pending.lock().unwrap().remove(reply) {
                    let _ = tx.send(window(v["call"].as_str().unwrap(), &v));
                }
            }),
            answers,
            timer: None,
        };
        (host, told)
    }

    #[test]
    fn lists_and_archives_agents_only_with_their_permission() {
        use squadron::api::host::Host as _;
        let window = |call: &str, v: &Value| match (call, v["agent"].as_str()) {
            ("agents", _) => Ok(
                json!([{"id": "a1", "name": "RING-1", "project": "/p", "dir": "/p/w", "status": "done", "since": 1_700_000_000_000u64, "pinned": false, "remote": false, "shown": false, "branch": "ring-1", "archivable": true}]),
            ),
            ("archive-agent", Some("a1")) => Ok(Value::Null),
            _ => Err("RING-2 stays: it works.".into()),
        };

        let (mut denied, told) = host(&["exec:git"], window);
        assert!(denied.agents().unwrap_err().contains("agents:read"));
        assert!(
            denied
                .archive_agent("a1".into())
                .unwrap_err()
                .contains(r#""archive" in permissions.agents"#)
        );
        assert!(told.lock().unwrap().is_empty());

        let (mut host, told) = host(&[READ_AGENTS, ARCHIVE_AGENTS], window);
        let agents = host.agents().unwrap();
        assert_eq!(agents[0].agent.dir, "/p/w");
        assert_eq!(agents[0].branch.as_deref(), Some("ring-1"));
        assert!(agents[0].archivable);
        host.archive_agent("a1".into()).unwrap();
        assert_eq!(
            host.archive_agent("a2".into()).unwrap_err(),
            "RING-2 stays: it works."
        );
        let calls: Vec<_> = told
            .lock()
            .unwrap()
            .iter()
            .map(|v| (v["call"].clone(), v["agent"].clone()))
            .collect();
        assert_eq!(
            calls,
            [
                (json!("agents"), Value::Null),
                (json!("archive-agent"), json!("a1")),
                (json!("archive-agent"), json!("a2"))
            ]
        );
        assert!(host.answers.lock().unwrap().is_empty());
    }

    #[test]
    fn gives_up_when_the_window_does_not_answer() {
        use squadron::api::host::Host as _;
        let (mut host, _) = host(&[READ_AGENTS], |_, _| Ok(json!([])));
        host.emit = Arc::new(|_| {});
        let started = Instant::now();
        assert_eq!(host.agents().unwrap_err(), "Squadron did not answer.");
        assert!(started.elapsed() >= WINDOW);
        assert!(host.answers.lock().unwrap().is_empty());
    }

    #[test]
    fn rings_the_timer_once_after_ten_seconds_at_least() {
        use squadron::api::host::Host as _;
        let (mut host, _) = host(&[], |_, _| Ok(Value::Null));
        host.set_timer(1);
        let left = host.timer.unwrap() - Instant::now();
        assert!(left > Duration::from_secs(9) && left <= Duration::from_secs(10));

        let (tx, rx) = mpsc::channel();
        assert!(matches!(wait(&rx, Some(Instant::now())), Some(Job::Timer)));
        tx.send(Job::Allow(vec![])).unwrap();
        assert!(matches!(
            wait(&rx, Some(Instant::now() + Duration::from_secs(60))),
            Some(Job::Allow(_))
        ));
        drop(tx);
        assert!(wait(&rx, None).is_none());
    }

    #[test]
    fn fetches_only_https_on_allowed_hosts() {
        let allowed = ["net:api.example.com".to_string(), "exec:git".to_string()];
        assert!(checked("https://api.example.com/v1?q=1", &allowed).is_ok());
        assert!(checked("https://API.Example.com:8443/", &allowed).is_ok());
        for refused in [
            "http://api.example.com/",
            "https://evil.com/",
            "https://api.example.com@evil.com/",
            "https://api.example.com.evil.com/",
            "https://git/",
            "file:///etc/passwd",
        ] {
            assert!(checked(refused, &allowed).is_err(), "{refused}");
        }
        let request = |url: &str| Request {
            method: "GET".into(),
            url: url.into(),
            headers: vec![],
            body: None,
        };
        let none = |_: &str| Ok(None);
        assert!(
            fetch(&allowed, request("https://evil.com/"), none)
                .unwrap_err()
                .contains("permissions.network")
        );
    }

    #[test]
    fn fills_secrets_in_headers_and_hides_them_in_answers() {
        let secret = |name: &str| Ok((name == "token").then(|| "s3cr3t".to_string()));
        let mut used = vec![];
        assert_eq!(
            fill("Bearer {{token}}", &secret, &mut used).unwrap(),
            "Bearer s3cr3t"
        );
        assert_eq!(fill("plain {{", &secret, &mut used).unwrap(), "plain {{");
        assert!(
            fill("{{other}}", &secret, &mut used)
                .unwrap_err()
                .contains("Settings › Extensions")
        );
        assert_eq!(
            scrub("echo: Bearer s3cr3t".into(), &used),
            "echo: Bearer [secret]"
        );
        assert_eq!(
            secret_key("hello", "token").unwrap(),
            "extension.hello.token"
        );
        assert!(secret_key("hello", "../x").is_err());
        assert!(secret_key("hello", "").is_err());
    }

    #[test]
    #[ignore = "needs the network"]
    fn fetches_a_real_page() {
        let answer = fetch(
            &["net:api.github.com".into()],
            Request {
                method: "get".into(),
                url: "https://api.github.com/zen".into(),
                headers: vec![("User-Agent".into(), "squadron-test".into())],
                body: None,
            },
            |_| Ok(None),
        )
        .unwrap();
        assert_eq!(answer.status, 200, "{}", answer.body);
        assert!(!answer.body.is_empty());
    }

    #[test]
    fn refuses_a_wasm_file_outside_the_extension_folder() {
        let dir = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("extension")).unwrap();
        std::fs::write(dir.join("outside.wasm"), b"").unwrap();
        let extension_dir = dir.join("extension").to_string_lossy().to_string();
        assert!(
            resolve(&extension_dir, "../outside.wasm")
                .unwrap_err()
                .contains("inside")
        );
        assert!(
            resolve(&extension_dir, "missing.wasm")
                .unwrap_err()
                .contains("Build the extension first")
        );
        let runtime = Runtime::new(dir.join("states"), |_| {}).unwrap();
        std::fs::write(dir.join("extension/broken.wasm"), b"not wasm").unwrap();
        let errors = runtime.sync(vec![Spec {
            id: "broken".into(),
            dir: extension_dir,
            wasm: "broken.wasm".into(),
            allowed: vec![],
        }]);
        assert!(errors.contains_key("broken"));
        assert!(
            runtime
                .run("broken", "x", None)
                .unwrap_err()
                .contains("not running")
        );
    }

    #[test]
    fn gives_each_extension_a_data_folder_next_to_its_state() {
        let root = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        let runtime = Runtime::new(root.join("extensions-state"), |_| {}).unwrap();
        let dir = runtime.data_dir("team-share").unwrap();
        assert_eq!(dir, root.join("extensions-data").join("team-share"));
        assert!(dir.is_dir());
        assert!(runtime.data_dir("../escape").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
