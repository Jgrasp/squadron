pub mod agents;
pub mod attach;
pub mod claude;
pub mod editors;
pub mod forge;
pub mod git;
pub mod permissions;
pub mod pilot;
pub mod projects;
pub mod sessions;
pub mod system;
pub mod terminal;

use agents::Agents;
use permissions::Gate;
use pilot::Pilot;
use projects::{Card, Folder, Project};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use terminal::Terminals;

const RELEASE_ID: &str = "dev.squadron.desktop";
const DEV_ID: &str = "dev.squadron.desktop.dev";

struct Projects {
    file: PathBuf,
    list: Mutex<Vec<Project>>,
}

impl Projects {
    fn views(list: &[Project]) -> Vec<Folder> {
        list.iter().map(Folder::of).collect()
    }

    fn update(
        &self,
        change: impl FnOnce(&mut Vec<Project>) -> Result<(), String>,
    ) -> Result<Vec<Folder>, String> {
        let mut list = self.list.lock().unwrap();
        let mut next = list.clone();
        change(&mut next)?;
        projects::paint(&mut next);
        projects::save(&self.file, &next).map_err(|e| format!("Could not save: {e}"))?;
        *list = next;
        Ok(Self::views(&list))
    }
}

fn check_new_folder(list: &[Project], path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("This folder does not exist or cannot be read.".into());
    }
    match list.iter().find(|p| projects::same_folder(&p.path, path)) {
        Some(p) => Err(format!("This folder is already the {} project.", p.name)),
        None => Ok(()),
    }
}

#[derive(Serialize)]
struct Inspection {
    #[serde(flatten)]
    folder: Folder,
    taken_by: Option<String>,
    taken_archived: bool,
    near: Option<PathBuf>,
}

#[tauri::command]
fn list_projects(projects: State<Projects>) -> Vec<Folder> {
    Projects::views(&projects.list.lock().unwrap())
}

#[tauri::command]
fn inspect_folder(projects: State<Projects>, path: PathBuf) -> Inspection {
    let list = projects.list.lock().unwrap();
    let taken = list.iter().find(|p| projects::same_folder(&p.path, &path));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Inspection {
        folder: Folder::of(&Project {
            name,
            path: path.clone(),
            ..Default::default()
        }),
        taken_by: taken.map(|p| p.name.clone()),
        taken_archived: taken.is_some_and(|p| p.archived),
        near: projects::near(&path),
    }
}

#[tauri::command]
fn add_project(
    projects: State<Projects>,
    path: PathBuf,
    name: String,
    color: Option<String>,
) -> Result<Vec<Folder>, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Give the project a name.".into());
    }
    let color = color
        .filter(|c| {
            c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
        })
        .unwrap_or_default();
    projects.update(|list| {
        check_new_folder(list, &path)?;
        list.push(Project {
            name,
            path,
            color,
            ..Default::default()
        });
        Ok(())
    })
}

#[tauri::command]
fn remove_project(projects: State<Projects>, path: PathBuf) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        list.retain(|p| p.path != path);
        Ok(())
    })
}

#[tauri::command]
fn add_rule(projects: State<Projects>, path: PathBuf, rule: String) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        let project = list
            .iter_mut()
            .find(|p| p.path == path)
            .ok_or("Unknown project.")?;
        if !project.rules.contains(&rule) {
            project.rules.push(rule);
        }
        Ok(())
    })
}

#[tauri::command]
fn remove_rule(
    projects: State<Projects>,
    path: PathBuf,
    rule: String,
) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        if let Some(project) = list.iter_mut().find(|p| p.path == path) {
            project.rules.retain(|r| r != &rule);
        }
        Ok(())
    })
}

#[tauri::command]
fn set_cards(
    projects: State<Projects>,
    path: PathBuf,
    cards: Vec<Card>,
) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        list.iter_mut()
            .find(|p| p.path == path)
            .ok_or("Unknown project.")?
            .cards = cards;
        Ok(())
    })
}

#[tauri::command]
fn answer_permission(
    gate: State<Gate>,
    id: String,
    decision: serde_json::Value,
) -> Result<(), String> {
    gate.answer(&id, decision)
}

