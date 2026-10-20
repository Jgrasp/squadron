pub mod claude;
pub mod projects;

use projects::{Folder, Project};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

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

#[derive(Default)]
struct Agent(Mutex<Option<claude::Session>>);

#[tauri::command]
fn send(app: AppHandle, agent: State<Agent>, dir: String, text: String) -> Result<(), String> {
    let mut session = agent.0.lock().unwrap();
    if session.is_none() {
        let started = claude::Session::start(claude::command(Path::new(&dir)), move |line, _| {
            let _ = app.emit("claude", line);
        });
        *session = Some(started.map_err(|e| e.to_string())?);
    }
    let sent = session.as_mut().unwrap().send(&text);
    if sent.is_err() {
        *session = None;
    }
    sent.map_err(|e| e.to_string())
}

#[tauri::command]
fn interrupt(agent: State<Agent>) -> Result<(), String> {
    match agent.0.lock().unwrap().take() {
        Some(session) => session.interrupt().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Agent::default())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            inspect_folder,
            add_project,
            remove_project,
            move_project,
            send,
            interrupt
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let file = app.path().app_config_dir()?.join("projects.json");
            app.manage(Projects {
                list: Mutex::new(projects::load(&file)),
                file,
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
