pub mod claude;

use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

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
        .manage(Agent::default())
        .invoke_handler(tauri::generate_handler![send, interrupt])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