#[tauri::command]
fn cancel_permissions(gate: State<Gate>, agent: String) {
    gate.cancel(&agent)
}

#[tauri::command]
fn edit_project(
    projects: State<Projects>,
    path: PathBuf,
    name: Option<String>,
    color: Option<String>,
    pinned: Option<bool>,
    archived: Option<bool>,
) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        let project = list
            .iter_mut()
            .find(|p| p.path == path)
            .ok_or("Unknown project.")?;
        if let Some(name) = name.map(|n| n.trim().to_string()) {
            if name.is_empty() {
                return Err("The project needs a name.".into());
            }
            project.name = name;
        }
        if let Some(color) = color {
            project.color = color;
        }
        if let Some(pinned) = pinned {
            project.pinned = pinned;
        }
        if let Some(archived) = archived {
            project.archived = archived;
        }
        Ok(())
    })
}

#[tauri::command]
fn order_projects(projects: State<Projects>, paths: Vec<PathBuf>) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        projects::reorder(list, &paths);
        Ok(())
    })
}

#[tauri::command]
fn move_project(
    projects: State<Projects>,
    path: PathBuf,
    to: PathBuf,
) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        check_new_folder(list, &to)?;
        if let Some(p) = list.iter_mut().find(|p| p.path == path) {
            p.path = to;
        }
        Ok(())
    })
}

struct Settings {
    file: PathBuf,
    config: Mutex<Config>,
}

#[derive(Clone, Serialize, serde::Deserialize)]
#[serde(default)]
struct Config {
    claude: String,
    defaults: claude::Options,
    tab_effect: String,
    tab_ring: String,
    tab_order: String,
    keys: std::collections::BTreeMap<String, String>,
    editors: Vec<editors::App>,
    pilot: bool,
    pilot_all: bool,
    archive_after_days: u32,
    steps: String,
    thinking: String,
    language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude: "claude".into(),
            defaults: claude::Options {
                model: "opus".into(),
                effort: "high".into(),
                mode: "default".into(),
            },
            tab_effect: "pulse".into(),
            tab_ring: "shake".into(),
            tab_order: "manual".into(),
            keys: Default::default(),
            editors: Vec::new(),
            pilot: true,
            pilot_all: false,
            archive_after_days: 0,
            steps: "grouped".into(),
            thinking: "duration".into(),
            language: "system".into(),
        }
    }
}

impl Settings {
    fn load(file: PathBuf) -> Self {
        let config = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok())
            .unwrap_or_default();
        Self {
            file,
            config: Mutex::new(config),
        }
    }

    fn claude(&self) -> String {
        self.config.lock().unwrap().claude.clone()
    }

    fn save(&self, mut config: Config) -> Result<(), String> {
        config.defaults.check()?;
        if !["pulse", "snake"].contains(&config.tab_effect.as_str()) {
            return Err(format!("Unknown tab effect: {}", config.tab_effect));
        }
        if !["off", "shake", "sound"].contains(&config.tab_ring.as_str()) {
            return Err(format!("Unknown tab ring: {}", config.tab_ring));
        }
        if !["manual", "name", "activity"].contains(&config.tab_order.as_str()) {
            return Err(format!("Unknown tab order: {}", config.tab_order));
        }
        config.keys.retain(|_, value| !value.trim().is_empty());
        config
            .editors
            .retain(|e| !e.name.trim().is_empty() && !e.command.trim().is_empty());
        config.claude = config.claude.trim().to_string();
        if config.claude.is_empty() {
            config.claude = "claude".into();
        }
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(&self.file, text).map_err(|e| e.to_string())?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }
}

