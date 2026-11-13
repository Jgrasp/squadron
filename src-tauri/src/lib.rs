pub mod account;
pub mod agents;
pub mod artifacts;
pub mod askpass;
pub mod attach;
pub mod azure;
pub mod claude;
pub mod definitions;
pub mod editors;
pub mod extension_tools;
pub mod extensions;
pub mod forge;
pub mod git;
pub mod gitlab;
pub mod keys;
mod links;
mod ondemand;
pub mod permissions;
pub mod pilot;
pub mod plugins;
pub mod preview;
mod process;
pub mod projects;
pub mod sessions;
pub mod ssh;
pub mod stored;
pub mod sudo;
pub mod system;
pub mod terminal;
pub mod wasm;

use agents::Agents;
use askpass::Askpass;
use permissions::Gate;
use pilot::Pilot;
use projects::{Card, Folder, Group, Project};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use terminal::Terminals;

const RELEASE_ID: &str = "dev.squadron.desktop";
const DEV_ID: &str = "dev.squadron.desktop.dev";

struct Projects {
    file: PathBuf,
    list: Mutex<Vec<Project>>,
    groups_file: PathBuf,
    groups: Mutex<Vec<Group>>,
}

#[derive(Serialize)]
struct Tree {
    groups: Vec<Group>,
    projects: Vec<Folder>,
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

