use crate::claude::{self, Event, Options, Session};
use serde::Serialize;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const NOT_FOUND: &str = "claude-not-found";

struct Agent {
    project: PathBuf,
    options: Options,
    session: Option<Session>,
    session_id: Arc<Mutex<Option<String>>>,
    run: Arc<AtomicU32>,
    mcp: Option<String>,
    fork: bool,
}

#[derive(Default)]
pub struct Agents {
    list: Mutex<HashMap<String, Agent>>,
    endpoint: Mutex<Option<(u16, String)>>,
    pilot: AtomicBool,
}

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
    pub fn endpoint(&self, port: u16, token: String) {
        *self.endpoint.lock().unwrap() = Some((port, token));
    }

    pub fn set_pilot(&self, on: bool) {
        self.pilot.store(on, Ordering::Relaxed);
    }

    fn mcp(&self, id: &str) -> Option<String> {
        let endpoint = self.endpoint.lock().unwrap();
        let (port, token) = endpoint.as_ref()?;
        Some(crate::permissions::mcp_config(
            *port,
            token,
            id,
            self.pilot.load(Ordering::Relaxed),
        ))
    }

    pub fn register(
        &self,
        id: &str,
        project: PathBuf,
        options: Options,
        session_id: Option<String>,
        fork: bool,
    ) -> Result<(), String> {
        options.check()?;
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Invalid agent id.".into());
        }
        let mcp = self.mcp(id);
        self.list
            .lock()
            .unwrap()
            .entry(id.to_string())
            .or_insert(Agent {
                project,
                options,
                session: None,
                session_id: Arc::new(Mutex::new(session_id)),
                run: Default::default(),
                mcp,
                fork,
            });
        Ok(())
    }

    pub fn session_id(&self, id: &str) -> Option<String> {
        let list = self.list.lock().unwrap();
        list.get(id)?.session_id.lock().unwrap().clone()
    }

    pub fn send(&self, app: &AppHandle, program: &str, id: &str, text: &str) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        send(app, program, id, agent, text)
    }

    pub fn interrupt(&self, id: &str) -> Result<(), String> {
        let list = self.list.lock().unwrap();
        match list.get(id).and_then(|a| a.session.as_ref()) {
            Some(session) => session.interrupt().map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn reload(&self, app: &AppHandle, program: &str, id: &str) -> Result<(), String> {
        let mcp = self.mcp(id);
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        agent.mcp = mcp;
        if let Some(session) = agent.session.take() {
            let _ = session.stop();
        }
        spawn(app, program, id, agent)
    }

    pub fn archive(&self, id: &str) {
        if let Some(agent) = self.list.lock().unwrap().remove(id) {
            stop(agent);
        }
    }

    pub fn stop_all(&self) {
        self.list
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
    sent.map_err(|e| format!("Message not delivered to Claude Code: {e}"))
}

fn spawn(app: &AppHandle, program: &str, id: &str, agent: &mut Agent) -> Result<(), String> {
    if !agent.project.is_dir() {
        return Err("The project folder cannot be found.".into());
    }
    let run = agent.run.fetch_add(1, Ordering::SeqCst) + 1;
    let resume = agent.session_id.lock().unwrap().clone();
    let cmd = claude::command(
        program,
        &agent.project,
        &agent.options,
        resume.as_deref(),
        agent.fork,
        agent.mcp.as_deref(),
    );
    agent.fork = false;
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
        _ => format!("Claude Code could not start: {e}"),
    })?;
    agent.session = Some(session);
    Ok(())
}
