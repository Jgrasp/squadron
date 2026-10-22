use serde_json::{Value, json};
use squadron_lib::claude::{Options, Session, command};
use squadron_lib::permissions::{Ask, Gate, mcp_config};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

enum Seen {
    Ask(Ask),
    Line(Value),
    Feedback(String),
    Exit,
}

struct Agent {
    gate: Gate,
    port: u16,
    seen: UnboundedReceiver<Seen>,
    tx: UnboundedSender<Seen>,
    session: Option<Session>,
    dir: PathBuf,
    mode: String,
    id: String,
    feedbacks: usize,
}

fn feedback(line: &Value) -> Option<String> {
    if line["type"] != "user" {
        return None;
    }
    line["message"]["content"]
        .as_array()?
        .iter()
        .filter_map(|c| c["text"].as_str())
        .find(|t| t.starts_with("Stop hook feedback"))
        .map(str::to_string)
}

impl Agent {
    async fn start(name: &str, mode: &str) -> Self {
        let (tx, seen) = unbounded_channel();
        let asks = tx.clone();
        let gate = Gate::new("token".into(), move |ask| {
            let _ = asks.send(Seen::Ask(ask));
        });
        let port = gate.clone().serve().await.unwrap();
        let dir = std::env::temp_dir().join(format!("squadron-goal-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut agent = Self {
            gate,
            port,
            seen,
            tx,
            session: None,
            dir,
            mode: mode.into(),
            id: String::new(),
            feedbacks: 0,
        };
        agent.launch(None);
        agent
    }

    fn launch(&mut self, resume: Option<&str>) {
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: self.mode.clone(),
        };
        let config = mcp_config(self.port, "token", "agent-1", false);
        let cmd = command("claude", &self.dir, &options, resume, false, Some(&config));
        let (lines, exit) = (self.tx.clone(), self.tx.clone());
        let mut record = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("stream.jsonl"))
            .unwrap();
        self.session = Some(
            Session::start(
                cmd,
                move |line, _| {
                    writeln!(record, "{line}").unwrap();
                    if let Ok(value) = serde_json::from_str::<Value>(line) {
                        let seen = match feedback(&value) {
                            Some(text) => Seen::Feedback(text),
                            None => Seen::Line(value),
                        };
                        let _ = lines.send(seen);
                    }
                },
                move |_| {
                    let _ = exit.send(Seen::Exit);
                },
            )
            .unwrap(),
        );
    }

    fn send(&mut self, text: &str) {
        println!("SEND {text}");
        self.session.as_mut().unwrap().send(text).unwrap();
    }

    fn allow(&self, ask: &Ask) {
        let decision = json!({"behavior": "allow", "updatedInput": ask.input});
        self.gate.answer(&ask.id, decision).unwrap();
    }

    fn deny(&self, ask: &Ask) {
        let decision = json!({"behavior": "deny", "message": "The user refused."});
        self.gate.answer(&ask.id, decision).unwrap();
    }

    async fn next(&mut self, idle: Duration) -> Option<Seen> {
        loop {
            let seen = tokio::time::timeout(idle, self.seen.recv()).await.ok()??;
            match &seen {
                Seen::Line(v) if v["type"] == "system" && v["subtype"] == "init" => {
                    self.id = v["session_id"].as_str().unwrap_or_default().into();
                }
                Seen::Line(v) if v["type"] == "assistant" => {
                    for c in v["message"]["content"].as_array().into_iter().flatten() {
                        if let Some(t) = c["text"].as_str() {
                            println!("TEXT {}", t.chars().take(160).collect::<String>());
                        }
                    }
                }
                Seen::Line(v) if v["type"] == "result" => {
                    println!(
                        "RESULT turns={} error={} cost={}",
                        v["num_turns"], v["is_error"], v["total_cost_usd"]
                    );
                    return Some(seen);
                }
                Seen::Line(_) => {}
                Seen::Feedback(text) => {
                    self.feedbacks += 1;
                    let reason: String = text.chars().take(200).collect();
                    println!("FEEDBACK {} {reason}", self.feedbacks);
                    return Some(seen);
                }
                Seen::Ask(ask) => {
                    println!("ASK {}", ask.tool_name);
                    return Some(seen);
                }
                Seen::Exit => {
                    println!("EXIT");
                    return Some(seen);
                }
            }
        }
    }

    async fn settle(&mut self, deny_tools: bool) {
        while let Some(seen) = self.next(Duration::from_secs(40)).await {
            if let Seen::Ask(ask) = seen {
                if deny_tools {
                    self.deny(&ask)
                } else {
                    self.allow(&ask)
                }
            }
        }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.dir.join("stream.jsonl")).unwrap()
    }
}