    fn arrange(
        &self,
        change: impl FnOnce(&mut Vec<Group>, &mut Vec<Project>) -> Result<(), String>,
    ) -> Result<Tree, String> {
        let mut list = self.list.lock().unwrap();
        let mut groups = self.groups.lock().unwrap();
        let (mut next_list, mut next_groups) = (list.clone(), groups.clone());
        change(&mut next_groups, &mut next_list)?;
        projects::save_groups(&self.groups_file, &next_groups)
            .map_err(|e| format!("Could not save: {e}"))?;
        projects::save(&self.file, &next_list).map_err(|e| format!("Could not save: {e}"))?;
        *list = next_list;
        *groups = next_groups;
        Ok(Tree {
            groups: groups.clone(),
            projects: Self::views(&list),
        })
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
fn list_groups(projects: State<Projects>) -> Vec<Group> {
    projects.groups.lock().unwrap().clone()
}

#[tauri::command]
fn add_group(
    projects: State<Projects>,
    id: String,
    name: String,
    parent: String,
) -> Result<Tree, String> {
    projects.arrange(|groups, _| projects::add_group(groups, id, &name, parent))
}

#[tauri::command]
fn edit_group(
    projects: State<Projects>,
    id: String,
    name: Option<String>,
    parent: Option<String>,
) -> Result<Tree, String> {
    projects.arrange(|groups, _| projects::edit_group(groups, &id, name.as_deref(), parent))
}

#[tauri::command]
fn remove_group(projects: State<Projects>, id: String) -> Result<Tree, String> {
    projects.arrange(|groups, list| projects::dissolve(groups, list, &id))
}

#[tauri::command]
fn move_projects(
    projects: State<Projects>,
    paths: Vec<PathBuf>,
    group: String,
) -> Result<Tree, String> {
    projects.arrange(|groups, list| projects::place(groups, list, &paths, &group))
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
fn scratch_project(
    app: AppHandle,
    projects: State<Projects>,
    name: String,
) -> Result<(PathBuf, Vec<Folder>), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("conversations");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let list = projects.update(|list| {
        match list.iter_mut().find(|p| p.path == dir) {
            Some(project) => project.archived = false,
            None => list.push(Project {
                name,
                path: dir.clone(),
                ..Default::default()
            }),
        }
        Ok(())
    })?;
    Ok((dir, list))
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
fn cancel_permissions(gate: State<Gate>, askpass: State<Askpass>, agent: String) {
    gate.cancel(&agent);
    askpass.cancel(&agent);
}

#[tauri::command]
fn answer_askpass(askpass: State<Askpass>, id: String, yes: bool) -> Result<(), String> {
    askpass.answer(&id, yes)
}

#[tauri::command]
fn edit_project(
    projects: State<Projects>,
    path: PathBuf,
    changes: projects::Edit,
) -> Result<Vec<Folder>, String> {
    projects.update(|list| {
        let project = list
            .iter_mut()
            .find(|p| p.path == path)
            .ok_or("Unknown project.")?;
        if let Some(name) = changes.name.map(|n| n.trim().to_string()) {
            if name.is_empty() {
                return Err("The project needs a name.".into());
            }
            project.name = name;
        }
        if let Some(color) = changes.color {
            project.color = color;
        }
        if let Some(pinned) = changes.pinned {
            project.pinned = pinned;
        }
        if let Some(archived) = changes.archived {
            project.archived = archived;
        }
        if let Some(runs) = changes.runs {
            project.runs_off = !runs;
        }
        if let Some(notify) = changes.runs_notify {
            project.runs_quiet = !notify;
        }
        if let Some(hidden) = changes.runs_hidden {
            project.runs_hidden = hidden;
        }
        if let Some(repository) = changes.repository {
            project.repository = repository.trim().to_string();
        }
        if let Some(prompt) = changes.review_prompt {
            project.review_prompt = prompt.trim().to_string();
        }
        if let Some(defaults) = changes.defaults {
            claude::Options {
                model: defaults.model.clone().unwrap_or_else(|| "opus".into()),
                effort: defaults.effort.clone().unwrap_or_else(|| "high".into()),
                mode: defaults.mode.clone().unwrap_or_else(|| "default".into()),
                agent: None,
                advisor: defaults.advisor.clone(),
            }
            .check()?;
            project.defaults = defaults;
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
    agents: State<Agents>,
    path: PathBuf,
    to: PathBuf,
) -> Result<Vec<Folder>, String> {
    let list = projects.update(|list| {
        check_new_folder(list, &to)?;
        if let Some(p) = list.iter_mut().find(|p| p.path == path) {
            p.path = to.clone();
        }
        Ok(())
    })?;
    agents.rehome(&path, &to);
    sessions::carry(&path, &to);
    Ok(list)
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
    projects_effect: String,
    tab_ring: String,
    tab_order: String,
    tab_hours: u32,
    keys: std::collections::BTreeMap<String, String>,
    editors: Vec<editors::App>,
    pilot: bool,
    pilot_all: bool,
    archive_after_days: u32,
    artifacts_after_days: u32,
    artifacts_since: u64,
    artifacts_extensions: std::collections::BTreeMap<String, i64>,
    compact_window: u32,
    steps: String,
    thinking: String,
    language: String,
    runs: bool,
    runs_linger: u32,
    runs_hidden: Vec<String>,
    runs_notify: bool,
    review_prompt: String,
    conversation_width: u32,
    sidebar: String,
    plugin_lines: String,
    plugin_card: String,
    plugin_projects: std::collections::BTreeMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude: "claude".into(),
            defaults: claude::Options {
                model: "opus".into(),
                effort: "high".into(),
                mode: "default".into(),
                agent: None,
                advisor: None,
            },
            tab_effect: "pulse".into(),
            projects_effect: "snake".into(),
            tab_ring: "shake".into(),
            tab_order: "manual".into(),
            tab_hours: 4,
            keys: Default::default(),
            editors: Vec::new(),
            pilot: true,
            pilot_all: false,
            archive_after_days: 0,
            artifacts_after_days: 30,
            artifacts_since: 0,
            artifacts_extensions: Default::default(),
            compact_window: 0,
            steps: "grouped".into(),
            thinking: "duration".into(),
            language: "system".into(),
            runs: true,
            runs_linger: 10,
            runs_hidden: Vec::new(),
            runs_notify: false,
            review_prompt: String::new(),
            conversation_width: 740,
            sidebar: "pinned".into(),
            plugin_lines: "tag".into(),
            plugin_card: "normal".into(),
            plugin_projects: Default::default(),
        }
    }
}

impl Settings {
    fn load(file: PathBuf) -> Self {
        let config = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok())
            .unwrap_or_default();
        claude::COMPACT_WINDOW.store(config.compact_window, Ordering::Relaxed);
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
        if !["snake", "bar", "pulse"].contains(&config.projects_effect.as_str()) {
            return Err(format!("Unknown projects effect: {}", config.projects_effect));
        }
        if !["off", "shake", "sound"].contains(&config.tab_ring.as_str()) {
            return Err(format!("Unknown tab ring: {}", config.tab_ring));
        }
        if !["manual", "name", "activity"].contains(&config.tab_order.as_str()) {
            return Err(format!("Unknown tab order: {}", config.tab_order));
        }
        if config.tab_hours > 24 * 30 && config.tab_hours != u32::MAX {
            return Err(format!("Unknown tab delay: {}", config.tab_hours));
        }
        if config.compact_window != 0 && !(100_000..=1_000_000).contains(&config.compact_window) {
            return Err(format!(
                "Unknown compaction window: {}",
                config.compact_window
            ));
        }
        config.keys.retain(|_, value| !value.trim().is_empty());
        config.runs_hidden.retain(|name| !name.trim().is_empty());
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
        claude::COMPACT_WINDOW.store(config.compact_window, Ordering::Relaxed);
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
    let window = settings.config.lock().unwrap().compact_window;
    if config.compact_window != window {
        agents.refresh_all();
    }
    settings.save(config)?;
    Ok(())
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
fn context_usage(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    start: Option<bool>,
) -> Result<(), String> {
    agents.context_usage(&app, &settings.claude(), &id, start.unwrap_or(false))
}

#[tauri::command]
fn stop_task(agents: State<Agents>, id: String, task_id: String) -> Result<(), String> {
    agents.stop_task(&id, &task_id)
}

#[tauri::command]
fn mcp_request(
    agents: State<Agents>,
    id: String,
    request_id: String,
    request: serde_json::Value,
) -> Result<(), String> {
    if !request["subtype"]
        .as_str()
        .is_some_and(|s| s.starts_with("mcp_"))
    {
        return Err("Not an MCP request.".into());
    }
    agents.request(&id, &request_id, request)
}

fn scratch(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("scratch");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[tauri::command]
async fn claude_config(
    app: AppHandle,
    settings: State<'_, Settings>,
    setting: String,
) -> Result<String, String> {
    let program = settings.claude();
    let dir = scratch(&app)?;
    let output = tauri::async_runtime::spawn_blocking(move || {
        process::command(program)
            .current_dir(dir)
            .args([
                "-p",
                "--output-format",
                "json",
                "--strict-mcp-config",
                "--no-session-persistence",
                format!("/config {setting}").trim(),
            ])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let reply: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let text = reply["result"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    if reply["is_error"].as_bool().unwrap_or(true) {
        return Err(text);
    }
    Ok(text)
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn plugins_installed(settings: State<'_, Settings>) -> Result<serde_json::Value, String> {
    let program = settings.claude();
    blocking(move || plugins::installed(&program)).await
}

#[tauri::command]
async fn plugins_catalog(settings: State<'_, Settings>) -> Result<serde_json::Value, String> {
    let program = settings.claude();
    blocking(move || plugins::catalog(&program)).await
}

#[tauri::command]
async fn plugins_marketplaces(settings: State<'_, Settings>) -> Result<serde_json::Value, String> {
    let program = settings.claude();
    blocking(move || plugins::marketplaces(&program)).await
}

#[tauri::command]
async fn plugin_details(
    settings: State<'_, Settings>,
    id: String,
) -> Result<plugins::Details, String> {
    let program = settings.claude();
    blocking(move || plugins::details(&program, &id)).await
}

#[tauri::command]
fn plugin_flags(projects: Vec<PathBuf>) -> plugins::Flags {
    plugins::read_flags(sessions::claude_dir(), &projects)
}

#[tauri::command]
async fn plugin_run(
    settings: State<'_, Settings>,
    args: Vec<String>,
    dir: Option<PathBuf>,
) -> Result<String, String> {
    let program = settings.claude();
    blocking(move || plugins::run(&program, &args, dir.as_deref())).await
}

#[tauri::command]
fn agent_definition(project: PathBuf, name: String) -> Option<definitions::Definition> {
    definitions::find(&project, &name)
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
async fn suggest_title(
    app: AppHandle,
    settings: State<'_, Settings>,
    text: String,
) -> Result<String, String> {
    let program = settings.claude();
    let task: String = text.chars().take(2000).collect();
    let dir = scratch(&app)?;
    let output = tauri::async_runtime::spawn_blocking(move || {
        process::command(program)
            .current_dir(dir)
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
                "--",
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
    let output = process::command(path.trim())
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

fn artifacts_root() -> Result<&'static PathBuf, String> {
    artifacts::ROOT
        .get()
        .ok_or_else(|| "No artifacts folder.".into())
}

#[tauri::command]
fn artifacts_dir() -> Result<PathBuf, String> {
    artifacts_root().cloned()
}

#[tauri::command]
fn list_artifacts(agent: String) -> Result<Vec<artifacts::Artifact>, String> {
    artifacts::list(artifacts_root()?, &agent)
}

#[tauri::command]
fn read_artifact(agent: String, id: String) -> Result<String, String> {
    artifacts::read(artifacts_root()?, &agent, &id)
}

#[tauri::command]
fn write_artifact(
    agent: String,
    id: String,
    kind: String,
    content: String,
) -> Result<PathBuf, String> {
    artifacts::write(artifacts_root()?, &agent, &id, &kind, &content)
}

#[tauri::command]
fn set_artifact_link(agent: String, id: String, link: String) -> Result<(), String> {
    artifacts::set_link(artifacts_root()?, &agent, &id, &link)
}

#[tauri::command]
fn set_artifact_keep(agent: String, id: String, keep: Option<u32>) -> Result<(), String> {
    artifacts::set_keep(artifacts_root()?, &agent, &id, keep)
}

#[tauri::command]
fn artifact_owners() -> Result<Vec<String>, String> {
    Ok(artifacts::owners(artifacts_root()?))
}

#[tauri::command]
fn set_artifact_share(
    agent: String,
    id: String,
    destination: String,
    link: Option<String>,
) -> Result<(), String> {
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    artifacts::set_shared(
        artifacts_root()?,
        &agent,
        &id,
        &destination,
        link.as_deref(),
        at,
    )
}

#[tauri::command]
fn delete_artifact(agent: String, id: String) -> Result<(), String> {
    artifacts::remove(artifacts_root()?, &agent, &id)
}

#[tauri::command]
fn open_artifact_folder(agent: String, id: String) -> Result<(), String> {
    let dir = artifacts::agent_dir(artifacts_root()?, &agent)?.join(&id);
    if id.is_empty() || id.starts_with('.') || id.contains('/') || !dir.is_dir() {
        return Err("The artifact is gone.".into());
    }
    open_path(&dir)
}

#[tauri::command]
fn save_text_file(path: PathBuf, text: String) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| e.to_string())
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
fn existing_files(project: PathBuf, paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter(|p| projects::inside(&project, Path::new(p)).is_some_and(|f| f.is_file()))
        .collect()
}

#[tauri::command]
async fn project_files(dir: PathBuf) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || git::files(&dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn project_grep(
    project: PathBuf,
    query: String,
    case: bool,
    word: bool,
    regex: bool,
) -> Result<Vec<git::Hit>, String> {
    tauri::async_runtime::spawn_blocking(move || git::grep(&project, &query, case, word, regex))
        .await
        .map_err(|e| e.to_string())?
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
fn shell_run(
    app: AppHandle,
    terminals: State<Terminals>,
    dir: PathBuf,
    command: String,
    cols: u16,
    rows: u16,
) -> Result<u32, String> {
    let shell = std::env::var("SHELL").ok();
    let out = app.clone();
    terminals.open(
        &dir,
        shell.as_deref(),
        Some(&command),
        cols,
        rows,
        move |id, data| {
            let _ = out.emit("shell-out", serde_json::json!({ "id": id, "data": data }));
        },
        move |id| {
            let code = app.state::<Terminals>().finish(id);
            let _ = app.emit("shell-exit", serde_json::json!({ "id": id, "code": code }));
        },
    )
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
async fn branch_changes(project: PathBuf) -> Result<git::Branched, String> {
    tauri::async_runtime::spawn_blocking(move || git::branch_changes(&project))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn git_changes(project: PathBuf) -> Result<git::Changes, String> {
    tauri::async_runtime::spawn_blocking(move || git::changes(&project))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pull_request(dir: PathBuf, branch: Option<String>) -> Result<Option<forge::Pull>, String> {
    forge::pull(&dir, branch).await
}

#[tauri::command]
async fn linked_pull(url: String, projects: Vec<PathBuf>) -> Result<forge::Linked, String> {
    forge::linked(&url, projects).await
}

#[tauri::command]
async fn numbered_pull(dir: PathBuf, number: u64) -> Result<forge::Pull, String> {
    forge::numbered(&dir, number).await
}

#[tauri::command]
async fn pipeline_runs(dir: PathBuf, repository: String) -> Result<Vec<forge::Run>, String> {
    forge::runs(&dir, &repository).await
}

#[tauri::command]
async fn pull_lists(dir: PathBuf) -> Result<forge::Lists, String> {
    forge::lists(&dir).await
}

#[tauri::command]
async fn review_lines(dir: PathBuf, base: String, path: String) -> Result<git::Shown, String> {
    tauri::async_runtime::spawn_blocking(move || git::shown(&dir, &base, &path))
        .await
        .map_err(|e| e.to_string())?
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

fn reviews_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("reviews"))
}

#[tauri::command]
async fn drop_review_checkout(app: AppHandle, dir: PathBuf) -> Result<bool, String> {
    if dir.parent() != Some(reviews_dir(&app)?.as_path()) {
        return Ok(false);
    }
    tauri::async_runtime::spawn_blocking(move || git::drop_review_checkout(&dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn prune_review_checkouts(app: AppHandle, keep: Vec<PathBuf>) -> Result<usize, String> {
    let root = reviews_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(list) = std::fs::read_dir(&root) else {
            return 0;
        };
        list.flatten()
            .map(|e| e.path())
            .filter(|p| !keep.contains(p) && git::drop_review_checkout(p).unwrap_or(false))
            .count()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn restore_review_checkout(
    app: AppHandle,
    project: PathBuf,
    dir: PathBuf,
    head: String,
) -> Result<(), String> {
    if dir.parent() != Some(reviews_dir(&app)?.as_path()) {
        return Err("Not a review checkout.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        git::restore_review_checkout(&project, &dir, &head)
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
async fn pull_me(dir: PathBuf) -> Result<forge::Person, String> {
    forge::me(&dir).await
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
async fn update_pull(
    dir: PathBuf,
    number: u64,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    forge::update(&dir, number, title, description).await
}

#[tauri::command]
async fn forge_accounts() -> forge::Accounts {
    tauri::async_runtime::spawn_blocking(forge::accounts)
        .await
        .unwrap_or(forge::Accounts {
            bitbucket: None,
            bitbucket_user: false,
            github: None,
            gitlab: None,
            azure: None,
        })
}

#[tauri::command]
async fn bitbucket_sign_in(email: String, token: String) -> Result<(), String> {
    forge::sign_in(email.trim().to_string(), token.trim().to_string()).await
}

#[tauri::command]
fn forge_sign_out(account: String) -> Result<(), String> {
    forge::sign_out(&account)
}

#[tauri::command]
async fn gitlab_sign_in(site: String, token: String) -> Result<(), String> {
    gitlab::sign_in(site, token.trim().to_string()).await
}

#[tauri::command]
async fn azure_sign_in(organization: String, token: String) -> Result<(), String> {
    azure::sign_in(organization, token.trim().to_string()).await
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
        .or_else(|| std::env::home_dir().map(|h| h.join(".claude")))
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
    open_path(&file)
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
    files: Option<Vec<PathBuf>>,
) -> Result<(), String> {
    agents.register(&id, project, options, None, false)?;
    agents.set_dir(&id, dir)?;
    if let Some(name) = worktree {
        agents.new_worktree(&id, &name)?;
    }
    let files = files.unwrap_or_default();
    if text.trim().is_empty() && files.is_empty() {
        return Ok(());
    }
    let content = attach::content(&text, &files)?;
    agents.send(&app, &settings.claude(), &id, content)
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
    agents.set_dir(&id, dir.filter(|d| d.join(".git").exists()))?;
    match at {
        Some(at) => agents.fork_at(&id, &at),
        None => Ok(()),
    }
}

#[tauri::command]
fn set_agent_servers(
    agents: State<Agents>,
    on_demand: State<ondemand::OnDemand>,
    id: String,
    names: Vec<String>,
) -> Result<(), String> {
    let configs = on_demand.configs(&names);
    agents.set_servers(&id, names, configs)
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
fn rewind_agent(agents: State<Agents>, id: String, at: Option<String>) -> Result<(), String> {
    agents.rewind(&id, at.as_deref())
}

#[tauri::command]
fn system_load(cpu: State<system::Cpu>) -> Option<system::Load> {
    cpu.load()
}

struct AgentFiles(PathBuf);

#[derive(Default)]
struct FoundCache(Mutex<HashMap<PathBuf, (std::time::SystemTime, stored::Found)>>);

impl FoundCache {
    fn get(&self, path: &Path) -> Option<stored::Found> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
        if let Some((at, found)) = self.0.lock().unwrap().get(path) {
            if *at == modified {
                return Some(found.clone());
            }
        }
        let found = stored::found(&std::fs::read_to_string(path).ok()?)?;
        self.0
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), (modified, found.clone()));
        Some(found)
    }
}

fn spread<T: Send>(paths: &[PathBuf], each: impl Fn(&Path) -> Option<T> + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    let chunk = paths.len().div_ceil(threads).max(1);
    let each = &each;
    std::thread::scope(|scope| {
        let workers: Vec<_> = paths
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || part.iter().filter_map(|p| each(p)).collect::<Vec<_>>())
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap_or_default())
            .collect()
    })
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    dir.filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect()
}

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
fn extensions_list(extensions: State<extensions::Extensions>) -> extensions::Listing {
    extensions.list()
}

#[tauri::command]
fn extensions_link(extensions: State<extensions::Extensions>, path: String) -> Result<(), String> {
    extensions.link(&path)
}

#[tauri::command]
fn extensions_unlink(
    extensions: State<extensions::Extensions>,
    path: String,
) -> Result<(), String> {
    extensions.unlink(&path)
}

#[tauri::command]
fn extensions_enable(
    extensions: State<extensions::Extensions>,
    id: String,
    on: bool,
) -> Result<(), String> {
    extensions.enable(&id, on)
}

#[tauri::command]
fn extensions_approve(
    extensions: State<extensions::Extensions>,
    id: String,
    commands: Vec<String>,
) -> Result<(), String> {
    extensions.approve(&id, commands)
}

#[tauri::command]
fn extensions_set_args(args: Vec<String>) {
    *claude::EXTENSION_ARGS.lock().unwrap() = args;
}

#[tauri::command]
fn extensions_set_skills(
    extensions: State<extensions::Extensions>,
    list: Vec<extensions::SkillSet>,
) -> Vec<String> {
    let root = extensions.file.with_file_name("extensions-plugins");
    let (plugins, errors) = extensions::sync_skills(&root, &list);
    *claude::EXTENSION_PLUGINS.lock().unwrap() = plugins;
    errors
}

#[tauri::command]
async fn extensions_set_tools(
    tools: State<'_, extension_tools::Tools>,
    list: Vec<extension_tools::ExtensionTool>,
) -> Result<(), String> {
    tools.set(list).await;
    Ok(())
}

#[tauri::command]
fn extensions_tool_reply(
    tools: State<extension_tools::Tools>,
    id: String,
    ok: bool,
    text: String,
) -> Result<(), String> {
    tools.reply(&id, ok, text)
}

#[tauri::command]
async fn extension_exec(
    dir: PathBuf,
    command: String,
    id: String,
    secrets: Vec<String>,
) -> Result<(bool, String), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut env = Vec::new();
        for name in secrets {
            if let Some(value) = forge::read_secret(&wasm::secret_key(&id, &name)?)? {
                env.push((
                    format!("SQUADRON_{}", name.to_uppercase().replace('-', "_")),
                    value,
                ));
            }
        }
        extension_tools::exec(&dir, &command, &env)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_secrets(id: String, names: Vec<String>) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut set = Vec::new();
        for name in names {
            if forge::read_secret(&wasm::secret_key(&id, &name)?)?.is_some() {
                set.push(name);
            }
        }
        Ok(set)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_secret_set(
    id: String,
    name: String,
    value: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        forge::write_secret(&wasm::secret_key(&id, &name)?, value.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_wasm_sync(
    app: AppHandle,
    list: Vec<wasm::Spec>,
) -> Result<HashMap<String, String>, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<wasm::Runtime>().sync(list))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn extensions_wasm_event(
    runtime: State<wasm::Runtime>,
    id: Option<String>,
    name: String,
    agent: Option<wasm::AgentInfo>,
    data: String,
) {
    runtime.event(id.as_deref(), &name, agent, &data);
}

#[tauri::command]
fn extensions_wasm_answer(
    runtime: State<wasm::Runtime>,
    reply: String,
    value: Option<Value>,
    error: Option<String>,
) {
    runtime.answer(&reply, error.map_or(Ok(value.unwrap_or_default()), Err));
}

#[tauri::command]
fn extension_data_dir(runtime: State<wasm::Runtime>, id: String) -> Result<PathBuf, String> {
    runtime.data_dir(&id)
}

#[tauri::command]
async fn extensions_wasm_run(
    app: AppHandle,
    id: String,
    action: String,
    agent: Option<wasm::AgentInfo>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<wasm::Runtime>().run(&id, &action, agent)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_wasm_tool(
    app: AppHandle,
    id: String,
    name: String,
    input: String,
    agent: Option<wasm::AgentInfo>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<wasm::Runtime>().tool(&id, &name, &input, agent)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_catalog(app: AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let sources = app.state::<extensions::Extensions>().choices().sources;
    Ok(extensions::catalog(sources).await)
}

#[tauri::command]
async fn extensions_source_add(app: AppHandle, source: String) -> Result<(), String> {
    if !extensions::valid_source(&source) {
        return Err(format!(
            "{source} is not a GitHub repository like owner/repo."
        ));
    }
    extensions::fetch_index(&source)
        .await
        .map_err(|e| format!("{source} has no extension catalog: {e}"))?;
    app.state::<extensions::Extensions>().add_source(&source)
}

#[tauri::command]
fn extensions_source_remove(
    extensions: State<extensions::Extensions>,
    source: String,
) -> Result<(), String> {
    extensions.remove_source(&source)
}

#[tauri::command]
async fn extensions_install(
    app: AppHandle,
    source: String,
    id: String,
    archive: String,
    sha256: String,
) -> Result<(), String> {
    if !app.state::<extensions::Extensions>().known_source(&source) {
        return Err(format!("{source} is not one of your extension sources."));
    }
    let bytes = extensions::fetch_archive(&source, &archive).await?;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<extensions::Extensions>()
            .install(&source, &id, &bytes, &sha256)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn account_keyring(id: &str) -> impl Fn(&str, &str, Option<&str>) -> Result<(), String> + '_ {
    move |account, field, value| forge::write_secret(&wasm::account_key(id, account, field)?, value)
}

#[tauri::command]
async fn extensions_uninstall(app: AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<extensions::Extensions>()
            .uninstall(&id, &account_keyring(&id))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_account_save(
    app: AppHandle,
    id: String,
    account: Option<String>,
    values: std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let saved = app.state::<extensions::Extensions>().save_account(
            &id,
            account.as_deref(),
            &values,
            &account_keyring(&id),
        )?;
        app.state::<wasm::Runtime>()
            .event(Some(&id), "accounts.changed", None, "{}");
        Ok(saved)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_option_set(
    app: AppHandle,
    id: String,
    option: String,
    value: Option<Value>,
) -> Result<serde_json::Map<String, Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let values = app
            .state::<extensions::Extensions>()
            .set_option(&id, &option, value)?;
        app.state::<wasm::Runtime>().event(
            Some(&id),
            "options.changed",
            None,
            &Value::Object(values.clone()).to_string(),
        );
        Ok(values)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn extensions_account_remove(
    app: AppHandle,
    id: String,
    account: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<extensions::Extensions>().remove_account(
            &id,
            &account,
            &account_keyring(&id),
        )?;
        app.state::<wasm::Runtime>()
            .event(Some(&id), "accounts.changed", None, "{}");
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_extensions_folder(extensions: State<extensions::Extensions>) -> Result<(), String> {
    open_path(&extensions.dir)
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
async fn save_agent(files: State<'_, AgentFiles>, id: String, data: String) -> Result<(), String> {
    let path = files.path(&id)?;
    std::fs::create_dir_all(&files.0).map_err(|e| e.to_string())?;
    let data = stored::keep_entries(data, || std::fs::read_to_string(&path).ok())?;
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{n}.tmp"));
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
            if let Some(root) = artifacts::ROOT.get() {
                artifacts::forget(root, &id);
            }
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
    if let Some(root) = artifacts::ROOT.get() {
        artifacts::forget(root, &id);
    }
    Ok(())
}

#[tauri::command]
async fn list_archived(files: State<'_, AgentFiles>) -> Result<Vec<String>, String> {
    Ok(spread(&json_files(&files.0.join("archived")), |p| {
        stored::header(&std::fs::read_to_string(p).ok()?)
    }))
}

#[tauri::command]
async fn archived_entries(
    files: State<'_, AgentFiles>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    let text = std::fs::read_to_string(files.archived(&id)?).map_err(|e| e.to_string())?;
    let entries = stored::entries(&text).ok_or("The agent's history could not be read.")?;
    Ok(tauri::ipc::Response::new(entries.into_bytes()))
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
async fn account_name() -> Option<String> {
    account::read_first_name()
}

#[tauri::command]
async fn load_agents(files: State<'_, AgentFiles>) -> Result<Vec<String>, String> {
    Ok(spread(&json_files(&files.0), |p| {
        stored::header(&std::fs::read_to_string(p).ok()?)
    }))
}

#[tauri::command]
async fn load_entries(
    files: State<'_, AgentFiles>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    let text = std::fs::read_to_string(files.path(&id)?).map_err(|e| e.to_string())?;
    let entries = stored::entries(&text).ok_or("The agent's history could not be read.")?;
    Ok(tauri::ipc::Response::new(entries.into_bytes()))
}

#[tauri::command]
async fn agent_texts(
    files: State<'_, AgentFiles>,
    cache: State<'_, FoundCache>,
    ids: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let paths: Vec<PathBuf> = ids.iter().filter_map(|id| files.path(id).ok()).collect();
    Ok(spread(&paths, |p| cache.get(p))
        .into_iter()
        .map(|found| (found.id, found.text))
        .collect())
}

#[tauri::command]
async fn archived_texts(
    files: State<'_, AgentFiles>,
    cache: State<'_, FoundCache>,
) -> Result<Vec<stored::Found>, String> {
    Ok(spread(&json_files(&files.0.join("archived")), |p| {
        cache.get(p)
    }))
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
fn copy_text(window: tauri::WebviewWindow, text: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        window
            .run_on_main_thread(move || {
                gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(&text)
            })
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, text);
        Err("unsupported".into())
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

const DOCUMENTS: &[&str] = &[
    "avif", "bmp", "csv", "doc", "docx", "gif", "heic", "jpeg", "jpg", "json", "key", "log", "m4a",
    "md", "mov", "mp3", "mp4", "numbers", "odg", "odp", "ods", "odt", "ogg", "pages", "pdf", "png",
    "ppt", "pptx", "rtf", "tif", "tiff", "tsv", "txt", "wav", "webm", "webp", "xls", "xlsx",
];

fn document(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| DOCUMENTS.contains(&e.to_ascii_lowercase().as_str()))
}

#[tauri::command]
fn open_attachment(path: PathBuf) -> Result<(), String> {
    let meta = std::fs::metadata(&path).map_err(|_| "The file is gone.")?;
    #[cfg(unix)]
    let runs = {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    };
    #[cfg(not(unix))]
    let runs = false;
    if !meta.is_file() || runs || !document(&path) {
        return Err("Squadron only opens documents and images from here.".into());
    }
    open_path(&path)
}

#[tauri::command]
async fn save_capture(data: String) -> Result<PathBuf, String> {
    let dir = claude::CAPTURES.get().ok_or("No captures folder.")?.clone();
    tauri::async_runtime::spawn_blocking(move || attach::capture(&dir, &data))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn attachment_size(path: PathBuf) -> Result<u64, String> {
    std::fs::metadata(&path)
        .map(|meta| meta.len())
        .map_err(|_| "The file is gone.".into())
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

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let uri = gtk::gio::prelude::FileExt::uri(&gtk::gio::File::for_path(path));
        launch(&uri, gtk::gio::AppLaunchContext::new())
    }
    #[cfg(not(target_os = "linux"))]
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn launch(uri: &str, context: gtk::gio::AppLaunchContext) -> Result<(), String> {
    gtk::gio::prelude::AppLaunchContextExt::unsetenv(&context, "LD_LIBRARY_PATH");
    gtk::gio::AppInfo::launch_default_for_uri(uri, Some(&context)).or_else(|e| {
        // The AppImage's GLib looks for gio-launch-desktop where the build distribution keeps it.
        log::warn!("{uri} did not open through GIO ({e}), trying xdg-open");
        let mut child = process::command("xdg-open")
            .arg(uri)
            .spawn()
            .map_err(|e| e.to_string())?;
        std::thread::spawn(move || child.wait());
        Ok(())
    })
}

#[tauri::command]
async fn link_preview(url: String) -> Option<preview::Preview> {
    preview::fetch(&url).await
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
                let context = parent
                    .gtk_window()
                    .ok()
                    .and_then(|w| gtk::prelude::WidgetExt::display(&w).app_launch_context())
                    .map(gtk::prelude::Cast::upcast)
                    .unwrap_or_else(gtk::gio::AppLaunchContext::new);
                if let Err(e) = launch(&url, context) {
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
    agents.archive(&id);
    keys::KEYS.revoke(&id);
}

#[tauri::command]
async fn close_agents(agents: State<'_, Agents>) -> Result<HashMap<String, Vec<Value>>, ()> {
    Ok(agents.close_all())
}

#[tauri::command]
fn sleep_agent(agents: State<Agents>, id: String) -> bool {
    agents.sleep(&id)
}

#[tauri::command]
fn resume_agent(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    content: Value,
) -> Result<(), String> {
    agents.send(&app, &settings.claude(), &id, content)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn login_path() {
    use std::io::Read;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/sh"
        }
        .into()
    });
    let Ok(mut child) = process::command(shell)
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
    for leaked in ["GIO_LAUNCHED_DESKTOP_FILE", "GIO_LAUNCHED_DESKTOP_FILE_PID"] {
        // SAFETY: runs first, before Squadron starts any thread.
        unsafe { std::env::remove_var(leaked) };
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
        && std::path::Path::new("/sys/module/nvidia").exists()
    {
        // SAFETY: runs first, before Squadron starts any thread.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
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
        .register_asynchronous_uri_scheme_protocol("artifact", |_, request, responder| {
            let path = request.uri().path().to_string();
            std::thread::spawn(move || match artifacts::ROOT.get() {
                Some(root) => responder.respond(artifacts::serve(root, &path)),
                None => responder.respond(
                    http::Response::builder()
                        .status(503)
                        .body(Vec::new())
                        .unwrap(),
                ),
            });
        })
        .register_asynchronous_uri_scheme_protocol("extension", |ctx, request, responder| {
            let (app, path) = (ctx.app_handle().clone(), request.uri().path().to_string());
            std::thread::spawn(move || match app.try_state::<extensions::Extensions>() {
                Some(extensions) => responder.respond(extensions.serve(&path)),
                None => responder.respond(
                    http::Response::builder()
                        .status(503)
                        .body(Vec::new())
                        .unwrap(),
                ),
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build())
        .plugin(links::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Agents::default())
        .manage(Terminals::default())
        .manage(sudo::Sudo::default())
        .manage(system::Cpu::default())
        .invoke_handler(tauri::generate_handler![
            agent_definition,
            plugins_installed,
            plugins_catalog,
            plugins_marketplaces,
            plugin_details,
            plugin_flags,
            plugin_run,
            system_load,
            stop_turn,
            open_link,
            link_preview,
            git_branch,
            set_workdir,
            agent_ids,
            load_agent,
            list_projects,
            list_groups,
            add_group,
            edit_group,
            remove_group,
            move_projects,
            inspect_folder,
            add_project,
            scratch_project,
            remove_project,
            move_project,
            edit_project,
            order_projects,
            archive_agent_file,
            restore_archived,
            delete_archived,
            list_archived,
            ondemand::mcp_servers,
            sudo::sudo_needs_password,
            sudo::sudo_run,
            sudo::sudo_stop,
            ssh::ssh_key_state,
            ssh::ssh_add_key,
            ondemand::mcp_on_demand,
            ondemand::mcp_everywhere,
            ondemand::mcp_describe,
            set_agent_servers,
            archived_entries,
            add_rule,
            answer_permission,
            pilot_reply,
            set_agent_options,
            remote_control,
            context_usage,
            claude::squadron_prompt_size,
            stop_task,
            mcp_request,
            claude_config,
            cancel_permissions,
            answer_askpass,
            read_project_file,
            write_project_file,
            save_text_file,
            artifacts_dir,
            list_artifacts,
            read_artifact,
            write_artifact,
            set_artifact_link,
            set_artifact_share,
            set_artifact_keep,
            artifact_owners,
            extension_data_dir,
            delete_artifact,
            open_artifact_folder,
            list_dir,
            project_files,
            existing_files,
            project_grep,
            git_file,
            term_open,
            shell_run,
            term_write,
            term_resize,
            term_close,
            open_in_editor,
            detect_editors,
            open_with,
            git_changes,
            branch_changes,
            git_patch,
            pull_request,
            pull_lists,
            pipeline_runs,
            linked_pull,
            numbered_pull,
            review_checkout,
            drop_review_checkout,
            prune_review_checkouts,
            restore_review_checkout,
            changed_files,
            review_lines,
            pull_changes,
            pull_remarks,
            pull_me,
            pull_reply,
            post_remarks,
            merge_pull,
            update_pull,
            forge_accounts,
            bitbucket_sign_in,
            forge_sign_out,
            gitlab_sign_in,
            azure_sign_in,
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
            rewind_agent,
            save_agent,
            delete_agent_file,
            load_agents,
            account_name,
            load_entries,
            agent_texts,
            archived_texts,
            get_layout,
            set_layout,
            extensions_list,
            extensions_link,
            extensions_unlink,
            extensions_enable,
            extensions_approve,
            extensions_set_tools,
            extensions_tool_reply,
            extension_exec,
            extensions_secrets,
            extensions_secret_set,
            extensions_option_set,
            extensions_account_save,
            extensions_account_remove,
            extensions_wasm_sync,
            extensions_wasm_event,
            extensions_wasm_answer,
            extensions_set_args,
            extensions_set_skills,
            extensions_wasm_run,
            extensions_wasm_tool,
            open_extensions_folder,
            extensions_catalog,
            extensions_source_add,
            extensions_source_remove,
            extensions_install,
            extensions_uninstall,
            send_message,
            save_attachment,
            attachment_preview,
            attachment_size,
            save_capture,
            open_attachment,
            paste_image,
            copy_text,
            project_sessions,
            read_session,
            interrupt_agent,
            reload_agent,
            archive_agent,
            close_agents,
            sleep_agent,
            resume_agent
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
                    for name in ["projects.json", "groups.json", "settings.json", "layout.json"] {
                        let _ = std::fs::copy(release.join(name), config.join(name));
                    }
                }
            }
            restore_place(app, &config);
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            let file = config.join("projects.json");
            let mut list = projects::load(&file);
            if projects::paint(&mut list) {
                let _ = projects::save(&file, &list);
            }
            let groups_file = config.join("groups.json");
            app.manage(Projects {
                list: Mutex::new(list),
                file,
                groups: Mutex::new(projects::load_groups(&groups_file)),
                groups_file,
            });
            app.manage(Settings::load(config.join("settings.json")));
            app.manage(AgentFiles(config.join("agents")));
            app.manage(ondemand::OnDemand::load(config.join("mcp-on-demand.json")));
            app.manage(FoundCache::default());
            let root = app.path().app_data_dir()?.join("artifacts");
            let _ = std::fs::create_dir_all(&root);
            let ids = |dir: PathBuf| {
                std::fs::read_dir(dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|f| f.file_name().to_str()?.strip_suffix(".json").map(String::from))
                    .collect::<Vec<_>>()
            };
            let agents = config.join("agents");
            let keep = ids(agents.clone()).into_iter().chain(ids(agents.join("archived"))).collect();
            artifacts::sweep(&root, &keep);
            let _ = artifacts::ROOT.set(root.clone());
            let handle = app.handle().clone();
            artifacts::watch(root, move |agent| {
                let _ = handle.emit("artifacts-changed", agent);
            });
            let captures = app.path().app_data_dir()?.join("captures");
            let _ = std::fs::create_dir_all(&captures);
            let _ = claude::CAPTURES.set(captures.clone());
            std::thread::spawn(move || {
                loop {
                    attach::expire(&captures, std::time::Duration::from_secs(7 * 24 * 3600));
                    std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
                }
            });
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
            let extensions_dir = config.join("extensions");
            std::fs::create_dir_all(&extensions_dir)?;
            let handle = app.handle().clone();
            extensions::Extensions { dir: extensions_dir.clone(), file: config.join("extensions.json") }.watch(move || {
                let _ = handle.emit_to("main", "extensions-changed", ());
            });
            app.manage(extensions::Extensions { dir: extensions_dir, file: config.join("extensions.json") });
            let handle = app.handle().clone();
            let (claude, accounts, options) = (app.handle().clone(), app.handle().clone(), app.handle().clone());
            app.manage(
                wasm::Runtime::new(config.join("extensions-state"), move |message| {
                    let _ = handle.emit_to("main", "extensions-host", message);
                })?
                .with_hooks(wasm::Hooks {
                    claude: std::sync::Arc::new(move || claude.state::<Settings>().claude()),
                    accounts: std::sync::Arc::new(move |id| accounts.state::<extensions::Extensions>().accounts(id)),
                    options: std::sync::Arc::new(move |id| Value::Object(options.state::<extensions::Extensions>().options(id))),
                }),
            );
            let handle = app.handle().clone();
            let gate = Gate::new(move |ask| {
                let _ = handle.emit("permission-request", ask);
            });
            let handle = app.handle().clone();
            let pilot = Pilot::new(move |call| {
                let _ = handle.emit_to("main", "pilot-request", call);
            });
            let handle = app.handle().clone();
            let extension_tools = extension_tools::Tools::new(move |call| {
                let _ = handle.emit_to("main", "extensions-tool", call);
            });
            let handle = app.handle().clone();
            let askpass = Askpass::new(move |question| {
                let _ = handle.emit("askpass-request", question);
            });
            let port = tauri::async_runtime::block_on(permissions::serve_with_pilot(
                gate.clone(),
                pilot.clone(),
                extension_tools.clone(),
                askpass.clone().router(),
            ))?;
            let helpers = app.path().app_data_dir()?.join("bin");
            ssh::start_own_agent(&helpers);
            match std::env::current_exe().and_then(|exe| askpass::install(&helpers, &exe)) {
                Ok(script) => {
                    let _ = askpass::HELPER.set((script, port));
                }
                Err(e) => log::warn!("password helper not installed: {e}"),
            }
            app.manage(askpass);
            let agents = app.state::<Agents>();
            agents.endpoint(port, app.path().app_data_dir()?.join("mcp"));
            agents.set_pilot(app.state::<Settings>().config.lock().unwrap().pilot);
            app.manage(gate);
            app.manage(pilot);
            app.manage(extension_tools);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
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
