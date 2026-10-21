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
You can prepare and start new agents as the user would, but you never answer, approve or steer an existing agent in the user's place.";

#[derive(Clone, Debug, Serialize)]
pub struct Call {
    pub id: String,
    pub agent: String,
    pub tool: String,
    pub args: Value,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
struct ReadAgent {
    /// The agent id, as listed by `overview`.
    agent: String,
    /// How many of its last messages to return, 10 by default.
    last: Option<u32>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
struct StartAgent {
    /// The first message of the new agent: what it must do.
    prompt: String,
    /// A short name for the agent. Squadron suggests one when left empty.
    name: Option<String>,
    /// Project name or path. Your own project when left empty.
    project: Option<String>,
    /// opus, sonnet or haiku. The user's default when left empty.
    model: Option<String>,
    /// low, medium, high, xhigh or max. The user's default when left empty.
    effort: Option<String>,
    /// default, plan, acceptEdits or auto. acceptEdits and auto need the user's confirmation.
    mode: Option<String>,
    /// Id of a Kanban card the agent works on; the card then follows the agent.
    card: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
struct CreateCard {
    /// What the card is about.
    title: String,
    /// An optional ticket key, such as T16 or BAT-812.
    key: Option<String>,
    /// The instruction an agent started from this card receives.
    prompt: Option<String>,
    /// Project name or path. Your own project when left empty.
    project: Option<String>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
struct MoveCard {
    /// The card id, as listed by `overview`.
    card: String,
    /// todo, progress, review or done.
    column: String,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
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

    /// Starts a new agent in Squadron with its first message, as the user would from the New agent dialog. Its permission requests and questions go to the user.
    #[tool]
    async fn start_agent(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(args): Parameters<StartAgent>,
    ) -> String {
        self.forward(&parts, "start_agent", json!(args)).await
    }

    /// Adds a card to the To do column of a project's Kanban.
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

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_moves_a_button_with_the_pilot() {
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
                let result = match call.tool.as_str() {
                    "get_layout" => json!({"slots": [{"slot": "agent.tools", "items": ["editor", "terminal", "diff", "kanban"], "hidden": []}]}).to_string(),
                    _ => "Done.".to_string(),
                };
                record.lock().unwrap().push(call.clone());
                pilot.reply(&call.id, result).unwrap();
            }
        });
        let dir = std::env::temp_dir().join("squadron-pilot-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let config = mcp_config(port, "token", "agent-7", true);
        let cmd = claude::command("claude", &dir, &options, None, false, Some(&config));
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
        session
            .send("In Squadron, put the Editor button at the far right of the agent tools.")
            .unwrap();
        let result = rx.recv_timeout(Duration::from_secs(240)).unwrap();
        println!("RESULT {}", &result[..result.len().min(400)]);
        session.stop().unwrap();
        let calls = seen.lock().unwrap();
        assert!(calls.iter().all(|c| c.agent == "agent-7"));
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
}
