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
    fork: bool,
    at: Option<String>,
    worktree: Option<String>,
    dir: Option<PathBuf>,
    stale: bool,
}

#[derive(Default)]
pub struct Agents {
    list: Mutex<HashMap<String, Agent>>,
    endpoint: Mutex<Option<(u16, String, PathBuf)>>,
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

fn write_private(path: &std::path::Path, text: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    std::io::Write::write_all(&mut open.open(path)?, text.as_bytes())?;
    Ok(path.to_path_buf())
}

impl Agents {
    pub fn endpoint(&self, port: u16, token: String, dir: PathBuf) {
        *self.endpoint.lock().unwrap() = Some((port, token, dir));
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
        let (port, token, dir) = endpoint.as_ref()?;
        let config =
            crate::permissions::mcp_config(*port, token, id, self.pilot.load(Ordering::Relaxed));
        match write_private(&dir.join(format!("{id}.json")), &config) {
            Ok(path) => Some(path.to_string_lossy().into_owned()),
            Err(e) => {
                log::warn!("MCP config file not written, passing it inline: {e}");
                Some(config)
            }
        }
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
        if session_id.as_deref().is_some_and(|s| {
            s.is_empty() || s.len() > 64 || !s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        }) {
            return Err("Invalid session id.".into());
        }
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

    pub fn rewind(&self, id: &str, at: Option<&str>) -> Result<(), String> {
        if let Some(at) = at {
            self.fork_at(id, at)?;
        }
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        if let Some(session) = agent.session.take() {
            let _ = session.stop();
        }
        agent.fork = at.is_some();
        if at.is_none() {
            agent.at = None;
            *agent.session_id.lock().unwrap() = None;
        }
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
        send(self, app, program, id, agent, content)
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
            agent.stale |=
                options.effort != agent.options.effort || options.advisor != agent.options.advisor;
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
            spawn(self, app, program, id, agent)?;
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

    pub fn context_usage(
        &self,
        app: &AppHandle,
        program: &str,
        id: &str,
        start: bool,
    ) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        if !agent.session.as_ref().is_some_and(Session::alive) {
            if !start {
                return Ok(());
            }
            spawn(self, app, program, id, agent)?;
        }
        agent
            .session
            .as_mut()
            .unwrap()
            .control(serde_json::json!({"subtype": "get_context_usage"}))
            .map_err(|e| e.to_string())
    }

    pub fn stop_task(&self, id: &str, task_id: &str) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        match list
            .get_mut(id)
            .and_then(|a| a.session.as_mut())
            .filter(|s| s.alive())
        {
            Some(session) => session
                .control(serde_json::json!({"subtype": "stop_task", "task_id": task_id}))
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub fn request(
        &self,
        id: &str,
        request_id: &str,
        request: serde_json::Value,
    ) -> Result<(), String> {
        let mut list = self.list.lock().unwrap();
        list.get_mut(id)
            .and_then(|a| a.session.as_mut())
            .filter(|s| s.alive())
            .ok_or("The agent is not running.")?
            .request(request_id, request)
            .map_err(|e| e.to_string())
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
        let mut list = self.list.lock().unwrap();
        let agent = list.get_mut(id).ok_or("Unknown agent.")?;
        if let Some(session) = agent.session.take() {
            let _ = session.stop();
        }
        spawn(self, app, program, id, agent)
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
    agents: &Agents,
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
        spawn(agents, app, program, id, agent)?;
    }
    let sent = agent.session.as_mut().unwrap().send_content(content);
    if sent.is_err() {
        agent.session = None;
    }
    sent.map_err(|e| format!("Message not delivered to Claude Code: {e}"))
}

fn spawn(
    agents: &Agents,
    app: &AppHandle,
    program: &str,
    id: &str,
    agent: &mut Agent,
) -> Result<(), String> {
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
        agents.mcp(id).as_deref(),
    );
    crate::askpass::env(&mut cmd, id);
    if let Some(dir) = crate::artifacts::ROOT
        .get()
        .and_then(|root| crate::artifacts::agent_dir(root, id).ok())
    {
        let _ = std::fs::create_dir_all(&dir);
        cmd.env("SQUADRON_ARTIFACTS", dir);
    }
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
    fn keeps_the_mcp_token_in_a_private_file() {
        let dir = std::env::temp_dir().join(format!("squadron-mcp-{}", uuid::Uuid::new_v4()));
        let agents = Agents::default();
        agents.endpoint(4242, "secret".into(), dir.clone());
        let path = PathBuf::from(agents.mcp("a1").unwrap());
        assert_eq!(path, dir.join("a1.json"));
        assert!(std::fs::read_to_string(&path).unwrap().contains("secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rewinds_to_a_message_or_to_a_fresh_session() {
        let agents = Agents::default();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
            agent: None,
            advisor: None,
        };
        let session = "2654721f-e7e6-4b3c-a2a7-25979e1d1873";
        agents
            .register(
                "a1",
                PathBuf::from("/tmp"),
                options,
                Some(session.into()),
                false,
            )
            .unwrap();
        assert!(
            agents
                .rewind("a1", Some("--dangerously-skip-permissions-0000"))
                .is_err()
        );
        assert!(!agents.list.lock().unwrap()["a1"].fork);
        agents.rewind("a1", Some(session)).unwrap();
        {
            let list = agents.list.lock().unwrap();
            assert!(list["a1"].fork);
            assert_eq!(list["a1"].at.as_deref(), Some(session));
        }
        agents.rewind("a1", None).unwrap();
        assert!(!agents.list.lock().unwrap()["a1"].fork);
        assert_eq!(agents.session_id("a1"), None);
    }

    #[test]
    fn accepts_only_a_message_uuid_as_fork_point() {
        let agents = Agents::default();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
            agent: None,
            advisor: None,
        };
        agents
            .register(
                "a1",
                PathBuf::from("/tmp"),
                options.clone(),
                Some("2654721f-e7e6-4b3c-a2a7-25979e1d1873".into()),
                true,
            )
            .unwrap();
        assert!(
            agents
                .register(
                    "a2",
                    PathBuf::from("/tmp"),
                    options,
                    Some("--resume".into()),
                    false
                )
                .is_err()
        );
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