const IMPOSSIBLE: &str = "/goal the file /etc/squadron-goal-marker exists and contains ok";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "runs the real claude"]
async fn goal_is_armed_when_the_plan_is_approved() {
    let mut agent = Agent::start("plan", "plan").await;
    agent.send("Write a Python script count.py that prints the numbers 1 to 5, one per line.");
    let mut plans = 0;
    while let Some(seen) = agent.next(Duration::from_secs(60)).await {
        match seen {
            Seen::Ask(ask) if ask.tool_name == "ExitPlanMode" => {
                plans += 1;
                if plans == 1 {
                    let decision = json!({"behavior": "deny", "message": "Also print done after the numbers."});
                    agent.gate.answer(&ask.id, decision).unwrap();
                } else {
                    agent.allow(&ask);
                    agent.send(
                        "/goal running python3 count.py prints 1 to 5 one per line, then done",
                    );
                }
            }
            Seen::Ask(ask) => agent.allow(&ask),
            Seen::Exit => panic!("claude exited"),
            _ => {}
        }
    }
    let output = std::process::Command::new("python3")
        .arg("count.py")
        .current_dir(&agent.dir)
        .output()
        .unwrap();
    println!("PLANS {plans} FEEDBACKS {}", agent.feedbacks);
    assert!(plans >= 2);
    assert!(agent.log().contains("Goal set"));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1\n2\n3\n4\n5\ndone\n"
    );
}

impl Agent {
    async fn relaunch_until(&mut self, stop: impl Fn(&mut Self)) {
        let mut stopped = false;
        while let Some(seen) = self.next(Duration::from_secs(60)).await {
            match seen {
                Seen::Ask(ask) => self.deny(&ask),
                Seen::Feedback(_) if !stopped => {
                    stopped = true;
                    stop(self);
                }
                Seen::Line(v) if v["type"] == "result" => break,
                Seen::Exit => break,
                _ => {}
            }
        }
    }

    async fn relaunches_after(&mut self, text: &str) -> usize {
        let before = self.feedbacks;
        self.send(text);
        self.settle(true).await;
        self.feedbacks - before
    }

    fn interrupt(&mut self) {
        let interrupt = json!({"subtype": "interrupt"});
        self.session.as_mut().unwrap().control(interrupt).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "runs the real claude"]
async fn a_goal_clear_sent_during_the_relaunches_waits_for_the_end_of_the_turn() {
    let mut agent = Agent::start("clear", "default").await;
    agent.send(IMPOSSIBLE);
    agent.relaunch_until(|a| a.send("/goal clear")).await;
    assert!(agent.feedbacks > 2, "{}", agent.feedbacks);
    agent.settle(true).await;
    assert!(agent.log().contains("Goal cleared"));
    assert_eq!(agent.relaunches_after("Say hi in one word.").await, 0);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "runs the real claude"]
async fn an_interrupt_request_stops_the_relaunches_and_keeps_the_session() {
    let mut agent = Agent::start("interrupt", "default").await;
    agent.send(IMPOSSIBLE);
    agent.relaunch_until(Agent::interrupt).await;
    assert_eq!(agent.feedbacks, 1);
    assert!(agent.session.as_ref().unwrap().alive());
    assert_eq!(agent.relaunches_after("/goal clear").await, 0);
    assert!(agent.log().contains("Goal cleared"));
    assert_eq!(agent.relaunches_after("Say hi in one word.").await, 0);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "runs the real claude"]
async fn the_goal_survives_a_resume_until_it_is_cleared() {
    let mut agent = Agent::start("resume", "default").await;
    agent.send(IMPOSSIBLE);
    agent
        .relaunch_until(|a| a.session.as_ref().unwrap().interrupt().unwrap())
        .await;
    agent.settle(true).await;
    let id = agent.id.clone();
    agent.launch(Some(&id));
    let before = agent.feedbacks;
    agent.send("Say hi in one word.");
    agent.relaunch_until(Agent::interrupt).await;
    assert!(
        agent.feedbacks > before,
        "the goal did not survive the resume"
    );
    assert_eq!(agent.relaunches_after("/goal clear").await, 0);
    assert_eq!(agent.relaunches_after("Say hi in one word.").await, 0);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "runs the real claude"]
async fn a_goal_cleared_first_after_a_resume_does_not_relaunch() {
    let mut agent = Agent::start("sigint", "default").await;
    agent.send(IMPOSSIBLE);
    agent
        .relaunch_until(|a| a.session.as_ref().unwrap().interrupt().unwrap())
        .await;
    agent.settle(true).await;
    let id = agent.id.clone();
    agent.launch(Some(&id));
    assert_eq!(agent.relaunches_after("/goal clear").await, 0);
    assert!(agent.log().contains("Goal cleared"));
    assert_eq!(agent.relaunches_after("Say hi in one word.").await, 0);
}
