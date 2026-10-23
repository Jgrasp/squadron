use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub const TESTED_VERSION: &str = "2.1.304";

const PROMPT: &str = "You run inside Squadron, a desktop app that shows your replies as Markdown. When the user wants text to copy or paste (a message, a pull request or ticket description, a document, a config snippet), write it in your reply inside a fenced code block with its language, ```markdown for prose, then go on with the rest of your answer. Squadron shows each block with a Copy button. Write such text to a file only when the user asks for a file.";

const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
const MODES: [&str; 4] = ["default", "acceptEdits", "plan", "auto"];

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    System {
        subtype: String,
        session_id: Option<String>,
        claude_code_version: Option<String>,
    },
    #[serde(rename = "stream_event")]
    Stream {
        event: Value,
    },
    Assistant {
        message: Value,
    },
    User {
        message: Value,
        #[serde(default, rename = "isReplay")]
        replay: bool,
        uuid: Option<String>,
    },
    Result {
        subtype: String,
        is_error: bool,
        result: Option<String>,
        total_cost_usd: f64,
        session_id: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

impl Options {
    pub fn check(&self) -> Result<(), String> {
        let model_id = |c: char| c.is_ascii_alphanumeric() || "-.[]".contains(c);
        if !self.model.starts_with(|c: char| c.is_ascii_alphanumeric())
            || !self.model.chars().all(model_id)
        {
            return Err(format!("Unknown model: {}", self.model));
        }
        if !EFFORTS.contains(&self.effort.as_str()) {
            return Err(format!("Unknown effort: {}", self.effort));
        }
        if !MODES.contains(&self.mode.as_str()) {
            return Err(format!("Unknown permission mode: {}", self.mode));
        }
        Ok(())
    }
}

pub fn command(
    program: &str,
    dir: &Path,
    options: &Options,
    resume: Option<&str>,
    fork: bool,
    at: Option<&str>,
    mcp: Option<&str>,
) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir)
        .env("CLAUDE_CODE_ENABLE_TODO_TOOLS", "1")
        .args([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--replay-user-messages",
            "--thinking-display",
            "summarized",
            "--model",
            &options.model,
            "--effort",
            &options.effort,
            "--permission-mode",
            &options.mode,
            "--append-system-prompt",
            PROMPT,
        ]);
    if let Some(id) = resume {
        cmd.args(["--resume", id]);
        if fork {
            cmd.arg("--fork-session");
        }
        if let Some(uuid) = at {
            cmd.args(["--resume-session-at", uuid]);
        }
    }
    if let Some(config) = mcp {
        cmd.args([
            "--mcp-config",
            config,
            "--permission-prompt-tool",
            crate::permissions::TOOL,
        ]);
    }
    cmd
}

pub struct Session {
    pid: libc::pid_t,
    stdin: ChildStdin,
    alive: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    sent: Arc<Mutex<HashSet<String>>>,
}

