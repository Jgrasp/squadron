use crate::claude::{self, Event, Options, Session};
use serde::Serialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const NOT_FOUND: &str = "claude-introuvable";

struct Agent {
    project: PathBuf,
    options: Options,
    session: Option<Session>,
    session_id: Arc<Mutex<Option<String>>>,
    run: Arc<AtomicU32>,
}

#[derive(Default)]
pub struct Agents(Mutex<HashMap<String, Agent>>);

#[derive(Clone, Serialize)]
struct Line<'a> {
    id: &'a str,
    line: &'a str,
}

#[derive(Clone, Serialize)]
struct Exit<'a> {
    id: &'a str,
    stderr: &'a str,
}

impl Agents {
    pub fn create(
        &self,
        app: &AppHandle,
        program: &str,
        id: String,
        project: PathBuf,
        options: Options,
        text: &str,
    ) -> Result<(), String> {
        options.check()?;
        let mut list = self.0.lock().unwrap();
        let agent = list.entry(id.clone()).or_insert(Agent {
            project,
            options,
            session: None,
            session_id: Default::default(),
            run: Default::default(),
        });
        send(app, program, &id, agent, text)
    }

    pub fn send(&self, app: &AppHandle, program: &str, id: &str, text: &str) -> Result<(), String> {
        let mut list = self.0.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Agent inconnu.")?;
        send(app, program, id, agent, text)
    }

    pub fn interrupt(&self, id: &str) -> Result<(), String> {
        let list = self.0.lock().unwrap();
        match list.get(id).and_then(|a| a.session.as_ref()) {
            Some(session) => session.interrupt().map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn reload(&self, app: &AppHandle, program: &str, id: &str) -> Result<(), String> {
        let mut list = self.0.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Agent inconnu.")?;
        if let Some(session) = agent.session.take() {
            let _ = session.stop();
        }
        spawn(app, program, id, agent)
    }

    pub fn archive(&self, id: &str) {
        if let Some(agent) = self.0.lock().unwrap().remove(id) {
            stop(agent);
        }
    }

    pub fn stop_all(&self) {
        self.0
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_, agent)| stop(agent));
    }
}

fn stop(agent: Agent) {
    agent.run.store(u32::MAX, Ordering::SeqCst);
    if let Some(session) = agent.session {
        let _ = session.stop();
    }
}

fn send(
    app: &AppHandle,
    program: &str,
    id: &str,
    agent: &mut Agent,
    text: &str,
) -> Result<(), String> {
    if !agent.session.as_ref().is_some_and(Session::alive) {
        spawn(app, program, id, agent)?;
    }
    let sent = agent.session.as_mut().unwrap().send(text);
    if sent.is_err() {
        agent.session = None;
    }
    sent.map_err(|e| format!("Message non transmis à Claude Code : {e}"))
}

fn spawn(app: &AppHandle, program: &str, id: &str, agent: &mut Agent) -> Result<(), String> {
    if !agent.project.is_dir() {
        return Err("Le dossier du projet est introuvable.".into());
    }
    let run = agent.run.fetch_add(1, Ordering::SeqCst) + 1;
    let resume = agent.session_id.lock().unwrap().clone();
    let cmd = claude::command(program, &agent.project, &agent.options, resume.as_deref());
    let (lines_app, exit_app) = (app.clone(), app.clone());
    let (lines_id, exit_id) = (id.to_string(), id.to_string());
    let (lines_run, exit_run) = (agent.run.clone(), agent.run.clone());
    let session_id = agent.session_id.clone();
    let session = Session::start(
        cmd,
        move |line, event| {
            if lines_run.load(Ordering::SeqCst) != run {
                return;
            }
            if let Event::System {
                session_id: Some(s),
                ..
            } = &event
            {
                *session_id.lock().unwrap() = Some(s.clone());
            }
            let _ = lines_app.emit(
                "agent-line",
                Line {
                    id: &lines_id,
                    line,
                },
            );
        },
        move |stderr| {
            if exit_run.load(Ordering::SeqCst) == run {
                let _ = exit_app.emit(
                    "agent-exit",
                    Exit {
                        id: &exit_id,
                        stderr: &stderr,
                    },
                );
            }
        },
    )
    .map_err(|e| match e.kind() {
        ErrorKind::NotFound => NOT_FOUND.to_string(),
        _ => format!("Lancement de Claude Code impossible : {e}"),
    })?;
    agent.session = Some(session);
    Ok(())
}
