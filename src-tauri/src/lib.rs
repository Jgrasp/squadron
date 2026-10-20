pub mod agents;
pub mod claude;
pub mod permissions;
pub mod projects;

use agents::Agents;
use permissions::Gate;
use projects::{Folder, Project};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

struct Projects {
    file: PathBuf,
    list: Mutex<Vec<Project>>,
}

impl Projects {
    fn views(list: &[Project]) -> Vec<Folder> {
        list.iter()
            .map(|p| Folder::of(p.name.clone(), &p.path, p.rules.clone()))
            .collect()
    }

    fn update(
        &self,
        change: impl FnOnce(&mut Vec<Project>) -> Result<(), String>,
    ) -> Result<Vec<Folder>, String> {
        let mut list = self.list.lock().unwrap();
        let mut next = list.clone();
        change(&mut next)?;
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
}

#[tauri::command]
fn list_projects(projects: State<Projects>) -> Vec<Folder> {
    Projects::views(&projects.list.lock().unwrap())
}

#[tauri::command]
fn inspect_folder(projects: State<Projects>, path: PathBuf) -> Inspection {
    let list = projects.list.lock().unwrap();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Inspection {
        folder: Folder::of(name, &path, Vec::new()),
        taken_by: list
            .iter()
            .find(|p| projects::same_folder(&p.path, &path))
            .map(|p| p.name.clone()),
    }
}

#[tauri::command]
fn add_project(
    projects: State<Projects>,
    path: PathBuf,
    name: String,
) -> Result<Vec<Folder>, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Give the project a name.".into());
    }
    projects.update(|list| {
        check_new_folder(list, &path)?;
        list.push(Project {
            name,
            path,
            rules: Vec::new(),
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
fn set_settings(settings: State<Settings>, config: Config) -> Result<(), String> {
    settings.save(config)
}

#[tauri::command]
fn set_claude_path(settings: State<Settings>, path: String) -> Result<(), String> {
    let mut config = settings.config.lock().unwrap().clone();
    config.claude = path;
    settings.save(config)
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
    match std::fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn open_in_editor(project: PathBuf, path: PathBuf) -> Result<(), String> {
    let file = projects::inside(&project, &path).ok_or("File outside the project.")?;
    tauri_plugin_opener::open_path(file, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_agent(
    app: AppHandle,
    agents: State<Agents>,
    settings: State<Settings>,
    id: String,
    project: PathBuf,
    options: claude::Options,
    text: String,
) -> Result<(), String> {
    agents.register(&id, project, options, None, false)?;
    agents.send(&app, &settings.claude(), &id, &text)
}

#[tauri::command]
fn restore_agent(
    agents: State<Agents>,
    id: String,
    project: PathBuf,
    options: claude::Options,
    session_id: Option<String>,
    fork: bool,
) -> Result<(), String> {
    agents.register(&id, project, options, session_id, fork)
}

#[tauri::command]
fn fork_agent(
    agents: State<Agents>,
    id: String,
    from: String,
    project: PathBuf,
    options: claude::Options,
) -> Result<(), String> {
    let session = agents
        .session_id(&from)
        .ok_or("This agent has no session to fork yet.")?;
    agents.register(&id, project, options, Some(session), true)
}

struct Usage {
    file: PathBuf,
    days: Mutex<std::collections::BTreeMap<String, f64>>,
}

fn check_day(day: &str) -> Result<(), String> {
    let ok = day.len() == 10
        && day.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    if ok {
        Ok(())
    } else {
        Err("Invalid day.".into())
    }
}

#[tauri::command]
fn add_cost(usage: State<Usage>, day: String, usd: f64) -> Result<f64, String> {
    check_day(&day)?;
    if !usd.is_finite() || usd < 0.0 {
        return Err("Invalid cost.".into());
    }
    let mut days = usage.days.lock().unwrap();
    let total = *days.entry(day).and_modify(|t| *t += usd).or_insert(usd);
    let text = serde_json::to_string_pretty(&*days).map_err(|e| e.to_string())?;
    if let Some(dir) = usage.file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&usage.file, text).map_err(|e| e.to_string())?;
    Ok(total)
}

#[tauri::command]
fn day_cost(usage: State<Usage>, day: String) -> f64 {
    usage.days.lock().unwrap().get(&day).copied().unwrap_or(0.0)
}

struct AgentFiles(PathBuf);

impl AgentFiles {
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Invalid agent id.".into());
        }
        Ok(self.0.join(format!("{id}.json")))
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
fn delete_agent_file(files: State<AgentFiles>, id: String) -> Result<(), String> {
    match std::fs::remove_file(files.path(&id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
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
) -> Result<(), String> {
    agents.send(&app, &settings.claude(), &id, &text)
}

#[tauri::command]
fn interrupt_agent(agents: State<Agents>, id: String) -> Result<(), String> {
    agents.interrupt(&id)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Agents::default())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            inspect_folder,
            add_project,
            remove_project,
            move_project,
            add_rule,
            answer_permission,
            cancel_permissions,
            read_project_file,
            open_in_editor,
            get_settings,
            set_settings,
            set_claude_path,
            claude_version,
            remove_rule,
            create_agent,
            restore_agent,
            fork_agent,
            save_agent,
            delete_agent_file,
            load_agents,
            add_cost,
            day_cost,
            send_message,
            interrupt_agent,
            reload_agent,
            archive_agent
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let config = app.path().app_config_dir()?;
            let file = config.join("projects.json");
            app.manage(Projects {
                list: Mutex::new(projects::load(&file)),
                file,
            });
            app.manage(Settings::load(config.join("settings.json")));
            app.manage(AgentFiles(config.join("agents")));
            let file = config.join("usage.json");
            let days = std::fs::read_to_string(&file)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default();
            app.manage(Usage {
                file,
                days: Mutex::new(days),
            });
            let handle = app.handle().clone();
            let token = uuid::Uuid::new_v4().simple().to_string();
            let gate = Gate::new(token.clone(), move |ask| {
                let _ = handle.emit("permission-request", ask);
            });
            let port = tauri::async_runtime::block_on(gate.clone().serve())?;
            app.state::<Agents>().endpoint(port, token);
            app.manage(gate);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Agents>().stop_all();
            }
        });
}