impl Session {
    pub fn start(
        mut cmd: Command,
        mut on_event: impl FnMut(&str, Event) + Send + 'static,
        on_exit: impl FnOnce(String) + Send + 'static,
    ) -> io::Result<Self> {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let errors = thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let alive = Arc::new(AtomicBool::new(true));
        let flag = alive.clone();
        let busy = Arc::new(AtomicBool::new(false));
        let turn = busy.clone();
        let pid = child.id() as libc::pid_t;
        let sent = Arc::new(Mutex::new(HashSet::new()));
        let own = sent.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                match serde_json::from_str::<Event>(&line) {
                    Ok(event) => {
                        if let Event::System {
                            claude_code_version: Some(v),
                            ..
                        } = &event
                            && v != TESTED_VERSION
                        {
                            log::warn!(
                                "Claude Code {v} is untested, pinned version: {TESTED_VERSION}"
                            );
                        }
                        if let Event::Result { .. } = event {
                            turn.store(false, Ordering::SeqCst);
                        }
                        if let Event::User {
                            replay: true,
                            uuid: Some(uuid),
                            ..
                        } = &event
                            && own.lock().unwrap().remove(uuid)
                        {
                            continue;
                        }
                        on_event(&line, event);
                    }
                    Err(e) => log::warn!("unreadable line in the claude stream: {e}"),
                }
            }
            let _ = child.wait();
            flag.store(false, Ordering::SeqCst);
            on_exit(errors.join().unwrap_or_default());
        });
        Ok(Self {
            pid,
            stdin,
            alive,
            busy,
            sent,
        })
    }

    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    pub fn send(&mut self, text: &str) -> io::Result<()> {
        self.send_content(json!(text))
    }

    pub fn send_content(&mut self, content: Value) -> io::Result<()> {
        let uuid = uuid::Uuid::new_v4().to_string();
        self.sent.lock().unwrap().insert(uuid.clone());
        let message =
            json!({"type": "user", "uuid": uuid, "message": {"role": "user", "content": content}});
        writeln!(self.stdin, "{message}")?;
        self.stdin.flush()?;
        self.busy.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Changes the permission mode or the model of the running session, or interrupts its turn; the effort has no such request.
    pub fn control(&mut self, request: Value) -> io::Result<()> {
        let id = format!("squadron-{}", self.pid);
        writeln!(
            self.stdin,
            "{}",
            json!({"type": "control_request", "request_id": id, "request": request})
        )?;
        self.stdin.flush()
    }

    /// Claude Code ends the turn with an error then exits: continuing needs `--resume`.
    pub fn interrupt(&self) -> io::Result<()> {
        self.signal(libc::SIGINT)
    }

    pub fn stop(&self) -> io::Result<()> {
        self.signal(libc::SIGTERM)
    }

    fn signal(&self, signal: libc::c_int) -> io::Result<()> {
        if !self.alive() {
            return Ok(());
        }
        if unsafe { libc::kill(self.pid, signal) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Duration;

    fn fake_claude(script: &str, fixture: &str) -> Command {
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            script,
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/"),
        ])
        .arg(fixture);
        cmd
    }

    fn replay(fixture: &str) -> Vec<Event> {
        let (tx, rx) = mpsc::channel();
        let mut session = Session::start(
            fake_claude(r#"read -r _ && cat "$0$1""#, fixture),
            move |_, event| tx.send(event).unwrap(),
            |_| {},
        )
        .unwrap();
        session.send("hello").unwrap();
        rx.iter().collect()
    }

    fn lines(fixture: &str) -> usize {
        let path = format!("{}/tests/fixtures/{fixture}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap().lines().count()
    }

    #[test]
    fn replays_recorded_streams() {
        for (fixture, error) in [
            ("hello.jsonl", false),
            ("tools.jsonl", false),
            ("sigint.jsonl", true),
            ("twoturns.jsonl", false),
            ("nologin.jsonl", true),
        ] {
            let events = replay(fixture);
            assert_eq!(events.len(), lines(fixture), "{fixture}");
            assert!(matches!(
                &events[0],
                Event::System { subtype, claude_code_version: Some(v), .. }
                    if subtype == "init" && v == TESTED_VERSION
            ));
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, Event::Result { is_error, .. } if *is_error == error)),
                "{fixture}"
            );
        }
    }

    #[test]
    fn tool_calls_are_typed() {
        let events = replay("tools.jsonl");
        let tools: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Assistant { message } => message["content"][0]["name"].as_str(),
                _ => None,
            })
            .collect();
        assert_eq!(tools, ["Read", "Bash", "Write"]);
    }

    #[test]
    fn interrupt_stops_the_process_and_reports_stderr() {
        let (started, ready) = mpsc::channel();
        let (tx, rx) = mpsc::channel();
        let mut session = Session::start(
            fake_claude(
                r#"echo oups >&2; echo '{"type":"ready"}'; read -r _ && exec sleep 30"#,
                "",
            ),
            move |_, _| started.send(()).unwrap(),
            move |stderr| tx.send(stderr).unwrap(),
        )
        .unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        session.send("hello").unwrap();
        assert!(session.alive());
        session.interrupt().unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), "oups\n");
        assert!(!session.alive());
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(100)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn forks_a_session_at_a_message() {
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let at = "2654721f-e7e6-4b3c-a2a7-25979e1d1873";
        let cmd = command(
            "claude",
            Path::new("/tmp"),
            &options,
            Some("s1"),
            true,
            Some(at),
            None,
        );
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let tail = &args[args.len() - 5..];
        assert_eq!(
            tail,
            [
                "--resume",
                "s1",
                "--fork-session",
                "--resume-session-at",
                at
            ]
        );
    }

    #[test]
    fn rejects_unknown_options() {
        let ok = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "auto".into(),
        };
        assert!(ok.check().is_ok());
        let flag = Options {
            model: "--dangerously-skip-permissions".into(),
            ..ok.clone()
        };
        assert!(flag.check().is_err());
        let pinned = Options {
            model: "claude-opus-4-7".into(),
            ..ok.clone()
        };
        assert!(pinned.check().is_ok());
        let spaced = Options {
            model: "opus --verbose".into(),
            ..ok
        };
        assert!(spaced.check().is_err());
    }

    fn real_turn(dir: &Path, resume: Option<&str>, fork: bool, prompt: &str) -> (String, String) {
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let (tx, rx) = mpsc::channel();
        let mut session = Session::start(
            command("claude", dir, &options, resume, fork, None, None),
            move |_, event| {
                if let Event::Result {
                    result, session_id, ..
                } = event
                {
                    tx.send((session_id, result.unwrap_or_default())).unwrap();
                }
            },
            |_| {},
        )
        .unwrap();
        session.send(prompt).unwrap();
        let answer = rx.recv_timeout(Duration::from_secs(120)).unwrap();
        session.stop().unwrap();
        answer
    }

    #[test]
    #[ignore = "runs the real claude"]
    fn real_claude_resumes_and_forks_a_session() {
        let dir = std::env::temp_dir().join("squadron-resume");
        std::fs::create_dir_all(&dir).unwrap();
        let (id, _) = real_turn(
            &dir,
            None,
            false,
            "Remember the code word PELICAN. Reply only OK.",
        );
        let (resumed, answer) = real_turn(
            &dir,
            Some(&id),
            false,
            "What is the code word? Reply with the word only.",
        );
        println!("resumed {resumed} -> {answer}");
        assert_eq!(resumed, id);
        assert!(answer.contains("PELICAN"));
        let (forked, answer) = real_turn(
            &dir,
            Some(&id),
            true,
            "What is the code word? Reply with the word only.",
        );
        println!("forked {forked} -> {answer}");
        assert_ne!(forked, id);
        assert!(answer.contains("PELICAN"));
    }

    #[test]
    #[ignore = "runs the real claude"]
    fn real_claude_echoes_only_messages_squadron_did_not_send() {
        let dir = std::env::temp_dir().join("squadron-echo");
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let (tx, rx) = mpsc::channel();
        let mut session = Session::start(
            command("claude", &dir, &options, None, false, None, None),
            move |line, event| {
                let done = matches!(event, Event::Result { .. });
                tx.send((line.to_string(), done)).unwrap();
            },
            |_| {},
        )
        .unwrap();
        session.send("Reply only OK.").unwrap();
        let mut lines = Vec::new();
        loop {
            let (line, done) = rx.recv_timeout(Duration::from_secs(120)).unwrap();
            lines.push(line);
            if done {
                break;
            }
        }
        session.stop().unwrap();
        assert!(lines.iter().all(|l| !l.contains(r#""isReplay":true"#)));
        assert!(session.sent.lock().unwrap().is_empty());
    }
}

#[cfg(test)]
mod live_options {
    use super::*;
    use std::time::Duration;

    #[test]
    #[ignore = "runs the real claude"]
    fn real_claude_changes_mode_and_model_mid_session() {
        let dir = std::env::temp_dir().join("squadron-live-options");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: "sonnet".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let mut session = Session::start(
            command("claude", &dir, &options, None, false, None, None),
            move |line, event| {
                let d: Value = serde_json::from_str(line).unwrap();
                match event {
                    Event::System { subtype, .. } if subtype == "init" => tx
                        .send(format!("init {} {}", d["model"], d["permissionMode"]))
                        .unwrap(),
                    Event::Result { .. } => tx.send("result".into()).unwrap(),
                    _ if d["type"] == "control_response" => {
                        let commands = d["response"]["response"]["commands"]
                            .as_array()
                            .map_or(0, Vec::len);
                        tx.send(format!("control {} {commands}", d["response"]["subtype"]))
                            .unwrap()
                    }
                    _ => {}
                }
            },
            |_| {},
        )
        .unwrap();
        let next = |rx: &std::sync::mpsc::Receiver<String>, what: &str| loop {
            let line = rx.recv_timeout(Duration::from_secs(120)).unwrap();
            println!("{line}");
            if line.starts_with(what) {
                return line;
            }
        };
        session.control(json!({"subtype": "initialize"})).unwrap();
        session.send("Reply with just: one").unwrap();
        let catalog = next(&rx, "control");
        assert!(
            catalog.starts_with("control \"success\"") && !catalog.ends_with(" 0"),
            "{catalog}"
        );
        assert!(next(&rx, "init").contains("default"));
        next(&rx, "result");
        assert!(!session.busy());
        session
            .control(json!({"subtype": "set_permission_mode", "mode": "acceptEdits"}))
            .unwrap();
        assert_eq!(next(&rx, "control"), "control \"success\" 0");
        session
            .control(json!({"subtype": "set_model", "model": "haiku"}))
            .unwrap();
        assert_eq!(next(&rx, "control"), "control \"success\" 0");
        session.send("Reply with just: two").unwrap();
        assert!(session.busy());
        let init = next(&rx, "init");
        assert!(
            init.contains("acceptEdits") && init.contains("haiku"),
            "{init}"
        );
        next(&rx, "result");
        session.stop().unwrap();
    }
}