#[tauri::command]
fn get_settings(settings: State<Settings>) -> Config {
    settings.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(
    settings: State<Settings>,
    agents: State<Agents>,
    config: Config,
) -> Result<(), String> {
    agents.set_pilot(config.pilot);
    settings.save(config)
}

#[tauri::command]
fn remote_control(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    agents.remote(&app, &settings.claude(), &id, enabled)
}

#[tauri::command]
fn set_agent_options(
    agents: State<Agents>,
    id: String,
    options: claude::Options,
) -> Result<(), String> {
    agents.set_options(&id, options)
}

#[tauri::command]
fn pilot_reply(pilot: State<Pilot>, id: String, result: String) -> Result<(), String> {
    pilot.reply(&id, result)
}

#[tauri::command]
fn set_claude_path(settings: State<Settings>, path: String) -> Result<(), String> {
    let mut config = settings.config.lock().unwrap().clone();
    config.claude = path;
    settings.save(config)
}

#[tauri::command]
async fn suggest_title(settings: State<'_, Settings>, text: String) -> Result<String, String> {
    let program = settings.claude();
    let task: String = text.chars().take(2000).collect();
    let output = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new(program)
            .current_dir(std::env::temp_dir())
            .args([
                "-p",
                "--model",
                "haiku",
                "--effort",
                "low",
                "--output-format",
                "json",
                "--tools",
                "",
                "--strict-mcp-config",
                "--setting-sources",
                "project",
                "--no-session-persistence",
                "--system-prompt",
                "You name software tasks. Reply with a title of 2 to 5 words in the language of the task, with no quotes and no final punctuation.",
                &task,
            ])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let reply: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let title = reply["result"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\'', '.'])
        .chars()
        .take(60)
        .collect::<String>();
    if reply["is_error"].as_bool().unwrap_or(true) || title.is_empty() {
        return Err("No title suggested.".into());
    }
    Ok(title)
}

