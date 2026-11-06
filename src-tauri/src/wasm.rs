use crate::modtools;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({ path: "../mods/api/wit", world: "squadron-mod" });

use squadron::api::types::Output;

const FUEL: u64 = 5_000_000_000;
const MEMORY: usize = 64 << 20;
const MAX_STATE: usize = 1 << 20;
const ANSWER: Duration = Duration::from_secs(150);

pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;

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
    state: BTreeMap<String, String>,
    allowed: Vec<String>,
    emit: Emit,
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
        let mut message = json!({"mod": self.id, "call": call});
        if let (Some(m), Some(a)) = (message.as_object_mut(), args.as_object()) {
            m.extend(a.clone());
        }
        (self.emit)(message);
    }
}

impl squadron::api::types::Host for Host {}

impl squadron::api::host::Host for Host {
    fn log(&mut self, message: String) {
        log::info!("mod {}: {message}", self.id);
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
                "mod {}: its state would pass 1 MB, the change is dropped",
                self.id
            );
            return;
        }
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&self.file, bytes) {
            log::warn!("mod {}: state not saved: {e}", self.id);
        }
        self.state = next;
    }

    fn exec(
        &mut self,
        program: String,
        args: Vec<String>,
        dir: Option<String>,
    ) -> Result<Output, String> {
        if !self.allowed.contains(&program) {
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
        let out = modtools::capture(cmd)?;
        Ok(Output {
            code: out.status.and_then(|s| s.code()).unwrap_or(-1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    host: Host,
) -> wasmtime::Result<(Store<Host>, SquadronMod)> {
    let mut linker = Linker::<Host>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    SquadronMod::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
    let mut store = Store::new(engine, host);
    store.limiter(|h| &mut h.limits);
    store.set_fuel(FUEL)?;
    let bindings = SquadronMod::instantiate(&mut store, component, &linker)?;
    Ok((store, bindings))
}

pub struct Runtime {
    engine: Engine,
    loaded: Mutex<HashMap<String, Loaded>>,
    states: PathBuf,
    emit: Emit,
}

fn resolve(dir: &str, wasm: &str) -> Result<PathBuf, String> {
    let root = Path::new(dir).canonicalize().map_err(|e| e.to_string())?;
    let path = root
        .join(wasm)
        .canonicalize()
        .map_err(|_| format!("{wasm} is missing. Build the mod first."))?;
    if !path.starts_with(&root) {
        return Err("The wasm file must stay inside the mod folder.".into());
    }
    Ok(path)
}

fn trapped(e: wasmtime::Error) -> String {
    format!("The mod stopped: {e:#}")
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
        })
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
        let (id, emit, file) = (
            spec.id.clone(),
            self.emit.clone(),
            self.states.join(format!("{}.json", spec.id)),
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
            allowed: allowed.clone(),
            emit: emit.clone(),
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
                let Some(next) = job.take().or_else(|| rx.recv().ok()) else {
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
                    report(json!({"mod": id, "call": "error", "error": error}));
                    match instantiate(&engine, &component, host(&allowed)) {
                        Ok(fresh) => current = fresh,
                        Err(e) => {
                            report(json!({"mod": id, "call": "error", "error": format!("{e:#}")}));
                            break;
                        }
                    }
                }
            }
        });
        ready_rx
            .recv()
            .map_err(|_| "The mod did not start.".to_string())??;
        Ok(tx)
    }

    pub fn event(&self, name: &str, agent: Option<AgentInfo>, data: &str) {
        for l in self.loaded.lock().unwrap().values() {
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
            .ok_or_else(|| format!("The mod {id} is not running."))
    }

    pub fn run(&self, id: &str, action: &str, agent: Option<AgentInfo>) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        self.sender(id)?
            .send(Job::Run(action.into(), agent.map(Agent::from), reply))
            .map_err(|_| "The mod stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The mod did not answer.".to_string())?
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
            .map_err(|_| "The mod stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The mod did not answer.".to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn hello() -> Option<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mods/examples/hello");
        let built = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2", "--quiet"])
            .current_dir(&dir)
            .status()
            .is_ok_and(|s| s.success());
        built.then(|| dir.canonicalize().unwrap())
    }

    #[test]
    #[ignore = "builds the example mod, needs the wasm32-wasip2 target"]
    fn runs_the_example_mod() {
        let dir = hello().expect("the example mod builds");
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

        assert!(runtime.sync(vec![spec(vec!["git".into()])]).is_empty());
        let changed = runtime.tool("hello", "changed_files", "{}", Some(agent.clone()));
        assert!(changed.is_ok(), "{changed:?}");
        runtime.event("agent.done", Some(agent), "{}");
        runtime.run("hello", "count", None).unwrap();

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
                "set-badge",
                "insert-prompt",
                "notify",
                "set-badge"
            ]
        );
        assert_eq!(told[0]["value"], "0");
        assert_eq!(told[3]["text"], "Hello Ada, what are you working on?");
        assert_eq!(
            std::fs::read_to_string(states.join("hello.json")).unwrap(),
            r#"{"count":"3"}"#
        );

        let restarted = Runtime::new(states, |_| {}).unwrap();
        assert!(restarted.sync(vec![spec(vec![])]).is_empty());
        assert_eq!(
            restarted.tool("hello", "clicks", "{}", None).unwrap(),
            "The Count button was clicked 3 times."
        );
    }

    #[test]
    fn refuses_a_wasm_file_outside_the_mod_folder() {
        let dir = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("mod")).unwrap();
        std::fs::write(dir.join("outside.wasm"), b"").unwrap();
        let mod_dir = dir.join("mod").to_string_lossy().to_string();
        assert!(
            resolve(&mod_dir, "../outside.wasm")
                .unwrap_err()
                .contains("inside")
        );
        assert!(
            resolve(&mod_dir, "missing.wasm")
                .unwrap_err()
                .contains("Build the mod first")
        );
        let runtime = Runtime::new(dir.join("states"), |_| {}).unwrap();
        std::fs::write(dir.join("mod/broken.wasm"), b"not wasm").unwrap();
        let errors = runtime.sync(vec![Spec {
            id: "broken".into(),
            dir: mod_dir,
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
}
