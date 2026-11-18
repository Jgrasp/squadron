use crate::activity::{self, ACTIVITY, Admitted, Fed, READ_PROJECTS, READ_USAGE, SHOW_AGENTS};
use crate::extension_tools;
use crate::provider::{ForgeSpec, Provider};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable, TypedFunc};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({ path: "../extensions/api/wit", world: "squadron-core" });

use crate::extensions::Account as Stored;
use squadron::api::types::{
    Account, AgentActivity, AgentState, BytesResponse, ClaudeUsage, Group, Output, Project,
    Request, Response, SavedFile, UsageWindow,
};

pub const READ_AGENTS: &str = "agents:read";
pub const ARCHIVE_AGENTS: &str = "agents:archive";

const FUEL: u64 = 5_000_000_000;
const MEMORY: usize = 64 << 20;
const MAX_STATE: usize = 1 << 20;
const ANSWER: Duration = Duration::from_secs(150);
const FETCH: Duration = Duration::from_secs(30);
const MAX_BODY: usize = 8 << 20;
const DOWNLOAD: Duration = Duration::from_secs(120);
const MAX_DOWNLOAD: usize = 200 << 20;
const WINDOW: Duration = Duration::from_secs(10);
const MIN_TIMER: u32 = 10;
const FLUSH: Duration = Duration::from_secs(1);
const SHOW_EVERY: Duration = Duration::from_secs(1);
const POSTS: u32 = 200;
const RECENT: Duration = Duration::from_millis(500);
const PAGE_EVENTS: u32 = 100;
const MAX_PAGE_EVENT: usize = 1 << 20;
const QUEUED_PAGE_EVENTS: usize = 50;
const QUEUED_PAGE_BYTES: usize = 8 << 20;
const POSTED: usize = 4 << 20;
const POOL: usize = 4;

pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;

type AccountsOf = Arc<dyn Fn(&str) -> Vec<Stored> + Send + Sync>;

type OptionsOf = Arc<dyn Fn(&str) -> Value + Send + Sync>;

#[derive(Clone)]
pub struct Hooks {
    pub claude: Arc<dyn Fn() -> String + Send + Sync>,
    pub accounts: AccountsOf,
    pub options: OptionsOf,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            claude: Arc::new(|| "claude".into()),
            accounts: Arc::new(|_| Vec::new()),
            options: Arc::new(|_| json!({})),
        }
    }
}

type Answers = Arc<Mutex<HashMap<String, mpsc::Sender<Result<Value, String>>>>>;

type CheckAccount = TypedFunc<(Account,), (Result<String, String>,)>;

type Forge = TypedFunc<(String, String), (Result<String, String>,)>;

pub type Answer = Result<Result<String, String>, String>;

type State = Arc<Mutex<BTreeMap<String, String>>>;

pub struct Trial {
    pub account: Stored,
    pub secrets: BTreeMap<String, String>,
}

impl Trial {
    fn secret(&self, key: &str) -> Option<String> {
        let (account, field) = key.split_once('.')?;
        (account == self.account.id)
            .then(|| self.secrets.get(field).cloned())
            .flatten()
    }
}

fn with_trial(mut accounts: Vec<Stored>, trial: Option<&Trial>) -> Vec<Stored> {
    if let Some(trial) = trial {
        accounts.retain(|a| a.id != trial.account.id);
        accounts.push(trial.account.clone());
    }
    accounts
}

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

impl From<activity::Project> for Project {
    fn from(p: activity::Project) -> Self {
        Project {
            id: p.id,
            name: p.name,
            color: p.color,
            group: p.group,
            archived: p.archived,
            active: p.active,
            week_agents: p.week_agents,
            agents: p.agents,
        }
    }
}

impl From<activity::Group> for Group {
    fn from(g: activity::Group) -> Self {
        Group {
            id: g.id,
            name: g.name,
            parent: g.parent,
        }
    }
}

impl From<activity::Agent> for AgentActivity {
    fn from(a: activity::Agent) -> Self {
        AgentActivity {
            id: a.id,
            project: a.project,
            name: a.name,
            status: a.status,
            waiting: a.waiting,
            tool: a.tool,
            error: a.error,
            by: a.by,
            archived: a.archived,
            since: a.since,
        }
    }
}

impl From<activity::Window> for UsageWindow {
    fn from(w: activity::Window) -> Self {
        UsageWindow {
            utilization: w.utilization,
            resets_at: w.resets_at,
        }
    }
}

impl From<activity::Usage> for ClaudeUsage {
    fn from(u: activity::Usage) -> Self {
        ClaudeUsage {
            five_hour: u.five_hour.map(Into::into),
            seven_day: u.seven_day.map(Into::into),
            api: u.api,
            retries: u.retries,
            since: u.since,
        }
    }
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
    #[serde(default)]
    pub panels: Vec<String>,
    #[serde(default)]
    pub forge: Option<ForgeSpec>,
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
    Check(Trial, mpsc::Sender<Result<Result<String, String>, String>>),
    Allow(Vec<String>, Vec<String>),
    Timer,
    Fed,
}

struct Loaded {
    tx: mpsc::Sender<Job>,
    path: PathBuf,
    modified: Option<SystemTime>,
    allowed: Vec<String>,
    inbox: Inbox,
    queued: Queued,
}

type Inbox = Arc<Mutex<Vec<Admitted>>>;

type Queued = Arc<(AtomicUsize, AtomicUsize)>;

fn page(name: &str) -> bool {
    matches!(name, "panel.message" | "settings.message")
}

type Running = Arc<Mutex<HashMap<String, Loaded>>>;

struct Host {
    id: String,
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    file: PathBuf,
    data: PathBuf,
    state: State,
    allowed: Vec<String>,
    emit: Emit,
    answers: Answers,
    timer: Option<Instant>,
    hooks: Hooks,
    trial: Option<Trial>,
    panels: Vec<String>,
    kept: Arc<Mutex<Kept>>,
}

#[derive(Default)]
struct Kept {
    shown: Option<Instant>,
    posts: HashMap<String, Posted>,
    recent: HashMap<&'static str, (Instant, Value)>,
    stray: bool,
}

