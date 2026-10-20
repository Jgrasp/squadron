pub mod agents;
pub mod claude;
pub mod projects;

use agents::Agents;
use projects::{Folder, Project};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, RunEvent, State};

struct Projects {
    file: PathBuf,
    list: Mutex<Vec<Project>>,
}

impl Projects {
    fn views(list: &[Project]) -> Vec<Folder> {
        list.iter()
            .map(|p| Folder::of(p.name.clone(), &p.path))
            .collect()
    }

    fn update(
        &self,
        change: impl FnOnce(&mut Vec<Project>) -> Result<(), String>,
    ) -> Result<Vec<Folder>, String> {
        let mut list = self.list.lock().unwrap();
        let mut next = list.clone();
        change(&mut next)?;
        projects::save(&self.file, &next).map_err(|e| format!("Sauvegarde impossible : {e}"))?;
        *list = next;
        Ok(Self::views(&list))
    }
}

fn check_new_folder(list: &[Project], path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("Ce dossier n'existe pas ou n'est pas accessible.".into());
    }
    match list.iter().find(|p| projects::same_folder(&p.path, path)) {
        Some(p) => Err(format!("Ce dossier est déjà le projet {}.", p.name)),
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
        folder: Folder::of(name, &path),
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
        return Err("Donne un nom au projet.".into());
    }
    projects.update(|list| {
        check_new_folder(list, &path)?;
        list.push(Project { name, path });
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
    claude: Mutex<String>,
}

#[derive(Serialize, serde::Deserialize)]
struct SettingsFile {
    claude: String,
}

impl Settings {
    fn load(file: PathBuf) -> Self {
        let claude = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str::<SettingsFile>(&text).ok())
            .map_or_else(|| "claude".to_string(), |s| s.claude);
        Self {
            file,
            claude: Mutex::new(claude),
        }
    }

    fn claude(&self) -> String {
        self.claude.lock().unwrap().clone()
    }
}

#[tauri::command]
fn claude_path(settings: State<Settings>) -> String {
    settings.claude()
}

#[tauri::command]
fn set_claude_path(settings: State<Settings>, path: String) -> Result<(), String> {
    let path = if path.trim().is_empty() {
        "claude".to_string()
    } else {
        path.trim().to_string()
    };
    if let Some(dir) = settings.file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&SettingsFile {
        claude: path.clone(),
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(&settings.file, text).map_err(|e| e.to_string())?;
    *settings.claude.lock().unwrap() = path;
    Ok(())
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
    agents.create(&app, &settings.claude(), id, project, options, &text)
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
        .manage(Agents::default())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            inspect_folder,
            add_project,
            remove_project,
            move_project,
            claude_path,
            set_claude_path,
            create_agent,
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
