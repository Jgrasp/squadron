use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;

pub const TESTED_VERSION: &str = "2.1.304";

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

pub fn command(dir: &Path) -> Command {
    let mut cmd = Command::new("claude");
    cmd.current_dir(dir).args([
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
    ]);
    cmd
}

pub struct Session {
    child: Child,
    stdin: ChildStdin,
}

impl Session {
    pub fn start(
        mut cmd: Command,
        mut on_event: impl FnMut(&str, Event) + Send + 'static,
    ) -> io::Result<Self> {
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
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
                                "Claude Code {v} non testé, version épinglée : {TESTED_VERSION}"
                            );
                        }
                        on_event(&line, event);
                    }
                    Err(e) => log::warn!("ligne illisible dans le flux de claude : {e}"),
                }
            }
        });
        Ok(Self { child, stdin })
    }

    pub fn send(&mut self, text: &str) -> io::Result<()> {
        let message = json!({"type": "user", "message": {"role": "user", "content": text}});
        writeln!(self.stdin, "{message}")?;
        self.stdin.flush()
    }

    /// Claude Code finit le tour en erreur puis s'arrête : continuer demandera `--resume`.
    pub fn interrupt(mut self) -> io::Result<()> {
        if unsafe { libc::kill(self.child.id() as libc::pid_t, libc::SIGINT) } != 0 {
            return Err(io::Error::last_os_error());
        }
        thread::spawn(move || self.child.wait());
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
        )
        .unwrap();
        session.send("bonjour").unwrap();
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
    fn interrupt_stops_the_process() {
        let (tx, rx) = mpsc::channel::<()>();
        let mut session = Session::start(
            fake_claude("read -r _ && exec sleep 30", ""),
            move |_, _| tx.send(()).unwrap(),
        )
        .unwrap();
        session.send("bonjour").unwrap();
        session.interrupt().unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)),
            Err(RecvTimeoutError::Disconnected)
        );
    }
}
