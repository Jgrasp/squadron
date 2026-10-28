use crate::claude::{self, Event, Options, Session};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
    at: Option<String>,
    worktree: Option<String>,
    dir: Option<PathBuf>,
    stale: bool,
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

    pub fn refresh_all(&self) {
        for agent in self.list.lock().unwrap().values_mut() {
            agent.stale |= agent.session.as_ref().is_some_and(Session::alive);
        }
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
                at: None,
                worktree: None,
                dir: None,
                stale: false,
            });
        Ok(())
    }

    pub fn set_dir(&self, id: &str, dir: Option<PathBuf>) -> Result<(), String> {
        if let Some(dir) = &dir
            && (!dir.is_absolute() || !dir.join(".git").exists())
        {
            return Err(format!("{} is not a git checkout.", dir.display()));
        }
        let mut list = self.list.lock().unwrap();
        list.get_mut(id).ok_or("Unknown agent.")?.dir = dir;
        Ok(())
    }

    pub fn new_worktree(&self, id: &str, name: &str) -> Result<(), String> {
        let valid = !name.is_empty()
            && name.len() <= 64
            && name.starts_with(|c: char| c.is_ascii_alphanumeric())
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
        if !valid {
            return Err("Invalid worktree name: use letters, digits, - _ and . only.".into());
        }
        let mut list = self.list.lock().unwrap();
        list.get_mut(id).ok_or("Unknown agent.")?.worktree = Some(name.into());
        Ok(())
    }

    pub fn fork_at(&self, id: &str, at: &str) -> Result<(), String> {
        if at.len() != 36 || !at.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err("Invalid message id.".into());
        }
        let mut list = self.list.lock().unwrap();
        list.get_mut(id).ok_or("Unknown agent.")?.at = Some(at.into());
        Ok(())
    }

    pub fn session_id(&self, id: &str) -> Option<String> {
        let list = self.list.lock().unwrap();
        list.get(id)?.session_id.lock().unwrap().clone()
    }

    pub fn send(
        &self,
        app: &AppHandle,
        program: &str,
        id: &str,
        content: Value,
    ) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        send(app, program, id, agent, content)
    }

    pub fn set_options(&self, id: &str, options: Options) -> Result<(), String> {
        options.check()?;
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        if let Some(session) = agent.session.as_mut().filter(|s| s.alive()) {
            if options.mode != agent.options.mode {
                session
                    .control(
                        serde_json::json!({"subtype": "set_permission_mode", "mode": options.mode}),
                    )
                    .map_err(|e| e.to_string())?;
            }
            if options.model != agent.options.model {
                session
                    .control(serde_json::json!({"subtype": "set_model", "model": options.model}))
                    .map_err(|e| e.to_string())?;
            }
            agent.stale |= options.effort != agent.options.effort;
        }
        agent.options = options;
        Ok(())
    }

    pub fn remote(
        &self,
        app: &AppHandle,
        program: &str,
        id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        if !agent.session.as_ref().is_some_and(Session::alive) {
            if !enabled {
                return Ok(());
            }
            spawn(app, program, id, agent)?;
        }
        agent
            .session
            .as_mut()
            .unwrap()
            .control(serde_json::json!({"subtype": "remote_control", "enabled": enabled}))
            .map_err(|e| e.to_string())
    }

    pub fn interrupt(&self, id: &str) -> Result<(), String> {
        let list = self.list.lock().unwrap();
        match list.get(id).and_then(|a| a.session.as_ref()) {
            Some(session) => session.interrupt().map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn context_usage(&self, id: &str) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        match list
            .get_mut(id)
            .and_then(|a| a.session.as_mut())
            .filter(|s| s.alive())
        {
            Some(session) => session
                .control(serde_json::json!({"subtype": "get_context_usage"}))
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn stop_turn(&self, id: &str) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        match list.get_mut(id).and_then(|a| a.session.as_mut()) {
            Some(session) => session
                .control(serde_json::json!({"subtype": "interrupt"}))
                .map_err(|e| e.to_string()),
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

    /// Stops every Claude Code process before the app quits, and returns per agent the messages it had not read yet.
    pub fn close_all(&self) -> HashMap<String, Vec<Value>> {
        let closing: Vec<(String, Session)> = self
            .list
            .lock()
            .unwrap()
            .iter_mut()
            .filter_map(|(id, agent)| {
                agent.run.fetch_add(1, Ordering::SeqCst);
                let session = agent.session.take()?;
                let _ = session.stop();
                Some((id.clone(), session))
            })
            .collect();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline && closing.iter().any(|(_, s)| s.alive()) {
            std::thread::sleep(Duration::from_millis(50));
        }
        closing
            .into_iter()
            .filter_map(|(id, session)| {
                let unread = session.unread();
                (!unread.is_empty()).then_some((id, unread))
            })
            .collect()
    }

    /// Stops the process of an agent that has nothing left to do; its next message starts it again with `--resume`.
    pub fn sleep(&self, id: &str) -> bool {
        let mut list = self.list.lock().unwrap();
        let Some(agent) = list.get_mut(id) else {
            return false;
        };
        if !agent
            .session
            .as_ref()
            .is_some_and(|s| s.alive() && !s.busy())
        {
            return false;
        }
        let _ = agent.session.take().unwrap().stop();
        true
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
    content: Value,
) -> Result<(), String> {
    if agent.stale && !agent.session.as_ref().is_some_and(Session::busy) {
        if let Some(session) = agent.session.take() {
            let _ = session.stop();
        }
        agent.stale = false;
    }
    if !agent.session.as_ref().is_some_and(Session::alive) {
        spawn(app, program, id, agent)?;
    }
    let sent = agent.session.as_mut().unwrap().send_content(content);
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
    let dir = agent
        .dir
        .as_ref()
        .filter(|d| d.is_dir())
        .unwrap_or(&agent.project);
    let mut cmd = claude::command(
        program,
        dir,
        &agent.options,
        resume.as_deref(),
        agent.fork,
        agent.at.as_deref(),
        agent.mcp.as_deref(),
    );
    crate::askpass::env(&mut cmd, id);
    if let Some(name) = agent.worktree.take() {
        cmd.args(["--worktree", &name]);
    }
    agent.fork = false;
    agent.at = None;
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
    let mut session = session;
    let _ = session.control(serde_json::json!({"subtype": "initialize"}));
    agent.session = Some(session);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_a_message_uuid_as_fork_point() {
        let agents = Agents::default();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        agents
            .register(
                "a1",
                PathBuf::from("/tmp"),
                options,
                Some("s1".into()),
                true,
            )
            .unwrap();
        assert!(
            agents
                .fork_at("a1", "2654721f-e7e6-4b3c-a2a7-25979e1d1873")
                .is_ok()
        );
        assert!(
            agents
                .fork_at("a1", "--dangerously-skip-permissions-0000")
                .is_err()
        );
        assert!(
            agents
                .fork_at("missing", "2654721f-e7e6-4b3c-a2a7-25979e1d1873")
                .is_err()
        );
        assert!(agents.new_worktree("a1", "bat-809-t70").is_ok());
        assert!(
            agents
                .set_dir("a1", Some(PathBuf::from("relative/wt")))
                .is_err()
        );
        assert!(agents.set_dir("a1", Some(PathBuf::from("/tmp"))).is_err());
        assert!(agents.set_dir("a1", None).is_ok());
        assert!(agents.new_worktree("a1", "--tmux").is_err());
        assert!(agents.new_worktree("a1", "a b").is_err());
    }
}