struct Posted {
    since: Instant,
    count: u32,
    bytes: usize,
    warned: bool,
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
        let (kind, wanted) = access.split_once(':').unwrap_or(("agents", access));
        Err(format!(
            "{access} is not allowed. List \"{wanted}\" in permissions.{kind}; the user then allows it."
        ))
    }

    fn post(&mut self, call: &str, panel: Option<&str>, message: String) {
        let mut kept = self.kept.lock().unwrap();
        if let Some(panel) = panel.filter(|p| !self.panels.iter().any(|known| known == p)) {
            if !std::mem::replace(&mut kept.stray, true) {
                log::warn!(
                    "extension {}: no panel {panel:.64} in contributes.panels, its messages are dropped",
                    self.id
                );
            }
            return;
        }
        let now = Instant::now();
        let posted = kept
            .posts
            .entry(panel.unwrap_or(":settings").to_string())
            .or_insert(Posted {
                since: now,
                count: 0,
                bytes: 0,
                warned: false,
            });
        if now.duration_since(posted.since) >= Duration::from_secs(1) {
            *posted = Posted {
                since: now,
                count: 0,
                bytes: 0,
                warned: false,
            };
        }
        posted.count += 1;
        posted.bytes += message.len();
        if posted.count > POSTS || posted.bytes > POSTED {
            if !std::mem::replace(&mut posted.warned, true) {
                log::warn!(
                    "extension {}: more than {POSTS} messages or 4 MB in a second to one page, the rest of this second is dropped",
                    self.id
                );
            }
            return;
        }
        drop(kept);
        match panel {
            Some(panel) => self.tell(call, json!({"panel": panel, "message": message})),
            None => self.tell(call, json!({"message": message})),
        }
    }

    fn recently(&mut self, call: &'static str) -> Result<Value, String> {
        if let Some((at, value)) = self.kept.lock().unwrap().recent.get(call)
            && at.elapsed() < RECENT
        {
            return Ok(value.clone());
        }
        let value = self.ask(call, json!({}))?;
        self.kept
            .lock()
            .unwrap()
            .recent
            .insert(call, (Instant::now(), value.clone()));
        Ok(value)
    }

    fn located(&self, path: &Path) -> String {
        match std::env::home_dir().and_then(|h| path.strip_prefix(h).ok().map(Path::to_path_buf)) {
            Some(rest) if !sees_files(&self.allowed) => format!("~/{}", rest.to_string_lossy()),
            _ => path.to_string_lossy().into_owned(),
        }
    }

    fn listed<T: for<'de> Deserialize<'de>>(
        &mut self,
        call: &'static str,
    ) -> Result<Vec<T>, String> {
        let list: Vec<Value> =
            serde_json::from_value(self.recently(call)?).map_err(|e| e.to_string())?;
        Ok(list
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect())
    }

    fn ask(&self, call: &str, args: Value) -> Result<Value, String> {
        self.ask_within(call, args, WINDOW)
    }

    fn ask_within(&self, call: &str, mut args: Value, within: Duration) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel();
        self.answers.lock().unwrap().insert(id.clone(), tx);
        args["reply"] = json!(id);
        self.tell(call, args);
        let answer = rx.recv_timeout(within);
        self.answers.lock().unwrap().remove(&id);
        answer.map_err(|_| "Squadron did not answer.".to_string())?
    }

    fn launch(
        &self,
        program: String,
        args: Vec<String>,
        dir: Option<String>,
        input: Option<String>,
    ) -> Result<Output, String> {
        if !self.allowed.contains(&format!("exec:{program}")) {
            return Err(format!(
                "{program} is not allowed. List it in permissions.exec; the user then allows it."
            ));
        }
        let program = if program == "claude" {
            (self.hooks.claude)()
        } else {
            program
        };
        let mut cmd = crate::process::command(&program);
        cmd.args(&args);
        if let Some(dir) = dir
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        {
            cmd.current_dir(dir);
        }
        let out = extension_tools::capture_input(cmd, input.map(String::into_bytes))?;
        Ok(Output {
            code: out.status.and_then(|s| s.code()).unwrap_or(-1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    fn send(&self, request: Request, max: usize, timeout: Duration) -> Result<Sent, String> {
        let id = self.id.clone();
        let trial = self.trial.as_ref();
        let accounts = with_trial((self.hooks.accounts)(&self.id), trial);
        send(&self.allowed, &accounts, request, max, timeout, |key| {
            if let Some(value) = trial.and_then(|t| t.secret(key)) {
                return Ok(Some(value));
            }
            let key = match key.split_once('.') {
                Some((account, field)) => account_key(&id, account, field)?,
                None => secret_key(&id, key)?,
            };
            crate::forge::read_secret(&key)
        })
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
        self.state.lock().unwrap().get(&key).cloned()
    }

    fn state_set(&mut self, key: String, value: Option<String>) {
        let mut state = self.state.lock().unwrap();
        let mut next = state.clone();
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
        *state = next;
    }

    fn exec(
        &mut self,
        program: String,
        args: Vec<String>,
        dir: Option<String>,
    ) -> Result<Output, String> {
        self.launch(program, args, dir, None)
    }

    fn exec_input(
        &mut self,
        program: String,
        args: Vec<String>,
        dir: Option<String>,
        input: String,
    ) -> Result<Output, String> {
        self.launch(program, args, dir, Some(input))
    }

    fn panel_post(&mut self, panel: String, message: String) {
        self.post("panel-post", Some(&panel), message);
    }

    fn panel_open(&mut self, panel: String) -> Result<(), String> {
        if !self.panels.contains(&panel) {
            return Err(format!("No panel {panel:.64} in contributes.panels."));
        }
        self.ask_within("panel-open", json!({"panel": panel}), ANSWER)
            .map(|_| ())
    }

    fn fetch(&mut self, request: Request) -> Result<Response, String> {
        let sent = self.send(request, MAX_BODY, FETCH)?;
        Ok(Response {
            status: sent.status,
            headers: sent.headers,
            body: scrub(String::from_utf8_lossy(&sent.body).into_owned(), &sent.used),
        })
    }

    fn fetch_bytes(&mut self, request: Request) -> Result<BytesResponse, String> {
        let sent = self.send(request, MAX_BODY, FETCH)?;
        Ok(BytesResponse {
            status: sent.status,
            headers: sent.headers,
            body: scrub_bytes(sent.body, &sent.used),
        })
    }

    fn download(&mut self, request: Request, name: String) -> Result<SavedFile, String> {
        let file = data_file(&self.data, &name)?;
        let sent = self.send(request, MAX_DOWNLOAD, DOWNLOAD)?;
        let body = scrub_bytes(sent.body, &sent.used);
        let dir = file.parent().ok_or("No data folder.")?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let part = dir.join(format!(".{}.part", uuid::Uuid::new_v4().simple()));
        std::fs::write(&part, &body)
            .and_then(|_| std::fs::rename(&part, &file))
            .map_err(|e| {
                let _ = std::fs::remove_file(&part);
                e.to_string()
            })?;
        Ok(SavedFile {
            status: sent.status,
            headers: sent.headers,
            path: self.located(&file),
            size: body.len() as u64,
        })
    }

    fn open_file(&mut self, path: String) -> Result<(), String> {
        let path = match (path.strip_prefix("~/"), std::env::home_dir()) {
            (Some(rest), Some(home)) => home.join(rest),
            _ => PathBuf::from(path),
        };
        let file = inside_data(&self.data, &path)?;
        crate::open_attachment(file)
    }

    fn accounts(&mut self) -> Vec<Account> {
        (self.hooks.accounts)(&self.id)
            .into_iter()
            .map(|a| Account {
                id: a.id,
                fields: a.values.into_iter().collect(),
            })
            .collect()
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
        self.located(&self.data)
    }

    fn options(&mut self) -> String {
        (self.hooks.options)(&self.id).to_string()
    }

    fn settings_post(&mut self, message: String) {
        self.post("settings-post", None, message);
    }

    fn open_new_agent(&mut self, agent: String, draft: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.tell(
            "open-new-agent",
            json!({"agent": agent, "draft": id, "fields": draft}),
        );
        id
    }

    fn update_new_agent(&mut self, id: String, draft: String) {
        self.tell("update-new-agent", json!({"draft": id, "fields": draft}));
    }

    fn projects(&mut self) -> Result<Vec<Project>, String> {
        self.may(READ_PROJECTS)?;
        Ok(self
            .listed::<activity::Project>("projects")?
            .into_iter()
            .filter_map(activity::Project::clean)
            .map(|v| v.masked(&self.id).into())
            .collect())
    }

    fn groups(&mut self) -> Result<Vec<Group>, String> {
        self.may(READ_PROJECTS)?;
        Ok(self
            .listed::<activity::Group>("groups")?
            .into_iter()
            .filter_map(activity::Group::clean)
            .map(|v| v.masked(&self.id).into())
            .collect())
    }

    fn all_agents(&mut self) -> Result<Vec<AgentActivity>, String> {
        self.may(ACTIVITY)?;
        Ok(self
            .listed::<activity::Agent>("all-agents")?
            .into_iter()
            .filter_map(activity::Agent::clean)
            .map(|v| v.masked(&self.id).into())
            .collect())
    }

    fn unread_agents(&mut self) -> Result<Vec<String>, String> {
        self.may(ACTIVITY)?;
        Ok(self
            .listed::<activity::Agent>("all-agents")?
            .into_iter()
            .filter_map(activity::Agent::clean)
            .filter(|a| a.unread)
            .map(|a| a.id)
            .collect())
    }

    fn usage(&mut self) -> Result<ClaudeUsage, String> {
        self.may(READ_USAGE)?;
        serde_json::from_value::<activity::Usage>(self.recently("usage")?)
            .ok()
            .and_then(activity::Usage::clean)
            .map(Into::into)
            .ok_or_else(|| "Squadron does not know Claude's usage yet.".to_string())
    }

    fn show_agent(&mut self, agent: String) -> Result<(), String> {
        self.may(SHOW_AGENTS)?;
        if !activity::id(&agent) {
            return Err(format!("{agent} is not an agent id."));
        }
        {
            let mut kept = self.kept.lock().unwrap();
            if kept.shown.is_some_and(|at| at.elapsed() < SHOW_EVERY) {
                return Err("show-agent waits a second between two calls.".into());
            }
            kept.shown = Some(Instant::now());
        }
        self.ask("show-agent", json!({"agent": agent})).map(|_| ())
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

pub fn account_key(id: &str, account: &str, field: &str) -> Result<String, String> {
    secret_key(id, field)?;
    Ok(format!("{}.{field}", secret_key(id, account)?))
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

fn data_file(data: &Path, name: &str) -> Result<PathBuf, String> {
    use std::path::Component;
    let path = Path::new(name);
    let plain = !name.is_empty()
        && !name.contains(['\\', '\0'])
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(p) if !p.to_string_lossy().starts_with('.')));
    if !plain {
        return Err(format!("{name} is not a file name inside the data folder."));
    }
    Ok(data.join(path))
}

fn inside_data(data: &Path, path: &Path) -> Result<PathBuf, String> {
    let outside = || "Only the files of the extension's data folder open from here.".to_string();
    if !path.starts_with(data)
        || path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(outside());
    }
    let root = data
        .canonicalize()
        .map_err(|_| "The data folder is empty.")?;
    let file = path.canonicalize().map_err(|_| "The file is gone.")?;
    if !file.starts_with(&root) {
        return Err(outside());
    }
    Ok(file)
}

fn lookup(
    name: &str,
    host: &str,
    accounts: &[Stored],
    secret: &impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<(String, bool), String> {
    let Some((account, field)) = name.split_once('.') else {
        secret_key("x", name)?;
        return secret(name)?.map(|v| (v, true)).ok_or_else(|| {
            format!("The secret {name} is not set. Set it in Settings › Extensions.")
        });
    };
    let found = accounts
        .iter()
        .find(|a| a.id == account)
        .ok_or_else(|| format!("The account {account} no longer exists."))?;
    if let Some(bound) = found.host.as_deref().filter(|h| *h != host) {
        return Err(format!(
            "The fields of the account {account} only go to {bound}."
        ));
    }
    if let Some(value) = found.values.get(field) {
        return Ok((value.clone(), false));
    }
    if !found.secrets.iter().any(|s| s == field) {
        return Err(format!("The account {account} has no field {field}."));
    }
    secret_key("x", field)?;
    secret(&format!("{account}.{field}"))?
        .map(|v| (v, true))
        .ok_or_else(|| format!("The field {field} of the account {account} is not set."))
}

fn fill(
    value: &str,
    resolve: &impl Fn(&str) -> Result<(String, bool), String>,
    used: &mut Vec<String>,
) -> Result<String, String> {
    let (mut out, mut rest) = (String::new(), value);
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else {
            break;
        };
        let name = &rest[start + 2..start + 2 + len];
        let (found, secret) = match name.strip_prefix("base64:") {
            Some(parts) => {
                let mut joined = Vec::new();
                let mut secret = false;
                for part in parts.split(':') {
                    let (value, hidden) = resolve(part)?;
                    if hidden {
                        used.push(value.clone());
                    }
                    joined.push(value);
                    secret |= hidden;
                }
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    joined.join(":"),
                );
                (encoded, secret)
            }
            None => resolve(name)?,
        };
        out.push_str(&rest[..start]);
        out.push_str(&found);
        if secret {
            used.push(found);
        }
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

fn scrub_bytes(mut body: Vec<u8>, used: &[String]) -> Vec<u8> {
    for needle in used.iter().map(|u| u.as_bytes()).filter(|u| !u.is_empty()) {
        if !body.windows(needle.len()).any(|w| w == needle) {
            continue;
        }
        let (mut out, mut i) = (Vec::with_capacity(body.len()), 0);
        while i < body.len() {
            if body[i..].starts_with(needle) {
                out.extend_from_slice(b"[secret]");
                i += needle.len();
            } else {
                out.push(body[i]);
                i += 1;
            }
        }
        body = out;
    }
    body
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

struct Sent {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    used: Vec<String>,
}

fn send(
    allowed: &[String],
    accounts: &[Stored],
    request: Request,
    max: usize,
    timeout: Duration,
    secret: impl Fn(&str) -> Result<Option<String>, String>,
) -> Result<Sent, String> {
    let hosts: Vec<String> = allowed
        .iter()
        .cloned()
        .chain(
            accounts
                .iter()
                .filter_map(|a| a.host.as_ref().map(|h| format!("net:{h}"))),
        )
        .collect();
    let url = checked(&request.url, &hosts)?;
    let host = url.host_str().unwrap_or_default().to_string();
    let method = reqwest::Method::from_bytes(request.method.to_uppercase().as_bytes())
        .map_err(|_| format!("{} is not an HTTP method.", request.method))?;
    let resolve = |name: &str| lookup(name, &host, accounts, &secret);
    let mut used = Vec::new();
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &request.headers {
        let value = fill(value, &resolve, &mut used)?;
        let bad = || format!("The header {name} is not valid.");
        headers.append(
            reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| bad())?,
            reqwest::header::HeaderValue::from_str(&value).map_err(|_| bad())?,
        );
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(timeout)
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
            if body.len() + chunk.len() > max {
                return Err(format!("The response is larger than {} MB.", max >> 20));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Sent {
            status: answer.status().as_u16(),
            headers,
            body,
            used,
        })
    })
}

struct Instance {
    store: Store<Host>,
    bindings: SquadronCore,
    check: Option<CheckAccount>,
    forge: Option<Forge>,
}

fn instantiate(engine: &Engine, component: &Component, host: Host) -> wasmtime::Result<Instance> {
    let mut linker = Linker::<Host>::new(engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    SquadronCore::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
    let mut store = Store::new(engine, host);
    store.limiter(|h| &mut h.limits);
    store.set_fuel(FUEL)?;
    let instance = linker.instantiate(&mut store, component)?;
    let bindings = SquadronCore::new(&mut store, &instance)?;
    let check = instance
        .get_typed_func::<(Account,), (Result<String, String>,)>(&mut store, "check-account")
        .ok();
    let forge = instance
        .get_typed_func::<(String, String), (Result<String, String>,)>(&mut store, "forge")
        .ok();
    Ok(Instance {
        store,
        bindings,
        check,
        forge,
    })
}

pub const NO_CHECK: &str = "This extension does not export check-account. Build it against squadron_extension_api 1.6.0 or later.";

pub const NO_FORGE: &str =
    "This extension does not export forge. Build it against squadron_extension_api 1.8.0 or later.";

const NO_ANSWER: &str = "The extension did not answer.";

fn unsupported(call: &str) -> String {
    json!({"code": "unsupported", "title": format!("This extension does not answer {call}.")})
        .to_string()
}

fn refused(error: &str) -> bool {
    serde_json::from_str::<Value>(error).is_ok_and(|e| e["code"] == "unsupported")
}

#[derive(Clone)]
struct Maker {
    engine: Engine,
    component: Component,
    id: String,
    emit: Emit,
    answers: Answers,
    file: PathBuf,
    data: PathBuf,
    hooks: Hooks,
    kept: Arc<Mutex<Kept>>,
    state: State,
}

impl Maker {
    fn host(&self, allowed: &[String], panels: &[String]) -> Host {
        Host {
            id: self.id.clone(),
            wasi: WasiCtx::builder().build(),
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new().memory_size(MEMORY).build(),
            state: self.state.clone(),
            file: self.file.clone(),
            data: self.data.clone(),
            allowed: allowed.to_vec(),
            emit: self.emit.clone(),
            answers: self.answers.clone(),
            timer: None,
            hooks: self.hooks.clone(),
            trial: None,
            panels: panels.to_vec(),
            kept: self.kept.clone(),
        }
    }

    fn instantiate(&self, allowed: &[String], panels: &[String]) -> Result<Instance, String> {
        instantiate(&self.engine, &self.component, self.host(allowed, panels))
            .map_err(|e| format!("{e:#}"))
    }

    fn report(&self, error: &str) {
        (self.emit)(json!({"extension": self.id, "call": "error", "error": error}));
    }
}

#[derive(Default)]
struct Slots {
    idle: Vec<Instance>,
    made: usize,
}

struct Pool {
    maker: Maker,
    rights: Mutex<(Vec<String>, Vec<String>)>,
    slots: Mutex<Slots>,
    freed: Condvar,
    flying: Mutex<HashMap<String, Vec<mpsc::Sender<Answer>>>>,
    unsupported: Mutex<Vec<String>>,
}

impl Pool {
    fn new(maker: Maker, allowed: Vec<String>, panels: Vec<String>) -> Self {
        Self {
            maker,
            rights: Mutex::new((allowed, panels)),
            slots: Mutex::default(),
            freed: Condvar::new(),
            flying: Mutex::default(),
            unsupported: Mutex::default(),
        }
    }

    fn allow(&self, allowed: Vec<String>, panels: Vec<String>) {
        *self.rights.lock().unwrap() = (allowed, panels);
    }

    fn call(self: &Arc<Self>, call: &str, input: &str) -> Answer {
        if self.unsupported.lock().unwrap().iter().any(|c| c == call) {
            return Ok(Err(unsupported(call)));
        }
        let key = format!("{call}\n{input}");
        let (tx, rx) = mpsc::channel();
        let first = {
            let mut flying = self.flying.lock().unwrap();
            let waiting = flying.entry(key.clone()).or_default();
            waiting.push(tx);
            waiting.len() == 1
        };
        if first {
            let (pool, call, input) = (self.clone(), call.to_string(), input.to_string());
            std::thread::spawn(move || {
                let answer = pool.serve(call, input);
                let waiting = pool.flying.lock().unwrap().remove(&key);
                for tx in waiting.into_iter().flatten() {
                    let _ = tx.send(answer.clone());
                }
            });
        }
        rx.recv_timeout(ANSWER).map_err(|_| NO_ANSWER.to_string())?
    }

    fn serve(&self, call: String, input: String) -> Answer {
        let mut instance = self.take()?;
        let Some(forge) = instance.forge else {
            self.give(Some(instance));
            return Err(NO_FORGE.into());
        };
        let (allowed, panels) = self.rights.lock().unwrap().clone();
        let store = &mut instance.store;
        store.data_mut().allowed = allowed;
        store.data_mut().panels = panels;
        let _ = store.set_fuel(FUEL);
        let call_name = call.clone();
        match forge.call(&mut *store, (call, input)) {
            Ok((result,)) => {
                self.give(Some(instance));
                if result.as_ref().is_err_and(|e| refused(e)) {
                    self.unsupported.lock().unwrap().push(call_name);
                }
                Ok(result)
            }
            Err(e) => {
                let e = trapped(e);
                self.maker.report(&e);
                self.give(None);
                Err(e)
            }
        }
    }

    fn take(&self) -> Result<Instance, String> {
        let deadline = Instant::now() + ANSWER;
        let mut slots = self.slots.lock().unwrap();
        loop {
            if let Some(instance) = slots.idle.pop() {
                return Ok(instance);
            }
            if slots.made < POOL {
                slots.made += 1;
                drop(slots);
                let (allowed, panels) = self.rights.lock().unwrap().clone();
                return self
                    .maker
                    .instantiate(&allowed, &panels)
                    .inspect_err(|_| self.give(None));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(NO_ANSWER.into());
            }
            slots = self.freed.wait_timeout(slots, left).unwrap().0;
        }
    }

    fn give(&self, instance: Option<Instance>) {
        let mut slots = self.slots.lock().unwrap();
        match instance {
            Some(instance) => slots.idle.push(instance),
            None => slots.made -= 1,
        }
        self.freed.notify_one();
    }
}

fn text_of(value: &Value) -> Option<Value> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(json!(s.trim())),
        Value::Object(o) => {
            let texts: Map<String, Value> = o
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), json!(v.as_str()?.trim()))))
                .filter(|(_, v)| v.as_str().is_some_and(|s| !s.is_empty()))
                .collect();
            texts.contains_key("en").then_some(Value::Object(texts))
        }
        _ => None,
    }
}

