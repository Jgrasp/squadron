use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub const TOOL: &str = "mcp__squadron__permission";

#[derive(Clone, Debug, Serialize)]
pub struct Ask {
    pub id: String,
    pub agent: String,
    pub tool_name: String,
    pub input: Value,
    pub tool_use_id: Option<String>,
    pub extra: Map<String, Value>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct Request {
    tool_name: String,
    input: Value,
    tool_use_id: Option<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

type Pending = Arc<Mutex<HashMap<String, (String, oneshot::Sender<Value>)>>>;

#[derive(Clone)]
pub struct Gate {
    token: String,
    pending: Pending,
    next: Arc<AtomicU64>,
    notify: Arc<dyn Fn(Ask) + Send + Sync>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Gate {
    pub fn new(token: String, notify: impl Fn(Ask) + Send + Sync + 'static) -> Self {
        Self {
            token,
            pending: Default::default(),
            next: Default::default(),
            notify: Arc::new(notify),
            tool_router: Self::tool_router(),
        }
    }

    /// Demande à l'utilisateur de Squadron s'il autorise un outil.
    #[tool]
    async fn permission(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(request): Parameters<Request>,
    ) -> String {
        let header = |name: &str| {
            parts
                .headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        if header("x-squadron-token") != self.token {
            return deny("Requête refusée : jeton Squadron invalide.").to_string();
        }
        let agent = header("x-squadron-agent");
        let id = self.next.fetch_add(1, Ordering::SeqCst).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap()
            .insert(id.clone(), (agent.clone(), tx));
        (self.notify)(Ask {
            id,
            agent,
            tool_name: request.tool_name,
            input: request.input,
            tool_use_id: request.tool_use_id,
            extra: request.extra,
        });
        rx.await
            .unwrap_or_else(|_| deny("Demande annulée."))
            .to_string()
    }

    pub fn answer(&self, id: &str, decision: Value) -> Result<(), String> {
        match decision["behavior"].as_str() {
            Some("allow") if decision["updatedInput"].is_object() => {}
            Some("deny") if decision["message"].is_string() => {}
            _ => return Err("Réponse de permission invalide.".into()),
        }
        let (_, tx) = self
            .pending
            .lock()
            .unwrap()
            .remove(id)
            .ok_or("Cette demande n'attend plus de réponse.")?;
        let _ = tx.send(decision);
        Ok(())
    }

    pub fn cancel(&self, agent: &str) {
        self.pending
            .lock()
            .unwrap()
            .retain(|_, (owner, _)| owner != agent);
    }

    pub async fn serve(self) -> std::io::Result<u16> {
        let service: StreamableHttpService<Gate, LocalSessionManager> = StreamableHttpService::new(
            move || Ok(self.clone()),
            Default::default(),
            StreamableHttpServerConfig::default(),
        );
        let router = axum::Router::new().nest_service("/mcp", service);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                log::error!("serveur de permissions arrêté : {e}");
            }
        });
        Ok(port)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Gate {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

fn deny(message: &str) -> Value {
    json!({"behavior": "deny", "message": message})
}

pub fn mcp_config(port: u16, token: &str, agent: &str) -> String {
    json!({"mcpServers": {"squadron": {
        "type": "http",
        "url": format!("http://127.0.0.1:{port}/mcp"),
        "headers": {"X-Squadron-Token": token, "X-Squadron-Agent": agent},
    }}})
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{self, Event, Options, Session};
    use std::time::Duration;

    fn answer_for(ask: &Ask) -> Value {
        match ask.tool_name.as_str() {
            "AskUserQuestion" => {
                let mut input = ask.input.clone();
                let mut answers = Map::new();
                for q in input["questions"].as_array().cloned().unwrap_or_default() {
                    answers.insert(
                        q["question"].as_str().unwrap_or_default().into(),
                        "Rouge".into(),
                    );
                }
                input["answers"] = Value::Object(answers);
                json!({"behavior": "allow", "updatedInput": input})
            }
            "ExitPlanMode" => json!({"behavior": "allow", "updatedInput": ask.input}),
            _ => json!({"behavior": "allow", "updatedInput": ask.input}),
        }
    }

    async fn run(mode: &str, prompts: &[&str]) {
        let (asks_tx, mut asks_rx) = tokio::sync::mpsc::unbounded_channel::<Ask>();
        let gate = Gate::new("jeton".into(), move |ask| asks_tx.send(ask).unwrap());
        let port = gate.clone().serve().await.unwrap();
        let answering = gate.clone();
        tokio::spawn(async move {
            while let Some(ask) = asks_rx.recv().await {
                println!(
                    "ASK {} {} extra={}",
                    ask.tool_name,
                    ask.input,
                    Value::Object(ask.extra.clone())
                );
                answering.answer(&ask.id, answer_for(&ask)).unwrap();
            }
        });
        let dir = std::env::temp_dir().join(format!("squadron-gate-{mode}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: mode.into(),
        };
        let config = mcp_config(port, "jeton", "agent-1");
        let cmd = claude::command("claude", &dir, &options, None, Some(&config));
        let (tx, rx) = std::sync::mpsc::channel();
        let mut session = Session::start(
            cmd,
            move |line, event| {
                if let Event::Result { .. } = event {
                    tx.send(line.to_string()).unwrap();
                } else if line.contains("\"type\":\"assistant\"") && line.contains("\"text\"") {
                    println!("TEXT {}", &line[..line.len().min(300)]);
                }
            },
            |stderr| println!("EXIT {stderr}"),
        )
        .unwrap();
        for prompt in prompts {
            session.send(prompt).unwrap();
            let result = rx.recv_timeout(Duration::from_secs(180)).unwrap();
            println!("RESULT {}", &result[..result.len().min(600)]);
        }
        session.stop().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "lance le vrai claude"]
    async fn real_claude_default_mode() {
        run(
            "default",
            &[
                "Exécute la commande bash `curl -s -o page.html https://example.com` puis réponds OK.",
                "Utilise l'outil AskUserQuestion pour me demander si je préfère bleu ou rouge, puis répète ma réponse en un mot.",
            ],
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "lance le vrai claude"]
    async fn real_claude_plan_mode() {
        run(
            "plan",
            &["Prévois de créer un fichier hello.txt contenant bonjour. Propose ton plan."],
        )
        .await;
    }
}