#[tauri::command]
fn claude_version(path: String) -> Result<String, String> {
    let output = std::process::Command::new(path.trim())
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
fn read_project_file(project: PathBuf, path: PathBuf) -> Result<Option<String>, String> {
    let file = projects::inside(&project, &path).ok_or("File outside the project.")?;
    if std::fs::metadata(&file).is_ok_and(|m| m.len() > 5_000_000) {
        return Err("File too large to show.".into());
    }
    match std::fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Err("Not a text file.".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn write_project_file(project: PathBuf, path: PathBuf, text: String) -> Result<(), String> {
    let file = projects::inside(&project, &path).ok_or("File outside the project.")?;
    std::fs::write(file, text).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Entry {
    name: String,
    dir: bool,
}

#[tauri::command]
fn list_dir(project: PathBuf, path: PathBuf) -> Result<Vec<Entry>, String> {
    let dir = projects::inside(&project, &path).ok_or("Folder outside the project.")?;
    let mut entries: Vec<Entry> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.file_name() != ".git")
        .map(|e| Entry {
            name: e.file_name().to_string_lossy().into_owned(),
            dir: e.path().is_dir(),
        })
        .collect();
    entries.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
async fn git_file(project: PathBuf, rev: String, path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || git::show(&project, &rev, &path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn term_open(
    app: AppHandle,
    terminals: State<Terminals>,
    project: PathBuf,
    cols: u16,
    rows: u16,
) -> Result<serde_json::Value, String> {
    let shell = std::env::var("SHELL").ok();
    let shell = shell
        .as_deref()
        .and_then(|s| Path::new(s).file_name()?.to_str())
        .unwrap_or("shell")
        .to_string();
    let out = app.clone();
    let id = terminals.open(
        &project,
        None,
        cols,
        rows,
        move |id, data| {
            let _ = out.emit("term-out", serde_json::json!({ "id": id, "data": data }));
        },
        move |id| {
            let _ = app.emit("term-exit", id);
        },
    )?;
    Ok(serde_json::json!({ "id": id, "shell": shell }))
}

#[tauri::command]
fn term_write(terminals: State<Terminals>, id: u32, data: String) -> Result<(), String> {
    terminals.write(id, &data)
}

#[tauri::command]
fn term_resize(terminals: State<Terminals>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
    terminals.resize(id, cols, rows)
}

#[tauri::command]
fn term_close(terminals: State<Terminals>, id: u32) {
    terminals.close(id)
}

#[tauri::command]
async fn git_changes(project: PathBuf) -> Result<git::Changes, String> {
    tauri::async_runtime::spawn_blocking(move || git::changes(&project))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pull_request(dir: PathBuf) -> Result<Option<forge::Pull>, String> {
    forge::pull(&dir).await
}

#[tauri::command]
async fn pull_lists(dir: PathBuf) -> Result<forge::Lists, String> {
    forge::lists(&dir).await
}

#[tauri::command]
async fn changed_files(dir: PathBuf, base: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || git::changed_files(&dir, &base))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn review_checkout(
    app: AppHandle,
    project: PathBuf,
    number: u64,
    branch: String,
    base: String,
) -> Result<PathBuf, String> {
    let name = project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let into = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("reviews")
        .join(format!("{name}-{number}"));
    std::fs::create_dir_all(into.parent().unwrap()).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        git::review_checkout(&project, &into, &branch, &base).map(|_| into)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pull_changes(
    dir: PathBuf,
    base: String,
    branch: String,
    head: String,
    since: Option<String>,
) -> Result<git::Pulled, String> {
    tauri::async_runtime::spawn_blocking(move || {
        git::pull_changes(&dir, &base, &branch, &head, since.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pull_remarks(dir: PathBuf, number: u64) -> Result<Vec<forge::Remark>, String> {
    forge::remarks(&dir, number).await
}

#[tauri::command]
async fn pull_reply(
    dir: PathBuf,
    number: u64,
    to: u64,
    kind: String,
    body: String,
) -> Result<(), String> {
    forge::reply(&dir, number, to, kind, body).await
}

#[tauri::command]
async fn post_remarks(
    dir: PathBuf,
    number: u64,
    head: String,
    drafts: Vec<forge::Draft>,
    summary: String,
    verdict: String,
) -> Result<forge::Posted, String> {
    forge::post(&dir, number, head, drafts, summary, verdict).await
}

#[tauri::command]
async fn merge_pull(dir: PathBuf, number: u64) -> Result<(), String> {
    forge::merge(&dir, number).await
}

#[tauri::command]
async fn forge_accounts() -> forge::Accounts {
    tauri::async_runtime::spawn_blocking(forge::accounts)
        .await
        .unwrap_or(forge::Accounts {
            bitbucket: None,
            bitbucket_user: false,
            github: None,
        })
}

#[tauri::command]
async fn bitbucket_sign_in(email: String, token: String) -> Result<(), String> {
    forge::sign_in(email.trim().to_string(), token.trim().to_string()).await
}

#[tauri::command]
fn bitbucket_sign_out() -> Result<(), String> {
    forge::sign_out()
}

#[tauri::command]
fn subagent_log(session: String, task: String) -> Result<Option<String>, String> {
    let safe =
        |id: &str| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    if !safe(&session) || !safe(&task) {
        return Err("Invalid session or subagent id.".into());
    }
    let root = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude")))
        .ok_or("No Claude Code folder.")?
        .join("projects");
    let file = format!("agent-{task}.jsonl");
    Ok(std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|dir| dir.path().join(&session).join("subagents").join(&file))
        .find(|path| path.is_file())
        .and_then(|path| std::fs::read_to_string(path).ok()))
}

#[tauri::command]
async fn git_patch(
    project: PathBuf,
    from: String,
    now: String,
    path: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || git::patch(&project, &from, &now, &path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn detect_editors() -> Vec<editors::App> {
    tauri::async_runtime::spawn_blocking(editors::detect)
        .await
        .unwrap_or_default()
}

#[tauri::command]
fn open_with(command: String, project: PathBuf, path: PathBuf) -> Result<(), String> {
    let file = if path.as_os_str().is_empty() {
        None
    } else {
        Some(projects::inside(&project, &path).ok_or("File outside the project.")?)
    };
    editors::open(&command, &project, file.as_deref())
}

#[tauri::command]
fn open_in_editor(project: PathBuf, path: PathBuf) -> Result<(), String> {
    let file = projects::inside(&project, &path).ok_or("File outside the project.")?;
    tauri_plugin_opener::open_path(file, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn create_agent(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    project: PathBuf,
    options: claude::Options,
    text: String,
    worktree: Option<String>,
    dir: Option<PathBuf>,
) -> Result<(), String> {
    agents.register(&id, project, options, None, false)?;
    agents.set_dir(&id, dir)?;
    if let Some(name) = worktree {
        agents.new_worktree(&id, &name)?;
    }
    if text.trim().is_empty() {
        return Ok(());
    }
    agents.send(&app, &settings.claude(), &id, serde_json::json!(text))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn restore_agent(
    agents: State<Agents>,
    id: String,
    project: PathBuf,
    options: claude::Options,
    session_id: Option<String>,
    fork: bool,
    at: Option<String>,
    dir: Option<PathBuf>,
) -> Result<(), String> {
    agents.register(&id, project, options, session_id, fork)?;
    agents.set_dir(&id, dir)?;
    match at {
        Some(at) => agents.fork_at(&id, &at),
        None => Ok(()),
    }
}

#[tauri::command]
fn set_workdir(agents: State<Agents>, id: String, dir: Option<PathBuf>) -> Result<(), String> {
    agents.set_dir(&id, dir)
}

#[tauri::command]
fn fork_agent(
    agents: State<Agents>,
    id: String,
    from: String,
    project: PathBuf,
    options: claude::Options,
    at: Option<String>,
) -> Result<(), String> {
    let session = agents
        .session_id(&from)
        .ok_or("This agent has no session to fork yet.")?;
    agents.register(&id, project, options, Some(session), true)?;
    match at {
        Some(at) => agents.fork_at(&id, &at),
        None => Ok(()),
    }
}

#[tauri::command]
fn system_load(cpu: State<system::Cpu>) -> Option<system::Load> {
    cpu.load()
}

struct AgentFiles(PathBuf);

struct LayoutFile(PathBuf);

#[tauri::command]
fn get_layout(file: State<LayoutFile>) -> serde_json::Value {
    std::fs::read_to_string(&file.0)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}))
}

#[tauri::command]
fn set_layout(file: State<LayoutFile>, layout: serde_json::Value) -> Result<(), String> {
    if !layout.is_object() {
        return Err("The layout must be an object.".into());
    }
    let tmp = file.0.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&layout).map_err(|e| e.to_string())?,
    )
    .and_then(|_| std::fs::rename(&tmp, &file.0))
    .map_err(|e| e.to_string())
}

impl AgentFiles {
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Invalid agent id.".into());
        }
        Ok(self.0.join(format!("{id}.json")))
    }

    fn archived(&self, id: &str) -> Result<PathBuf, String> {
        let file = self.path(id)?;
        Ok(self.0.join("archived").join(file.file_name().unwrap()))
    }
}

#[tauri::command]
fn save_agent(files: State<AgentFiles>, id: String, data: String) -> Result<(), String> {
    let path = files.path(&id)?;
    std::fs::create_dir_all(&files.0).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_agent_file(app: AppHandle, files: State<AgentFiles>, id: String) -> Result<(), String> {
    let path = files.path(&id)?;
    let gone = std::fs::read_to_string(&path).unwrap_or_default();
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => {
            forget_attachments(&app, &files.0, &gone);
            Ok(())
        }
    }
}

fn forget_attachments(app: &AppHandle, agents: &Path, gone: &str) {
    if let Ok(data) = app.path().app_data_dir() {
        attach::forget(&data.join("attachments"), agents, gone);
    }
}

#[tauri::command]
fn archive_agent_file(files: State<AgentFiles>, id: String) -> Result<(), String> {
    let from = files.path(&id)?;
    let to = files.archived(&id)?;
    std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(from, to).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_archived(files: State<AgentFiles>, id: String) -> Result<String, String> {
    let from = files.archived(&id)?;
    let to = files.path(&id)?;
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    std::fs::read_to_string(to).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_archived(app: AppHandle, files: State<AgentFiles>, id: String) -> Result<(), String> {
    let path = files.archived(&id)?;
    let gone = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::remove_file(path).map_err(|e| e.to_string())?;
    forget_attachments(&app, &files.0, &gone);
    Ok(())
}

#[tauri::command]
fn list_archived(files: State<AgentFiles>) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(files.0.join("archived")) else {
        return Vec::new();
    };
    dir.filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect()
}

#[tauri::command]
fn agent_ids(files: State<AgentFiles>) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(&files.0) else {
        return Vec::new();
    };
    dir.filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            (path.extension()? == "json").then(|| path.file_stem()?.to_str().map(String::from))?
        })
        .collect()
}

#[tauri::command]
fn load_agent(files: State<AgentFiles>, id: String) -> Option<String> {
    std::fs::read_to_string(files.path(&id).ok()?).ok()
}

#[tauri::command]
fn load_agents(files: State<AgentFiles>) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(&files.0) else {
        return Vec::new();
    };
    dir.filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect()
}

#[tauri::command]
fn send_message(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    text: String,
    files: Option<Vec<PathBuf>>,
) -> Result<(), String> {
    let content = attach::content(&text, &files.unwrap_or_default())?;
    agents.send(&app, &settings.claude(), &id, content)
}

#[tauri::command]
fn save_attachment(app: AppHandle, name: String, data: String) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    attach::save(&dir.join("attachments"), &name, &data)
}

#[tauri::command]
async fn paste_image(
    app: AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<PathBuf>, String> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("attachments")
        .join(uuid::Uuid::new_v4().simple().to_string());
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        window
            .run_on_main_thread(move || {
                let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
                clipboard.request_image(move |_, image| {
                    let saved = image.map(|pixbuf| {
                        let file = folder.join("screenshot.png");
                        std::fs::create_dir_all(&folder)
                            .map_err(|e| e.to_string())
                            .and_then(|_| {
                                pixbuf.savev(&file, "png", &[]).map_err(|e| e.to_string())
                            })
                            .map(|_| file)
                    });
                    let _ = tx.send(saved);
                });
            })
            .map_err(|e| e.to_string())?;
        rx.await.unwrap_or(None).transpose()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, folder);
        Ok(None)
    }
}

#[tauri::command]
async fn project_sessions(project: PathBuf) -> Vec<sessions::Found> {
    tauri::async_runtime::spawn_blocking(move || sessions::list(&project))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn read_session(project: PathBuf, id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || sessions::transcript(&project, &id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_attachment(path: PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(&path).map_err(|_| "The file is gone.")?;
    let launcher = path.extension().is_some_and(|e| e == "desktop");
    if !meta.is_file() || meta.permissions().mode() & 0o111 != 0 || launcher {
        return Err("Squadron only opens documents and images from here.".into());
    }
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
async fn attachment_preview(path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || attach::preview(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn interrupt_agent(agents: State<Agents>, id: String) -> Result<(), String> {
    agents.interrupt(&id)
}

#[tauri::command]
fn git_branch(path: PathBuf) -> Result<Option<String>, String> {
    if !path.is_absolute() || !path.join(".git").exists() {
        return Err("Not a git checkout.".into());
    }
    Ok(projects::branch(&path))
}

#[cfg(target_os = "linux")]
fn new_browser_window(url: &str) -> Result<(), String> {
    use gtk::gio::{AppInfo, AppInfoCreateFlags, prelude::*};
    let browser = AppInfo::default_for_uri_scheme("https").ok_or("No default browser.")?;
    let program = browser.executable();
    let line = format!(
        "{} --new-window",
        shell_words::quote(&program.to_string_lossy())
    );
    let window =
        AppInfo::create_from_commandline(line, Some("Browser"), AppInfoCreateFlags::SUPPORTS_URIS)
            .map_err(|e| e.to_string())?;
    let context = gtk::gdk::Display::default().and_then(|d| d.app_launch_context());
    window
        .launch_uris(&[url], context.as_ref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_link(window: tauri::WebviewWindow, url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Only web links open from here.".into());
    }
    #[cfg(target_os = "linux")]
    {
        let parent = window.clone();
        window
            .run_on_main_thread(move || {
                let shown = new_browser_window(&url).or_else(|_| {
                    parent
                        .gtk_window()
                        .map_err(|e| e.to_string())
                        .and_then(|w| {
                            gtk::show_uri_on_window(Some(&w), &url, 0).map_err(|e| e.to_string())
                        })
                });
                if let Err(e) = shown {
                    log::error!("{url} did not open: {e}");
                }
            })
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn stop_turn(agents: State<Agents>, id: String) -> Result<(), String> {
    agents.stop_turn(&id)
}

#[tauri::command]
fn reload_agent(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
) -> Result<(), String> {
    agents.reload(&app, &settings.claude(), &id)
}

#[tauri::command]
fn archive_agent(agents: State<Agents>, id: String) {
    agents.archive(&id)
}

#[cfg(target_os = "macos")]
fn login_path() {
    use std::io::Read;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let Ok(mut child) = std::process::Command::new(shell)
        .args(["-ilc", "printf '\\n%s' \"$PATH\""])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return;
    };
    let start = std::time::Instant::now();
    while child.try_wait().ok().flatten().is_none() {
        if start.elapsed() > std::time::Duration::from_secs(3) {
            let _ = child.kill();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    let mut out = String::new();
    let _ = child.stdout.take().map(|mut o| o.read_to_string(&mut out));
    let found = out.lines().last().unwrap_or_default().trim().to_string();
    let home = std::env::var("HOME").unwrap_or_default();
    let local = format!("{home}/.local/bin");
    let path = [
        found.as_str(),
        "/opt/homebrew/bin",
        "/usr/local/bin",
        local.as_str(),
    ]
    .into_iter()
    .filter(|p| !p.is_empty())
    .collect::<Vec<_>>()
    .join(":");
    // SAFETY: runs first, before Squadron starts any thread.
    unsafe { std::env::set_var("PATH", path) };
}

#[derive(Serialize, serde::Deserialize)]
struct Place {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

fn keep_place(window: &tauri::Window) {
    let (Ok(at), Ok(size), Ok(maximized), Ok(dir)) = (
        window.outer_position(),
        window.inner_size(),
        window.is_maximized(),
        window.app_handle().path().app_config_dir(),
    ) else {
        return;
    };
    let place = Place {
        x: at.x,
        y: at.y,
        width: size.width,
        height: size.height,
        maximized,
    };
    if let Ok(text) = serde_json::to_string(&place) {
        let _ = std::fs::write(dir.join("window.json"), text);
    }
}

fn restore_place(app: &tauri::App, config: &std::path::Path) {
    let (Some(window), Some(place)) = (
        app.get_webview_window("main"),
        std::fs::read_to_string(config.join("window.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Place>(&text).ok()),
    ) else {
        return;
    };
    let _ = window.set_size(tauri::PhysicalSize::new(place.width, place.height));
    let at = tauri::PhysicalPosition::new(place.x, place.y);
    let _ = window.set_position(at);
    #[cfg(target_os = "linux")]
    if let Ok(gtk) = window.gtk_window() {
        use gtk::prelude::WidgetExtManual;
        let placed = std::cell::Cell::new(false);
        let target = window.clone();
        gtk.connect_map_event(move |_, _| {
            if !placed.replace(true) {
                let _ = target.set_position(at);
            }
            gtk::glib::Propagation::Proceed
        });
    }
    if place.maximized {
        let _ = window.maximize();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    for leaked in [
        "GIO_LAUNCHED_DESKTOP_FILE",
        "GIO_LAUNCHED_DESKTOP_FILE_PID",
        "LD_LIBRARY_PATH",
    ] {
        // SAFETY: runs first, before Squadron starts any thread.
        unsafe { std::env::remove_var(leaked) };
    }
    #[cfg(target_os = "macos")]
    login_path();
    #[cfg(all(debug_assertions, target_os = "linux"))]
    {
        gtk::glib::set_prgname(Some("squadron-dev"));
        // Wayland lets no window choose its position: X11 lets the dev build reopen where it was.
        gtk::gdk::set_allowed_backends("x11");
    }
    #[cfg(all(debug_assertions, target_os = "linux"))]
    if let Some(data) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        let _ = std::fs::remove_dir_all(data.join(DEV_ID).join("WebKitCache"));
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Agents::default())
        .manage(Terminals::default())
        .manage(system::Cpu::default())
        .invoke_handler(tauri::generate_handler![
            system_load,
            stop_turn,
            open_link,
            git_branch,
            set_workdir,
            agent_ids,
            load_agent,
            list_projects,
            inspect_folder,
            add_project,
            remove_project,
            move_project,
            edit_project,
            order_projects,
            archive_agent_file,
            restore_archived,
            delete_archived,
            list_archived,
            add_rule,
            answer_permission,
            pilot_reply,
            set_agent_options,
            remote_control,
            cancel_permissions,
            read_project_file,
            write_project_file,
            list_dir,
            git_file,
            term_open,
            term_write,
            term_resize,
            term_close,
            open_in_editor,
            detect_editors,
            open_with,
            git_changes,
            git_patch,
            pull_request,
            pull_lists,
            review_checkout,
            changed_files,
            pull_changes,
            pull_remarks,
            pull_reply,
            post_remarks,
            merge_pull,
            forge_accounts,
            bitbucket_sign_in,
            bitbucket_sign_out,
            subagent_log,
            get_settings,
            set_settings,
            set_claude_path,
            claude_version,
            suggest_title,
            remove_rule,
            set_cards,
            create_agent,
            restore_agent,
            fork_agent,
            save_agent,
            delete_agent_file,
            load_agents,
            get_layout,
            set_layout,
            send_message,
            save_attachment,
            attachment_preview,
            open_attachment,
            paste_image,
            project_sessions,
            read_session,
            interrupt_agent,
            reload_agent,
            archive_agent
        ])
        .setup(|app| {
            if cfg!(debug_assertions) && app.config().identifier != DEV_ID {
                return Err("Start the dev build with `pnpm app`: it keeps its own data, apart from the installed Squadron.".into());
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let config = app.path().app_config_dir()?;
            if cfg!(debug_assertions) && !config.exists() {
                std::fs::create_dir_all(&config)?;
                if let Some(release) = config.parent().map(|p| p.join(RELEASE_ID)) {
                    for name in ["projects.json", "settings.json", "layout.json"] {
                        let _ = std::fs::copy(release.join(name), config.join(name));
                    }
                }
            }
            if cfg!(debug_assertions) {
                restore_place(app, &config);
            }
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            let file = config.join("projects.json");
            let mut list = projects::load(&file);
            if projects::paint(&mut list) {
                let _ = projects::save(&file, &list);
            }
            app.manage(Projects {
                list: Mutex::new(list),
                file,
            });
            app.manage(Settings::load(config.join("settings.json")));
            app.manage(AgentFiles(config.join("agents")));
            let (store, agents) = (
                app.path().app_data_dir()?.join("attachments"),
                config.join("agents"),
            );
            std::thread::spawn(move || {
                attach::sweep(
                    &store,
                    &agents,
                    std::time::Duration::from_secs(7 * 24 * 3600),
                )
            });
            app.manage(LayoutFile(config.join("layout.json")));
            let handle = app.handle().clone();
            let token = uuid::Uuid::new_v4().simple().to_string();
            let gate = Gate::new(token.clone(), move |ask| {
                let _ = handle.emit("permission-request", ask);
            });
            let handle = app.handle().clone();
            let pilot = Pilot::new(token.clone(), move |call| {
                let _ = handle.emit_to("main", "pilot-request", call);
            });
            let port = tauri::async_runtime::block_on(permissions::serve_with_pilot(
                gate.clone(),
                pilot.clone(),
            ))?;
            let agents = app.state::<Agents>();
            agents.endpoint(port, token);
            agents.set_pilot(app.state::<Settings>().config.lock().unwrap().pilot);
            app.manage(gate);
            app.manage(pilot);
            Ok(())
        })
        .on_window_event(|window, event| {
            if cfg!(debug_assertions)
                && window.label() == "main"
                && matches!(
                    event,
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) | WindowEvent::CloseRequested { .. }
                )
            {
                keep_place(window);
            }
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Agents>().stop_all();
                app.state::<Terminals>().close_all();
            }
        });
}