pub fn verdict(answer: Result<Result<String, String>, String>, fields: &[String]) -> Value {
    let (fine, raw) = match answer {
        Err(problem) => {
            return json!({"ok": false, "title": null, "detail": problem, "field": null});
        }
        Ok(Ok(raw)) => (true, raw),
        Ok(Err(raw)) => (false, raw),
    };
    let parsed = serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw));
    let Value::Object(o) = parsed else {
        return json!({"ok": fine, "title": text_of(&parsed), "detail": null, "field": null});
    };
    let ok = o.get("ok").and_then(Value::as_bool).unwrap_or(fine);
    let main = if ok {
        ["text", "title"]
    } else {
        ["title", "text"]
    };
    let title = main.iter().find_map(|k| o.get(*k).and_then(text_of));
    let detail = o.get("detail").and_then(text_of);
    let field = o
        .get("field")
        .and_then(Value::as_str)
        .filter(|f| !ok && fields.iter().any(|known| known == f));
    json!({"ok": ok, "title": title, "detail": detail, "field": field})
}

pub struct Runtime {
    engine: Engine,
    loaded: Running,
    pools: Mutex<HashMap<String, Arc<Pool>>>,
    forges: Mutex<Vec<(String, ForgeSpec)>>,
    pages: Mutex<HashMap<String, (Instant, u32)>>,
    feed: mpsc::Sender<Admitted>,
    states: PathBuf,
    emit: Emit,
    answers: Answers,
    hooks: Hooks,
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
        activity::keep_salt(&states.with_file_name("extensions-salt"));
        let loaded = Running::default();
        let (feed, fed) = mpsc::channel();
        let running = loaded.clone();
        std::thread::spawn(move || pump(&fed, &running, FLUSH));
        Ok(Self {
            engine: Engine::new(&config).map_err(|e| e.to_string())?,
            loaded,
            pools: Mutex::default(),
            forges: Mutex::default(),
            pages: Default::default(),
            feed,
            states,
            emit: Arc::new(emit),
            answers: Default::default(),
            hooks: Hooks::default(),
        })
    }

    pub fn with_hooks(mut self, hooks: Hooks) -> Self {
        self.hooks = hooks;
        self
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
        *self.forges.lock().unwrap() = specs
            .iter()
            .filter_map(|s| Some((s.id.clone(), s.forge.clone()?)))
            .collect();
        let mut loaded = self.loaded.lock().unwrap();
        let mut pools = self.pools.lock().unwrap();
        loaded.retain(|id, _| specs.iter().any(|s| &s.id == id));
        pools.retain(|id, _| loaded.contains_key(id));
        for spec in specs {
            let path = match resolve(&spec.dir, &spec.wasm) {
                Ok(path) => path,
                Err(e) => {
                    loaded.remove(&spec.id);
                    pools.remove(&spec.id);
                    errors.insert(spec.id, e);
                    continue;
                }
            };
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if let Some(l) = loaded
                .get(&spec.id)
                .filter(|l| l.path == path && l.modified == modified)
            {
                let _ =
                    l.tx.send(Job::Allow(spec.allowed.clone(), spec.panels.clone()));
                if let Some(pool) = pools.get(&spec.id) {
                    pool.allow(spec.allowed.clone(), spec.panels.clone());
                }
                if let Some(l) = loaded.get_mut(&spec.id) {
                    l.allowed = spec.allowed;
                }
                continue;
            }
            match self.start(&spec, &path) {
                Ok((tx, inbox, queued, pool)) => {
                    pools.insert(spec.id.clone(), pool);
                    let allowed = spec.allowed;
                    loaded.insert(
                        spec.id,
                        Loaded {
                            tx,
                            path,
                            modified,
                            allowed,
                            inbox,
                            queued,
                        },
                    );
                }
                Err(e) => {
                    loaded.remove(&spec.id);
                    pools.remove(&spec.id);
                    errors.insert(spec.id, e);
                }
            }
        }
        errors
    }

    fn start(
        &self,
        spec: &Spec,
        path: &Path,
    ) -> Result<(mpsc::Sender<Job>, Inbox, Queued, Arc<Pool>), String> {
        let component = Component::from_file(&self.engine, path).map_err(|e| format!("{e:#}"))?;
        let file = self.states.join(format!("{}.json", spec.id));
        let state = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let maker = Maker {
            engine: self.engine.clone(),
            component,
            id: spec.id.clone(),
            emit: self.emit.clone(),
            answers: self.answers.clone(),
            file,
            data: self.data_of(&spec.id),
            hooks: self.hooks.clone(),
            kept: Arc::default(),
            state: Arc::new(Mutex::new(state)),
        };
        let (mut allowed, mut panels) = (spec.allowed.clone(), spec.panels.clone());
        let pool = Arc::new(Pool::new(maker.clone(), allowed.clone(), panels.clone()));
        let provides = spec.forge.is_some();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (tx, rx) = mpsc::channel::<Job>();
        let id = spec.id.clone();
        let inbox = Inbox::default();
        let fed = inbox.clone();
        let queued = Queued::default();
        let waiting = queued.clone();
        std::thread::spawn(move || {
            let mut current = match maker.instantiate(&allowed, &panels) {
                Ok(current) if provides && current.forge.is_none() => {
                    return ready_tx.send(Err(NO_FORGE.into())).unwrap_or(());
                }
                Ok(current) => current,
                Err(e) => return ready_tx.send(Err(e)).unwrap_or(()),
            };
            let _ = ready_tx.send(Ok(()));
            let mut job = Some(Job::Event("started".into(), None, "{}".into()));
            loop {
                let timer = current.store.data().timer;
                let Some(next) = job.take().or_else(|| wait(&rx, timer)) else {
                    break;
                };
                let Instance {
                    store,
                    bindings,
                    check,
                    ..
                } = &mut current;
                let _ = store.set_fuel(FUEL);
                let failure = match next {
                    Job::Allow(list, pages) => {
                        allowed = list.clone();
                        panels = pages.clone();
                        store.data_mut().allowed = list;
                        store.data_mut().panels = pages;
                        None
                    }
                    Job::Timer => {
                        store.data_mut().timer = None;
                        bindings
                            .call_on_event(&mut *store, "timer", None, "{}")
                            .err()
                            .map(trapped)
                    }
                    Job::Event(name, agent, data) => {
                        if page(&name) {
                            waiting.0.fetch_sub(1, Ordering::SeqCst);
                            waiting.1.fetch_sub(data.len(), Ordering::SeqCst);
                        }
                        bindings
                            .call_on_event(&mut *store, &name, agent.as_ref(), &data)
                            .err()
                            .map(trapped)
                    }
                    Job::Fed => {
                        let events = std::mem::take(&mut *fed.lock().unwrap());
                        events
                            .into_iter()
                            .filter(|e| allowed.iter().any(|a| a == e.access))
                            .find_map(|e| {
                                bindings
                                    .call_on_event(
                                        &mut *store,
                                        &e.name,
                                        None,
                                        &activity::masked(&id, &e.name, &e.data).to_string(),
                                    )
                                    .err()
                                    .map(trapped)
                            })
                    }
                    Job::Check(trial, reply) => match check {
                        None => {
                            let _ = reply.send(Err(NO_CHECK.into()));
                            None
                        }
                        Some(check) => {
                            let account = Account {
                                id: trial.account.id.clone(),
                                fields: trial.account.values.clone().into_iter().collect(),
                            };
                            store.data_mut().trial = Some(trial);
                            let out = check.call(&mut *store, (account,));
                            store.data_mut().trial = None;
                            match out {
                                Ok((result,)) => {
                                    let _ = reply.send(Ok(result));
                                    None
                                }
                                Err(e) => {
                                    let e = trapped(e);
                                    let _ = reply.send(Err(e.clone()));
                                    Some(e)
                                }
                            }
                        }
                    },
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
                    maker.report(&error);
                    let timer = current.store.data().timer;
                    match maker.instantiate(&allowed, &panels) {
                        Ok(mut fresh) => {
                            fresh.store.data_mut().timer = timer;
                            current = fresh;
                        }
                        Err(e) => {
                            maker.report(&e);
                            break;
                        }
                    }
                }
            }
        });
        ready_rx
            .recv()
            .map_err(|_| "The extension did not start.".to_string())??;
        Ok((tx, inbox, queued, pool))
    }

    fn page_event(&self, id: &str, data: &str) -> bool {
        if data.len() > MAX_PAGE_EVENT {
            return false;
        }
        let now = Instant::now();
        let mut pages = self.pages.lock().unwrap();
        let (since, count) = pages.entry(id.into()).or_insert((now, 0));
        if now.duration_since(*since) >= Duration::from_secs(1) {
            (*since, *count) = (now, 0);
        }
        *count += 1;
        *count <= PAGE_EVENTS
    }

    pub fn event(&self, only: Option<&str>, name: &str, agent: Option<AgentInfo>, data: &str) {
        if page(name) && !only.is_some_and(|id| self.page_event(id, data)) {
            return;
        }
        for (id, l) in self
            .loaded
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| only.is_none_or(|only| only == id.as_str()))
        {
            if page(name) {
                let (count, bytes) = (&l.queued.0, &l.queued.1);
                if count.load(Ordering::SeqCst) >= QUEUED_PAGE_EVENTS
                    || bytes.load(Ordering::SeqCst) + data.len() > QUEUED_PAGE_BYTES
                {
                    continue;
                }
                count.fetch_add(1, Ordering::SeqCst);
                bytes.fetch_add(data.len(), Ordering::SeqCst);
            }
            let (agent, data) = withheld(id, name, agent.clone(), data, &l.allowed);
            let _ =
                l.tx.send(Job::Event(name.into(), agent.map(Agent::from), data));
        }
    }

    pub fn feed(&self, list: Vec<Fed>) {
        for admitted in list.into_iter().filter_map(activity::admit) {
            let _ = self.feed.send(admitted);
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

    fn veiled(&self, id: &str, agent: Option<AgentInfo>) -> Option<AgentInfo> {
        let loaded = self.loaded.lock().unwrap();
        match loaded.get(id) {
            Some(l) if !sees_files(&l.allowed) => veil(id, agent),
            _ => agent,
        }
    }

    pub fn run(&self, id: &str, action: &str, agent: Option<AgentInfo>) -> Result<(), String> {
        let (reply, answer) = mpsc::channel();
        let agent = self.veiled(id, agent);
        self.sender(id)?
            .send(Job::Run(action.into(), agent.map(Agent::from), reply))
            .map_err(|_| "The extension stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The extension did not answer.".to_string())?
    }

    pub fn check(&self, id: &str, trial: Trial) -> Result<Result<String, String>, String> {
        let (reply, answer) = mpsc::channel();
        self.sender(id)?
            .send(Job::Check(trial, reply))
            .map_err(|_| "The extension stopped.".to_string())?;
        answer
            .recv_timeout(ANSWER)
            .map_err(|_| "The extension did not answer.".to_string())?
    }

    pub fn providers(&self) -> Vec<Provider> {
        let running = self.pools.lock().unwrap();
        self.forges
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, _)| running.contains_key(id))
            .map(|(id, spec)| Provider {
                extension: id.clone(),
                spec: spec.clone(),
                hosts: (self.hooks.accounts)(id)
                    .into_iter()
                    .filter_map(|a| a.host)
                    .collect(),
            })
            .collect()
    }

    pub fn forge(&self, id: &str, call: &str, input: &str) -> Answer {
        let pool = self
            .pools
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| format!("The extension {id} is not running."))?;
        pool.call(call, input)
    }

    pub fn tool(
        &self,
        id: &str,
        name: &str,
        input: &str,
        agent: Option<AgentInfo>,
    ) -> Result<String, String> {
        let (reply, answer) = mpsc::channel();
        let agent = self.veiled(id, agent);
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

fn sees_files(allowed: &[String]) -> bool {
    allowed
        .iter()
        .any(|a| a == READ_AGENTS || a.starts_with("exec:"))
}

fn veil(extension: &str, agent: Option<AgentInfo>) -> Option<AgentInfo> {
    agent.map(|a| AgentInfo {
        project: activity::opaque(extension, &a.project),
        dir: String::new(),
        ..a
    })
}

fn withheld(
    extension: &str,
    name: &str,
    agent: Option<AgentInfo>,
    data: &str,
    allowed: &[String],
) -> (Option<AgentInfo>, String) {
    if agent.is_none() || sees_files(allowed) {
        return (agent, data.into());
    }
    let agent = veil(extension, agent);
    if !matches!(name, "agent.done" | "agent.waiting") {
        return (agent, data.into());
    }
    let kind = serde_json::from_str::<Value>(data)
        .ok()
        .and_then(|d| d.get("kind")?.as_str().map(String::from))
        .filter(|k| name == "agent.waiting" && matches!(k.as_str(), "permission" | "question"));
    let data = kind.map_or_else(|| "{}".into(), |kind| json!({ "kind": kind }).to_string());
    (agent, data)
}

fn deliver(running: &Running, pending: Vec<Admitted>) {
    for l in running.lock().unwrap().values() {
        let mut inbox = l.inbox.lock().unwrap();
        let idle = inbox.is_empty();
        for event in pending
            .iter()
            .filter(|e| l.allowed.iter().any(|a| a == e.access))
        {
            activity::merge(&mut inbox, event.clone());
        }
        if idle && !inbox.is_empty() {
            let _ = l.tx.send(Job::Fed);
        }
    }
}

fn pump(fed: &mpsc::Receiver<Admitted>, running: &Running, every: Duration) {
    let mut last: Option<Instant> = None;
    while let Ok(first) = fed.recv() {
        let mut pending = vec![first];
        let due = last.map_or_else(Instant::now, |at| at + every);
        loop {
            match fed.recv_timeout(due.saturating_duration_since(Instant::now())) {
                Ok(next) => activity::merge(&mut pending, next),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        deliver(running, pending);
        last = Some(Instant::now());
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
            panels: vec!["main".into()],
            forge: None,
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
        let ada = agent.clone();
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

        let unreachable = runtime.check(
            "hello",
            Trial {
                account: Stored {
                    id: "trial-1".into(),
                    values: [("api".to_string(), "nowhere.invalid".to_string())].into(),
                    host: Some("nowhere.invalid".into()),
                    secrets: vec!["token".into()],
                },
                secrets: [("token".to_string(), "t0k".to_string())].into(),
            },
        );
        let answer = verdict(unreachable, &["api".into(), "token".into()]);
        assert_eq!(answer["ok"], false);
        assert_eq!(answer["field"], "api");
        assert_eq!(answer["title"]["en"], "Could not reach nowhere.invalid");
        assert!(!answer["detail"].as_str().unwrap().contains("t0k"));

        let restarted = Runtime::new(states, |_| {}).unwrap();
        assert!(restarted.sync(vec![spec(vec![])]).is_empty());
        assert_eq!(
            restarted.tool("hello", "clicks", "{}", None).unwrap(),
            "The Count button was clicked 4 times."
        );

        let told = Arc::new(Mutex::new(Vec::<Value>::new()));
        let record = told.clone();
        let set = Runtime::new(
            std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4())),
            move |v| record.lock().unwrap().push(v),
        )
        .unwrap()
        .with_hooks(Hooks {
            options: Arc::new(|_| json!({"greeting": "Bonjour", "tone": "plain"})),
            ..Hooks::default()
        });
        assert!(set.sync(vec![spec(vec![])]).is_empty());
        assert_eq!(
            set.tool("hello", "options", "{}", None).unwrap(),
            r#"{"greeting":"Bonjour","tone":"plain"}"#
        );
        set.run("hello", "greet", Some(ada)).unwrap();
        set.event(
            Some("hello"),
            "settings.message",
            None,
            r#"{"message":{"type":"ready"}}"#,
        );
        set.run("hello", "nope", None).unwrap_err();
        let told = told.lock().unwrap().clone();
        assert!(
            told.iter()
                .any(|v| v["text"] == "Bonjour Ada, what are you working on?")
        );
        assert!(
            told.iter()
                .any(|v| v["call"] == "settings-post" && v["message"] == r#"{"count":0}"#)
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
            state: State::default(),
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
            hooks: Hooks::default(),
            trial: None,
            panels: vec!["main".into(), "other".into()],
            kept: Default::default(),
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
    fn reads_projects_agents_and_usage_only_with_their_permission() {
        use squadron::api::host::Host as _;
        let window = |call: &str, _: &Value| match call {
            "projects" => Ok(json!([
                {"id": "/home/me/ring", "name": "Ring", "color": "#60A5FA", "group": "g1", "archived": false, "active": 1, "week_agents": 1, "agents": 2, "path": "/home/me/ring"},
                {"broken": true}
            ])),
            "groups" => Ok(json!([{"id": "g1", "name": "Work", "parent": ""}])),
            "all-agents" => Ok(json!([
                {"id": "a1", "project": "/home/me/ring", "name": "RING-1", "status": "waiting", "waiting": "question", "tool": null, "error": null, "by": null, "archived": true, "since": 3, "dir": "/home/me/ring/w"},
                {"id": "a2", "project": "/home/me/ring", "name": "RING-2", "status": "done", "archived": false, "since": 4, "unread": true},
                {"id": "a3", "project": "/home/me/ring", "name": "RING-3", "status": "done", "archived": true, "since": 4, "unread": true}
            ])),
            "usage" => Ok(
                json!({"five_hour": {"utilization": 0.5, "resets_at": 10}, "seven_day": null, "api": "overloaded", "retries": 2, "since": 4}),
            ),
            "show-agent" => Ok(Value::Null),
            _ => Err("unknown".into()),
        };

        let (mut denied, told) = host(&[READ_AGENTS, ARCHIVE_AGENTS], window);
        assert!(
            denied
                .projects()
                .unwrap_err()
                .contains(r#""read" in permissions.projects"#)
        );
        assert!(
            denied
                .groups()
                .unwrap_err()
                .contains(r#""read" in permissions.projects"#)
        );
        assert!(
            denied
                .all_agents()
                .unwrap_err()
                .contains(r#""activity" in permissions.agents"#)
        );
        assert!(
            denied
                .unread_agents()
                .unwrap_err()
                .contains(r#""activity" in permissions.agents"#)
        );
        assert!(
            denied
                .usage()
                .unwrap_err()
                .contains(r#""read" in permissions.usage"#)
        );
        assert!(
            denied
                .show_agent("a1".into())
                .unwrap_err()
                .contains(r#""show" in permissions.agents"#)
        );
        assert!(told.lock().unwrap().is_empty());

        let (mut host, told) = host(&[READ_PROJECTS, ACTIVITY, READ_USAGE, SHOW_AGENTS], window);
        let projects = host.projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, activity::opaque("jira", "/home/me/ring"));
        assert_eq!(projects[0].group, Some(activity::opaque("jira", "g1")));
        assert_eq!(host.groups().unwrap()[0].parent, None);
        let agents = host.all_agents().unwrap();
        assert_eq!(agents[0].project, projects[0].id);
        assert_eq!(agents[0].waiting.as_deref(), Some("question"));
        assert!(agents[0].archived);
        assert_eq!(agents.len(), 3);
        assert_eq!(host.unread_agents().unwrap(), ["a2"]);
        let usage = host.usage().unwrap();
        assert_eq!((usage.api.as_str(), usage.retries), ("overloaded", 2));
        assert_eq!(usage.five_hour.unwrap().resets_at, Some(10));
        assert!(
            host.show_agent("../a1".into())
                .unwrap_err()
                .contains("not an agent id")
        );
        host.show_agent("a1".into()).unwrap();
        assert!(
            host.show_agent("a1".into())
                .unwrap_err()
                .contains("a second")
        );
        assert_eq!(host.projects().unwrap().len(), 1);
        let calls: Vec<_> = told
            .lock()
            .unwrap()
            .iter()
            .map(|v| v["call"].clone())
            .collect();
        assert_eq!(
            calls,
            ["projects", "groups", "all-agents", "usage", "show-agent"]
        );
        assert!(!format!("{:?}", told.lock().unwrap()).contains("dir"));
    }

    #[test]
    fn feeds_each_extension_what_it_may_see_at_most_once_a_second() {
        let running = Running::default();
        let mut boxes = Vec::new();
        for (id, allowed) in [("office", ACTIVITY), ("hello", READ_AGENTS)] {
            let (tx, rx) = mpsc::channel();
            let inbox = Inbox::default();
            boxes.push((rx, inbox.clone()));
            running.lock().unwrap().insert(
                id.into(),
                Loaded {
                    tx,
                    path: PathBuf::new(),
                    modified: None,
                    allowed: vec![allowed.into()],
                    inbox,
                    queued: Queued::default(),
                },
            );
        }
        let status = |s: &str| {
            activity::admit(Fed {
                name: "agent.status".into(),
                data: json!({"id": "a1", "project": "/p", "name": "A", "status": s, "archived": false, "since": 0}),
            })
            .unwrap()
        };
        let (feed, fed) = mpsc::channel();
        for s in ["working", "waiting", "done"] {
            feed.send(status(s)).unwrap();
        }
        let pumping = running.clone();
        std::thread::spawn(move || pump(&fed, &pumping, Duration::from_millis(400)));
        let (office, hello) = (&boxes[0], &boxes[1]);
        let received = |within: Duration| match office.0.recv_timeout(within) {
            Ok(Job::Fed) => std::mem::take(&mut *office.1.lock().unwrap())
                .into_iter()
                .map(|e| e.data["status"].clone())
                .collect(),
            _ => Vec::new(),
        };
        assert_eq!(received(Duration::from_secs(2)), [json!("done")]);
        assert!(received(Duration::from_millis(200)).is_empty());
        let sent = Instant::now();
        feed.send(status("working")).unwrap();
        assert_eq!(received(Duration::from_secs(2)), [json!("working")]);
        assert!(sent.elapsed() >= Duration::from_millis(150));
        assert!(hello.0.try_recv().is_err());
        assert!(hello.1.lock().unwrap().is_empty());

        let mut slow = Vec::new();
        for s in ["working", "done"] {
            deliver(&running, vec![status(s)]);
            slow.push(office.0.try_recv().is_ok());
        }
        assert_eq!(slow, [true, false]);
        assert_eq!(received(Duration::ZERO), Vec::<Value>::new());
        assert_eq!(office.1.lock().unwrap()[0].data["status"], "done");
    }

    #[test]
    fn hides_folders_and_waiting_text_from_extensions_that_do_not_read_agents() {
        let runtime = Runtime::new(
            std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4())),
            |_| {},
        )
        .unwrap();
        let mut jobs = Vec::new();
        for (id, allowed) in [("blind", vec![]), ("reader", vec![READ_AGENTS.to_string()])] {
            let (tx, rx) = mpsc::channel();
            jobs.push(rx);
            runtime.loaded.lock().unwrap().insert(
                id.into(),
                Loaded {
                    tx,
                    path: PathBuf::new(),
                    modified: None,
                    allowed,
                    inbox: Inbox::default(),
                    queued: Queued::default(),
                },
            );
        }
        let agent = AgentInfo {
            id: "a1".into(),
            name: "RING-1".into(),
            project: "/home/me/ring".into(),
            dir: "/home/me/ring/w".into(),
        };
        assert_eq!(
            runtime.veiled("blind", Some(agent.clone())).unwrap().dir,
            ""
        );
        assert_eq!(
            runtime.veiled("reader", Some(agent.clone())).unwrap().dir,
            "/home/me/ring/w"
        );
        let waiting = r#"{"waiting":"Allow Bash: cat ~/.ssh/id_ed25519","kind":"permission"}"#;
        runtime.event(None, "agent.waiting", Some(agent.clone()), waiting);
        let got = |rx: &mpsc::Receiver<Job>| match rx.try_recv() {
            Ok(Job::Event(_, Some(a), data)) => (a.project, a.dir, data),
            _ => panic!("no event"),
        };
        let (project, dir, data) = got(&jobs[0]);
        assert_eq!(
            (project, dir),
            (activity::opaque("blind", "/home/me/ring"), String::new())
        );
        assert_eq!(data, r#"{"kind":"permission"}"#);
        assert_eq!(got(&jobs[1]).2, waiting);
        runtime.event(None, "agent.done", Some(agent.clone()), "{}");
        assert_eq!(got(&jobs[0]).1, "");
        assert_eq!(got(&jobs[1]).1, "/home/me/ring/w");

        for _ in 0..PAGE_EVENTS + 20 {
            runtime.event(Some("blind"), "panel.message", Some(agent.clone()), "{}");
        }
        runtime.event(
            Some("reader"),
            "panel.message",
            None,
            &"x".repeat(MAX_PAGE_EVENT + 1),
        );
        let queued: Vec<_> = jobs[0].try_iter().collect();
        assert_eq!(queued.len(), QUEUED_PAGE_EVENTS);
        assert!(
            queued
                .iter()
                .all(|j| matches!(j, Job::Event(_, Some(a), _) if a.dir.is_empty()))
        );
        assert_eq!(jobs[1].try_iter().count(), 0);
        let budget: Vec<_> = (0..PAGE_EVENTS + 1)
            .map(|_| runtime.page_event("other", "{}"))
            .collect();
        assert!(budget[..PAGE_EVENTS as usize].iter().all(|ok| *ok));
        assert!(!budget[PAGE_EVENTS as usize]);
    }

    #[test]
    fn shows_folders_only_to_extensions_that_can_read_files() {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let data = home.join(".config/squadron/extensions-data/office");
        let (blind, _) = host(&[READ_PROJECTS], |_, _| Ok(Value::Null));
        assert_eq!(
            blind.located(&data),
            "~/.config/squadron/extensions-data/office"
        );
        let (exec, _) = host(&["exec:git"], |_, _| Ok(Value::Null));
        assert_eq!(exec.located(&data), data.to_string_lossy());
        assert_eq!(
            inside_data(&data, Path::new("/no/such/folder")).unwrap_err(),
            inside_data(&data, &home).unwrap_err()
        );
        assert!(inside_data(&data, &data.join("../../../.ssh")).is_err());

        let agent = AgentInfo {
            id: "a1".into(),
            name: "RING-1".into(),
            project: "/home/me/ring".into(),
            dir: "/home/me/ring/w".into(),
        };
        let waiting = r#"{"waiting":"Allow Bash","kind":"permission"}"#;
        let (kept, data) = withheld(
            "x",
            "agent.waiting",
            Some(agent.clone()),
            waiting,
            &["exec:git".into()],
        );
        assert_eq!(
            (kept.unwrap().dir, data.as_str()),
            ("/home/me/ring/w".into(), waiting)
        );
        let (hidden, data) = withheld(
            "x",
            "new-agent.started",
            Some(agent),
            r#"{"draft":"d"}"#,
            &[],
        );
        assert_eq!(
            (hidden.unwrap().dir, data.as_str()),
            (String::new(), r#"{"draft":"d"}"#)
        );
    }

    #[test]
    fn drops_page_messages_past_two_hundred_a_second() {
        use squadron::api::host::Host as _;
        let (mut host, told) = host(&[], |_, _| Ok(Value::Null));
        for i in 0..250 {
            host.panel_post("main".into(), format!("{i}"));
        }
        host.panel_post("other".into(), "{}".into());
        for i in 0..50 {
            host.panel_post(format!("stray{i}{}", "x".repeat(1 << 20)), String::new());
        }
        assert_eq!(host.kept.lock().unwrap().posts.len(), 1 + 1);
        host.settings_post("x".repeat(5 << 20));
        let told = told.lock().unwrap();
        assert_eq!(
            told.iter().filter(|v| v["panel"] == "main").count(),
            POSTS as usize
        );
        assert_eq!(told.iter().filter(|v| v["panel"] == "other").count(), 1);
        assert!(!told.iter().any(|v| v["call"] == "settings-post"));
    }

    #[test]
    fn opens_only_its_own_panels_and_passes_the_window_error_on() {
        use squadron::api::host::Host as _;
        let (mut host, told) = host(&[], |_, v| match v["panel"].as_str() {
            Some("main") => Ok(Value::Null),
            _ => Err("No agent is on screen to open the panel other beside.".into()),
        });
        assert_eq!(
            host.panel_open("stray".into()).unwrap_err(),
            "No panel stray in contributes.panels."
        );
        assert!(told.lock().unwrap().is_empty());
        assert_eq!(host.panel_open("main".into()), Ok(()));
        assert_eq!(
            host.panel_open("other".into()).unwrap_err(),
            "No agent is on screen to open the panel other beside."
        );
        let told = told.lock().unwrap();
        assert_eq!(
            told.iter()
                .map(|v| (v["call"].as_str().unwrap(), v["panel"].as_str().unwrap()))
                .collect::<Vec<_>>(),
            [("panel-open", "main"), ("panel-open", "other")]
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
        tx.send(Job::Allow(vec![], vec![])).unwrap();
        assert!(matches!(
            wait(&rx, Some(Instant::now() + Duration::from_secs(60))),
            Some(Job::Allow(..))
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
            send(
                &allowed,
                &[],
                request("https://evil.com/"),
                MAX_BODY,
                FETCH,
                none
            )
            .err()
            .unwrap()
            .contains("permissions.network")
        );
        let site = account("a1", Some("jira.acme.com"));
        assert!(
            send(
                &allowed,
                &[site],
                request("https://jira.acme.com.evil.com/"),
                MAX_BODY,
                FETCH,
                none
            )
            .err()
            .unwrap()
            .contains("permissions.network")
        );
    }

    fn account(id: &str, host: Option<&str>) -> Stored {
        Stored {
            id: id.into(),
            values: [("email".to_string(), "me@acme.com".to_string())]
                .into_iter()
                .chain(host.map(|h| ("site".to_string(), h.to_string())))
                .collect(),
            host: host.map(str::to_string),
            secrets: vec!["token".into()],
        }
    }

    #[test]
    fn a_trial_account_shadows_the_saved_one_and_keeps_its_secrets_in_memory() {
        let keys = |key: &str| {
            Ok(match key {
                "a1.token" => Some("saved".to_string()),
                _ => None,
            })
        };
        let mut edited = account("a1", Some("jira.other.com"));
        edited.secrets.push("pin".into());
        let trial = Trial {
            account: edited,
            secrets: [("pin".to_string(), "1234".to_string())].into(),
        };
        let accounts = with_trial(
            vec![account("a1", Some("jira.acme.com")), account("b2", None)],
            Some(&trial),
        );
        assert_eq!(accounts.len(), 2);
        let secret = |key: &str| match trial.secret(key) {
            Some(value) => Ok(Some(value)),
            None => keys(key),
        };
        let resolve = |name: &str| lookup(name, "jira.other.com", &accounts, &secret);
        let mut used = vec![];
        assert_eq!(
            fill("{{a1.pin}}:{{a1.token}}", &resolve, &mut used).unwrap(),
            "1234:saved"
        );
        assert_eq!(used, ["1234", "saved"]);
        let before = |name: &str| lookup(name, "jira.acme.com", &accounts, &secret);
        assert!(
            fill("{{a1.token}}", &before, &mut vec![])
                .unwrap_err()
                .contains("only go to jira.other.com")
        );
        assert_eq!(trial.secret("b2.pin"), None);
        assert_eq!(with_trial(vec![account("b2", None)], None).len(), 1);
    }

    #[test]
    fn reads_what_check_account_returned() {
        let fields = ["site".to_string(), "token".to_string()];
        assert_eq!(
            verdict(
                Ok(Ok(
                    r#"{"text":{"en":"Connected as Ada","fr":"Connectée"},"detail":"12 projects"}"#
                        .into()
                )),
                &fields
            ),
            json!({"ok": true, "title": {"en": "Connected as Ada", "fr": "Connectée"}, "detail": "12 projects", "field": null})
        );
        assert_eq!(
            verdict(
                Ok(Err(
                    r#"{"title":"Refused","detail":"Expired","field":"token"}"#.into()
                )),
                &fields
            ),
            json!({"ok": false, "title": "Refused", "detail": "Expired", "field": "token"})
        );
        assert_eq!(
            verdict(
                Ok(Ok(r#"{"ok":false,"title":"Refused","field":"nope"}"#.into())),
                &fields
            ),
            json!({"ok": false, "title": "Refused", "detail": null, "field": null})
        );
        assert_eq!(
            verdict(Ok(Err("connection refused".into())), &fields),
            json!({"ok": false, "title": "connection refused", "detail": null, "field": null})
        );
        assert_eq!(
            verdict(Ok(Ok("  ".into())), &fields),
            json!({"ok": true, "title": null, "detail": null, "field": null})
        );
        assert_eq!(
            verdict(
                Ok(Ok(
                    r#"{"text":{"fr":"Sans anglais"},"detail":{"en":7}}"#.into()
                )),
                &fields
            ),
            json!({"ok": true, "title": null, "detail": null, "field": null})
        );
        assert_eq!(
            verdict(Err(NO_CHECK.into()), &fields),
            json!({"ok": false, "title": null, "detail": NO_CHECK, "field": null})
        );
    }

    #[test]
    fn fills_secrets_in_headers_and_hides_them_in_answers() {
        let keys = |key: &str| {
            Ok(match key {
                "token" => Some("s3cr3t".to_string()),
                "a1.token" => Some("t0k".to_string()),
                _ => None,
            })
        };
        let accounts = [account("a1", Some("jira.acme.com")), account("b2", None)];
        let resolve = |name: &str| lookup(name, "jira.acme.com", &accounts, &keys);
        let mut used = vec![];
        assert_eq!(
            fill("Bearer {{token}}", &resolve, &mut used).unwrap(),
            "Bearer s3cr3t"
        );
        assert_eq!(fill("plain {{", &resolve, &mut used).unwrap(), "plain {{");
        assert!(
            fill("{{other}}", &resolve, &mut used)
                .unwrap_err()
                .contains("Settings › Extensions")
        );
        assert_eq!(
            fill("Basic {{base64:a1.email:a1.token}}", &resolve, &mut used).unwrap(),
            "Basic bWVAYWNtZS5jb206dDBr"
        );
        assert_eq!(
            fill("{{a1.email}}", &resolve, &mut used).unwrap(),
            "me@acme.com"
        );
        assert_eq!(used, ["s3cr3t", "t0k", "bWVAYWNtZS5jb206dDBr"]);
        assert!(fill("{{a1.nope}}", &resolve, &mut used).is_err());
        assert!(fill("{{zz.token}}", &resolve, &mut used).is_err());
        assert!(fill("{{../x}}", &resolve, &mut used).is_err());
        assert!(
            fill("{{b2.token}}", &resolve, &mut used)
                .unwrap_err()
                .contains("is not set")
        );

        let elsewhere = |name: &str| lookup(name, "api.github.com", &accounts, &keys);
        assert!(
            fill("{{base64:a1.email:a1.token}}", &elsewhere, &mut vec![])
                .unwrap_err()
                .contains("only go to jira.acme.com")
        );
        assert!(fill("{{token}}", &elsewhere, &mut vec![]).is_ok());

        assert_eq!(
            scrub("echo: Bearer s3cr3t from me@acme.com".into(), &used),
            "echo: Bearer [secret] from me@acme.com"
        );
        assert_eq!(
            scrub_bytes(b"\x00t0k\xfft0".to_vec(), &used),
            b"\x00[secret]\xfft0".to_vec()
        );
        assert_eq!(
            secret_key("hello", "token").unwrap(),
            "extension.hello.token"
        );
        assert!(secret_key("hello", "../x").is_err());
        assert!(secret_key("hello", "").is_err());
    }

    #[test]
    fn keeps_downloads_and_opened_files_inside_the_data_folder() {
        let data = std::env::temp_dir().join(format!("squadron-data-{}", uuid::Uuid::new_v4()));
        assert_eq!(
            data_file(&data, "jira/12/shot.png").unwrap(),
            data.join("jira/12/shot.png")
        );
        for bad in [
            "",
            "../x",
            "/etc/passwd",
            "a/../../x",
            ".hidden",
            "a/.git/x",
            "a\\b",
            "./a",
        ] {
            assert!(data_file(&data, bad).is_err(), "{bad}");
        }
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("doc.txt"), "x").unwrap();
        let outside = data.with_extension("txt");
        std::fs::write(&outside, "x").unwrap();
        assert!(inside_data(&data, &data.join("doc.txt")).is_ok());
        assert!(inside_data(&data, &data.join("../").join(outside.file_name().unwrap())).is_err());
        assert!(inside_data(&data, &outside).is_err());
        assert!(inside_data(&data, &data.join("gone.txt")).is_err());
    }

    #[test]
    fn feeds_the_standard_input_and_runs_squadron_s_claude() {
        let (mut host, _) = self::host(&["exec:wc", "exec:claude"], |_, _| Ok(Value::Null));
        use squadron::api::host::Host as _;
        let input = "x".repeat(5 << 20);
        let out = host
            .exec_input("wc".into(), vec!["-c".into()], None, input)
            .unwrap();
        assert_eq!(out.stdout.trim(), (5 << 20).to_string());
        assert!(
            host.exec_input("cat".into(), vec![], None, "x".into())
                .is_err()
        );
        host.hooks.claude = Arc::new(|| "echo".into());
        let out = host.exec("claude".into(), vec!["hi".into()], None).unwrap();
        assert_eq!(out.stdout.trim(), "hi");
    }

    #[test]
    #[ignore = "builds the example extension, needs the wasm32-wasip2 target"]
    fn opens_and_fills_a_new_agent_draft() {
        let dir = hello().expect("the example extension builds");
        let states = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        let told = Arc::new(Mutex::new(Vec::<Value>::new()));
        let record = told.clone();
        let runtime = Runtime::new(states, move |v| record.lock().unwrap().push(v)).unwrap();
        assert!(
            runtime
                .sync(vec![Spec {
                    id: "hello".into(),
                    dir: dir.to_string_lossy().into(),
                    wasm: "target/wasm32-wasip2/release/hello.wasm".into(),
                    allowed: vec![],
                    panels: vec![],
                    forge: None,
                }])
                .is_empty()
        );
        let agent = AgentInfo {
            id: "a1".into(),
            name: "Ada".into(),
            project: dir.to_string_lossy().into(),
            dir: dir.to_string_lossy().into(),
        };

        runtime.run("hello", "draft", Some(agent.clone())).unwrap();
        let draft = told
            .lock()
            .unwrap()
            .iter()
            .find(|v| v["call"] == "open-new-agent")
            .map(|v| v["draft"].as_str().unwrap().to_string())
            .unwrap();
        runtime.event(
            Some("hello"),
            "new-agent.regenerate",
            None,
            &json!({"draft": draft, "choices": ["short"]}).to_string(),
        );
        runtime.event(
            Some("hello"),
            "new-agent.started",
            Some(agent),
            &json!({"draft": draft}).to_string(),
        );
        runtime.run("hello", "nope", None).unwrap_err();

        let told: Vec<_> = told
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v["call"] != "set-badge")
            .cloned()
            .collect();
        let calls: Vec<_> = told.iter().map(|v| v["call"].as_str().unwrap()).collect();
        assert_eq!(
            calls,
            [
                "open-new-agent",
                "update-new-agent",
                "update-new-agent",
                "notify"
            ]
        );
        assert_eq!(told[0]["extension"], "hello");
        assert_eq!(told[0]["agent"], "a1");
        assert_eq!(uuid::Uuid::parse_str(&draft).map(|_| ()), Ok(()));
        let fields = |i: usize| -> Value {
            serde_json::from_str(told[i]["fields"].as_str().unwrap()).unwrap()
        };
        assert_eq!(fields(0)["name"], "Hello task");
        assert_eq!(fields(0)["status"]["state"], "running");
        assert_eq!(told[1]["draft"], draft.as_str());
        assert_eq!(
            fields(1)["text"],
            "Say hello, then list the files you would read first."
        );
        assert_eq!(fields(1)["status"]["state"], "done");
        assert_eq!(told[2]["draft"], draft.as_str());
        assert_eq!(fields(2)["text"], "Say hello.");
        assert_eq!(told[3]["body"], "Ada started from the Hello draft.");
    }

    #[test]
    #[ignore = "needs the network"]
    fn fetches_a_real_page() {
        let answer = send(
            &["net:api.github.com".into()],
            &[],
            Request {
                method: "get".into(),
                url: "https://api.github.com/zen".into(),
                headers: vec![("User-Agent".into(), "squadron-test".into())],
                body: None,
            },
            MAX_BODY,
            FETCH,
            |_| Ok(None),
        )
        .unwrap();
        assert_eq!(answer.status, 200);
        assert!(!answer.body.is_empty());
    }

    #[test]
    #[ignore = "needs the network"]
    fn drops_the_authorization_on_a_redirect_to_another_host() {
        let answer = send(
            &["net:postman-echo.com".into(), "net:httpbingo.org".into()],
            &[],
            Request {
                method: "GET".into(),
                url:
                    "https://postman-echo.com/redirect-to?url=https%3A%2F%2Fhttpbingo.org%2Fheaders"
                        .into(),
                headers: vec![
                    ("Authorization".into(), "Bearer {{token}}".into()),
                    ("User-Agent".into(), "squadron-test".into()),
                ],
                body: None,
            },
            MAX_BODY,
            FETCH,
            |k| Ok((k == "token").then(|| "s3cr3t".to_string())),
        )
        .unwrap();
        let body = String::from_utf8_lossy(&answer.body);
        assert_eq!(answer.status, 200, "{body}");
        assert!(body.contains("squadron-test"), "{body}");
        assert!(!body.to_lowercase().contains("authorization"), "{body}");
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
            panels: vec![],
            forge: None,
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

    type Told = Arc<Mutex<Vec<Value>>>;

    fn forge_spec(dir: &Path, allowed: Vec<String>) -> Spec {
        Spec {
            id: "forge".into(),
            dir: dir.to_string_lossy().into(),
            wasm: "target/wasm32-wasip2/release/forge.wasm".into(),
            allowed,
            panels: vec![],
            forge: None,
        }
    }

    fn forge_fixture() -> (Runtime, Told, PathBuf, PathBuf) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forge");
        let built = Command::new("cargo")
            .args(["build", "--release", "--target", "wasm32-wasip2", "--quiet"])
            .current_dir(&dir)
            .status()
            .is_ok_and(|s| s.success());
        assert!(built, "the forge fixture builds");
        let dir = dir.canonicalize().unwrap();
        let states = std::env::temp_dir().join(format!("squadron-wasm-{}", uuid::Uuid::new_v4()));
        let told = Arc::new(Mutex::new(Vec::<Value>::new()));
        let record = told.clone();
        let runtime =
            Runtime::new(states.clone(), move |v| record.lock().unwrap().push(v)).unwrap();
        (runtime, told, states, dir)
    }

    fn at_once(runtime: &Arc<Runtime>, inputs: &[&str]) -> Duration {
        let started = Instant::now();
        let calls: Vec<_> = inputs
            .iter()
            .map(|input| {
                let (runtime, input) = (runtime.clone(), input.to_string());
                std::thread::spawn(move || runtime.forge("forge", "wait", &input))
            })
            .collect();
        for call in calls {
            assert_eq!(call.join().unwrap().unwrap().unwrap().len(), 3);
        }
        started.elapsed()
    }

    #[test]
    #[ignore = "builds the forge fixture, needs the wasm32-wasip2 target"]
    fn runs_forge_calls_on_up_to_four_instances_that_share_the_state() {
        let (runtime, _, states, dir) = forge_fixture();
        assert!(runtime.sync(vec![forge_spec(&dir, vec![])]).is_empty());
        let runtime = Arc::new(runtime);

        let unknown: Value =
            serde_json::from_str(&runtime.forge("forge", "lists", "{}").unwrap().unwrap_err())
                .unwrap();
        assert_eq!(unknown["code"], "unsupported");
        runtime.forge("forge", "lists", "{}").unwrap().unwrap_err();
        assert_eq!(
            runtime.tool("forge", "unsupported", "{}", None).unwrap(),
            "1"
        );

        let four = at_once(&runtime, &["600", "601", "602", "603"]);
        assert!(four < Duration::from_millis(1500), "{four:?}");
        let five = at_once(&runtime, &["500", "501", "502", "503", "504"]);
        assert!(five >= Duration::from_millis(1000), "{five:?}");

        for _ in 0..3 {
            runtime.forge("forge", "bump", "{}").unwrap().unwrap();
        }
        assert_eq!(
            runtime.tool("forge", "wait-603", "{}", None).unwrap(),
            "603"
        );
        assert_eq!(runtime.tool("forge", "bumps", "{}", None).unwrap(), "3");
        let saved: Map<String, Value> =
            serde_json::from_slice(&std::fs::read(states.join("forge.json")).unwrap()).unwrap();
        assert_eq!(saved.len(), 11, "{saved:?}");
    }

    #[test]
    #[ignore = "builds the forge fixture, needs the wasm32-wasip2 target"]
    fn joins_a_forge_call_identical_to_one_in_flight() {
        let (runtime, _, _, dir) = forge_fixture();
        assert!(runtime.sync(vec![forge_spec(&dir, vec![])]).is_empty());
        let runtime = Arc::new(runtime);
        at_once(&runtime, &["400", "400", "400"]);
        assert_eq!(runtime.tool("forge", "waits", "{}", None).unwrap(), "1");
        at_once(&runtime, &["400", "401"]);
        assert_eq!(runtime.tool("forge", "waits", "{}", None).unwrap(), "2");
        assert_eq!(
            runtime.tool("forge", "wait-401", "{}", None).unwrap(),
            "401"
        );
    }

    #[test]
    #[ignore = "builds the forge fixture, needs the wasm32-wasip2 target"]
    fn forge_calls_follow_permission_changes_and_outlive_a_trap() {
        let (runtime, told, _, dir) = forge_fixture();
        assert!(runtime.sync(vec![forge_spec(&dir, vec![])]).is_empty());
        let refused = runtime.forge("forge", "git", "{}").unwrap().unwrap_err();
        assert!(refused.contains("git is not allowed"), "{refused}");

        assert!(
            runtime
                .sync(vec![forge_spec(&dir, vec!["exec:git".into()])])
                .is_empty()
        );
        let version = runtime.forge("forge", "git", "{}").unwrap().unwrap();
        assert!(version.starts_with("git version"), "{version}");

        runtime.forge("forge", "bump", "{}").unwrap().unwrap();
        let trapped = runtime.forge("forge", "trap", "{}").unwrap_err();
        assert!(trapped.starts_with("The extension stopped"), "{trapped}");
        assert!(told.lock().unwrap().iter().any(|v| v["call"] == "error"));
        assert_eq!(runtime.forge("forge", "bump", "{}").unwrap().unwrap(), "2");
        assert!(
            runtime
                .forge("other", "bump", "{}")
                .unwrap_err()
                .contains("not running")
        );
    }

    #[test]
    #[ignore = "builds the forge fixture, needs the wasm32-wasip2 target"]
    fn routes_a_remote_to_the_extension_that_provides_its_forge() {
        let (runtime, _, _, dir) = forge_fixture();
        let runtime = runtime.with_hooks(Hooks {
            accounts: Arc::new(|_| {
                vec![Stored {
                    id: "a1".into(),
                    values: BTreeMap::new(),
                    host: Some("git.acme.io".into()),
                    secrets: vec![],
                }]
            }),
            ..Hooks::default()
        });
        let spec = Spec {
            forge: Some(ForgeSpec {
                remotes: vec![r"(?<site>{account})/(?<path>[a-z/]+)".into()],
                calls: vec!["me".into(), "runs".into()],
                ..ForgeSpec::default()
            }),
            ..forge_spec(&dir, vec![])
        };
        assert!(runtime.sync(vec![spec]).is_empty());
        let providers = runtime.providers();
        assert_eq!(providers[0].hosts, ["git.acme.io"]);
        let target =
            crate::provider::remote_target(&providers, "git@git.acme.io:team/shop.git").unwrap();
        let me: crate::forge::Person =
            crate::provider::ask(&runtime, &target, "me", json!({})).unwrap();
        assert_eq!(me.name, "git.acme.io/team/shop on git.acme.io");
        let refused = crate::provider::ask::<Value>(&runtime, &target, "runs", json!({}));
        assert_eq!(refused.unwrap_err(), "This extension does not answer runs.");
        let undeclared = crate::provider::ask::<Value>(&runtime, &target, "merge", json!({}));
        assert_eq!(undeclared.unwrap_err(), "forge does not answer merge.");

        assert!(runtime.sync(vec![]).is_empty());
        assert!(runtime.providers().is_empty());
    }
}
