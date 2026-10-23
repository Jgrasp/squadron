use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub const INSTRUCTIONS: &str = "You run inside Squadron, the desktop app where the user drives several Claude Code agents. \
These tools act on Squadron itself: its projects and agents, the Kanban cards, and the layout of its buttons and menus. \
When the user asks to change Squadron (move or hide a button, add a card, start an agent), use these tools and never edit Squadron's source code. \
You can prepare and start new agents as the user would, but you never answer, approve or steer an existing agent in the user's place. \
When you start working in a git worktree that you entered yourself (a script, git worktree add, cd), call set_worktree so Squadron shows its branch. \
To start an agent in a worktree a script made, pass its path to start_agent.";

#[derive(Clone, Debug, Serialize)]
pub struct Call {
    pub id: String,
    pub agent: String,
    pub tool: String,
    pub args: Value,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReadAgent {
    /// The agent id, as listed by `overview`.
    agent: String,
    /// How many of its last messages to return, 10 by default.
    last: Option<u32>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct StartAgent {
    /// The first message of the new agent: what it must do. Taken from the card when you start a card and leave it empty.
    prompt: Option<String>,
    /// A short name for the agent. The card title, or a name Squadron suggests, when left empty.
    name: Option<String>,
    /// Project name or path. Your own project when left empty.
    project: Option<String>,
    /// opus, fable, sonnet, haiku, or a pinned version such as claude-opus-4-7. The user's default when left empty.
    model: Option<String>,
    /// low, medium, high, xhigh or max. The user's default when left empty.
    effort: Option<String>,
    /// default, plan, acceptEdits or auto. acceptEdits and auto need the user's confirmation.
    mode: Option<String>,
    /// Id of a Kanban card the agent works on; the card then follows the agent.
    card: Option<String>,
    /// When the task is done, in the user's words (\"done when…\"): Squadron runs it as Claude Code's /goal, so the agent keeps working after each turn until it is true, then the card goes to review. Ask for visible proof, such as tests run with their output shown. In plan mode it starts once the user approves the plan. Taken from the card when left empty.
    done_when: Option<String>,
    /// Absolute path of an existing git worktree, made by a script or git worktree add: the agent starts and keeps running there, and Squadron shows its branch.
    worktree: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateCard {
    /// What the task is about.
    title: String,
    /// The description: the first message an agent started from this card receives.
    prompt: Option<String>,
    /// Optional. When the task is done, in the user's words (\"done when…\"): Squadron runs it as Claude Code's /goal, so the agent keeps working after each turn until it is true, then the card goes to review. Ask for visible proof, such as tests run with their output shown. In plan mode it starts once the user approves the plan.
    done_when: Option<String>,
    /// Project name or path. Your own project when left empty.
    project: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetWorktree {
    /// The absolute path of the git worktree, or an empty string when the agent is back in the project's main checkout.
    path: String,
    /// The agent id, as listed by overview, to set the worktree of another agent. Yourself when left empty.
    agent: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct MoveCard {
    /// The card id, as listed by `overview`.
    card: String,
    /// todo, progress, review or done.
    column: String,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetLayout {
    /// The slot to change, as listed by `get_layout`.
    slot: String,
    /// Every item id of the slot, in the new order. Omit to keep the order.
    order: Option<Vec<String>>,
    /// The item ids to hide; the others are shown. Omit to keep the visibility.
    hidden: Option<Vec<String>>,
}

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

#[derive(Clone)]
pub struct Pilot {
    token: String,
    pending: Pending,
    next: Arc<AtomicU64>,
    notify: Arc<dyn Fn(Call) + Send + Sync>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Pilot {
    pub fn new(token: String, notify: impl Fn(Call) + Send + Sync + 'static) -> Self {
        Self {
            token,
            pending: Default::default(),
            next: Default::default(),
            notify: Arc::new(notify),
            tool_router: Self::tool_router(),
        }
    }

    async fn forward(&self, parts: &http::request::Parts, tool: &str, args: Value) -> String {
        let header = |name: &str| {
            parts
                .headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        if header("x-squadron-token") != self.token {
            return "Refused: invalid Squadron token.".into();
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);
        (self.notify)(Call {
            id,
            agent: header("x-squadron-agent"),
            tool: tool.into(),
            args,
        });
        rx.await
            .unwrap_or_else(|_| "Squadron did not answer.".into())
    }

    pub fn reply(&self, id: &str, result: String) -> Result<(), String> {
        let tx = self
            .pending
            .lock()
            .unwrap()
            .remove(id)
            .ok_or("This call no longer waits for an answer.")?;
        let _ = tx.send(result);
        Ok(())
    }

    /// Lists Squadron's projects with their agents (id, name, state, what they wait for, cost) and their Kanban cards.
    #[tool]
    async fn overview(&self, Extension(parts): Extension<http::request::Parts>) -> String {
        self.forward(&parts, "overview", json!({})).await
    }

    /// Reads the state and the last messages of one agent.
    #[tool]
    async fn read_agent(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<ReadAgent>,
    ) -> String {
        self.forward(&parts, "read_agent", json!(args)).await
    }

    /// Starts a new agent in Squadron, as the user would from the New agent dialog or from a Kanban card. With done_when, the agent keeps working until it is true. Its permission requests and questions go to the user.
    #[tool]
    async fn start_agent(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<StartAgent>,
    ) -> String {
        self.forward(&parts, "start_agent", json!(args)).await
    }

    /// Adds a task to the To do column of a project's Kanban, with its description and, when the user says when it is done, done_when. Start its agent later with start_agent and the card id.
    #[tool]
    async fn create_card(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<CreateCard>,
    ) -> String {
        self.forward(&parts, "create_card", json!(args)).await
    }

    /// Moves a Kanban card to another column.
    #[tool]
    async fn move_card(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<MoveCard>,
    ) -> String {
        self.forward(&parts, "move_card", json!(args)).await
    }

    /// Tells Squadron which git worktree an agent works in, yours by default, when it was entered by hand (a script, git worktree add, cd): Squadron shows its branch and runs the agent there from its next start. Not needed when Claude Code created it (--worktree, EnterWorktree).
    #[tool]
    async fn set_worktree(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<SetWorktree>,
    ) -> String {
        self.forward(&parts, "set_worktree", json!(args)).await
    }

    /// Returns Squadron's layout: each slot (a bar of buttons or a menu), its items and which are hidden.
    #[tool]
    async fn get_layout(&self, Extension(parts): Extension<http::request::Parts>) -> String {
        self.forward(&parts, "get_layout", json!({})).await
    }

    /// Changes the order or the hidden items of one slot of Squadron's layout. Use this to move or hide a button or a menu entry.
    #[tool]
    async fn set_layout(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<SetLayout>,
    ) -> String {
        self.forward(&parts, "set_layout", json!(args)).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Pilot {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{self, Event, Options, Session};
    use crate::permissions::{Gate, mcp_config, serve_with_pilot};
    use std::time::Duration;

    async fn drive(model: &str, prompt: &str, reply: fn(&Call) -> String) -> Vec<Call> {
        let (asks_tx, mut asks_rx) = tokio::sync::mpsc::unbounded_channel();
        let gate = Gate::new("token".into(), move |ask| asks_tx.send(ask).unwrap());
        let (calls_tx, mut calls_rx) = tokio::sync::mpsc::unbounded_channel::<Call>();
        let pilot = Pilot::new("token".into(), move |call| calls_tx.send(call).unwrap());
        let port = serve_with_pilot(gate.clone(), pilot.clone()).await.unwrap();
        tokio::spawn(async move {
            while let Some(ask) = asks_rx.recv().await {
                let ask: crate::permissions::Ask = ask;
                println!("ASK {}", ask.tool_name);
                gate.answer(
                    &ask.id,
                    json!({"behavior": "allow", "updatedInput": ask.input}),
                )
                .unwrap();
            }
        });
        let seen = Arc::new(Mutex::new(Vec::<Call>::new()));
        let record = seen.clone();
        tokio::spawn(async move {
            while let Some(call) = calls_rx.recv().await {
                println!("CALL {} {} {}", call.agent, call.tool, call.args);
                record.lock().unwrap().push(call.clone());
                pilot.reply(&call.id, reply(&call)).unwrap();
            }
        });
        let dir = std::env::temp_dir().join("squadron-pilot-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: model.into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let config = mcp_config(port, "token", "agent-7", true);
        let cmd = claude::command("claude", &dir, &options, None, false, None, Some(&config));
        let (tx, rx) = std::sync::mpsc::channel();
        let mut session = Session::start(
            cmd,
            move |line, event| {
                if let Event::Result { .. } = event {
                    tx.send(line.to_string()).unwrap();
                }
            },
            |stderr| println!("EXIT {stderr}"),
        )
        .unwrap();
        session.control(json!({"subtype": "initialize"})).unwrap();
        session.send(prompt).unwrap();
        let result = rx.recv_timeout(Duration::from_secs(240)).unwrap();
        println!("RESULT {}", &result[..result.len().min(400)]);
        session.stop().unwrap();
        let calls = seen.lock().unwrap().clone();
        assert!(calls.iter().all(|c| c.agent == "agent-7"));
        calls
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_moves_a_button_with_the_pilot() {
        let calls = drive(
            "haiku",
            "In Squadron, put the Editor button at the far right of the agent tools.",
            |call| match call.tool.as_str() {
                "get_layout" => json!({"slots": [{"slot": "agent.tools", "items": ["editor", "terminal", "diff", "kanban"], "hidden": []}]}).to_string(),
                _ => "Done.".to_string(),
            },
        )
        .await;
        let set = calls
            .iter()
            .find(|c| c.tool == "set_layout")
            .expect("set_layout called");
        assert_eq!(set.args["slot"], "agent.tools");
        assert_eq!(
            set.args["order"].as_array().unwrap().last().unwrap(),
            "editor"
        );
    }

    fn script_worktree() {
        let main = std::env::temp_dir().join("squadron-wt-main");
        let _ = std::fs::remove_dir_all(&main);
        let _ = std::fs::remove_dir_all("/tmp/squadron-wt");
        std::fs::create_dir_all(&main).unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .current_dir(&main)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["commit", "-q", "--allow-empty", "-m", "init"]);
        git(&[
            "worktree",
            "add",
            "-q",
            "/tmp/squadron-wt/ORB-809-T70",
            "-b",
            "ia/ORB-809-T70",
        ]);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_starts_an_agent_in_the_worktree_a_script_made() {
        script_worktree();
        let calls = drive(
            "sonnet",
            "`make worktree TICKET=ORB-809 TACHE=T70` just created the worktree /tmp/squadron-wt/ORB-809-T70 for task T70. Start an agent with haiku that works in it on: fix the admin menu.",
            |call| match call.tool.as_str() {
                "start_agent" if call.args["prompt"].is_null() => {
                    "Error: The prompt is empty.".to_string()
                }
                "start_agent" => "Started agent a-9 in Orbit.".to_string(),
                _ => "[]".to_string(),
            },
        )
        .await;
        let start = calls
            .iter()
            .find(|c| c.tool == "start_agent" && !c.args["prompt"].is_null())
            .expect("start_agent called");
        assert_eq!(start.args["worktree"], "/tmp/squadron-wt/ORB-809-T70");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_declares_the_worktree_a_script_made() {
        script_worktree();
        let calls = drive(
            "sonnet",
            "I just ran `make worktree TICKET=ORB-809 TACHE=T70`: it created a git worktree at /tmp/squadron-wt/ORB-809-T70 on the branch ia/ORB-809-T70. From now on you work there. Get ready, nothing else to do yet.",
            |call| match call.tool.as_str() {
                "set_worktree" if call.args["path"].as_str().unwrap_or_default().is_empty() => {
                    "Squadron shows the branch of the main checkout again.".to_string()
                }
                "set_worktree" => "Squadron now shows that you work in the worktree.".to_string(),
                _ => "[]".to_string(),
            },
        )
        .await;
        let set = calls
            .iter()
            .find(|c| c.tool == "set_worktree")
            .expect("set_worktree called");
        assert_eq!(set.args["path"], "/tmp/squadron-wt/ORB-809-T70");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_creates_a_task_with_a_goal_and_starts_it() {
        let calls = drive(
            "sonnet",
            "In Squadron's Kanban, add a task to fix the CSV export of segments. It is done when the export tests pass with their output shown. Then start its agent with haiku.",
            |call| match call.tool.as_str() {
                "create_card" => "Card c-42 added to the To do column of Nova.".to_string(),
                "start_agent" => "Started agent a-9 in Nova.".to_string(),
                _ => "[]".to_string(),
            },
        )
        .await;
        let card = calls
            .iter()
            .find(|c| c.tool == "create_card")
            .expect("create_card called");
        assert!(
            card.args["done_when"]
                .as_str()
                .unwrap_or_default()
                .contains("test")
        );
        let start = calls
            .iter()
            .find(|c| c.tool == "start_agent")
            .expect("start_agent called");
        assert_eq!(start.args["card"], "c-42");
        assert_eq!(start.args["model"], "haiku");
    }
}
