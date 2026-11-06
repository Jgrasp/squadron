use crate::pilot::Call;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, SubscriptionFilter, Tool,
};
use rmcp::service::{NotificationContext, RequestContext, SubscriptionContext, SubscriptionSink};
use rmcp::{ErrorData, Peer, RoleServer, ServerHandler};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_OUTPUT: usize = 30_000;

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<(bool, String)>>>>;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ExtensionTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input: Option<Map<String, Value>>,
}

#[derive(Clone)]
pub struct Tools {
    token: String,
    list: Arc<Mutex<Vec<ExtensionTool>>>,
    peers: Arc<Mutex<Vec<Peer<RoleServer>>>>,
    sinks: Arc<Mutex<Vec<SubscriptionSink>>>,
    pending: Pending,
    next: Arc<AtomicU64>,
    notify: Arc<dyn Fn(Call) + Send + Sync>,
}

impl Tools {
    pub fn new(token: String, notify: impl Fn(Call) + Send + Sync + 'static) -> Self {
        Self {
            token,
            list: Default::default(),
            peers: Default::default(),
            sinks: Default::default(),
            pending: Default::default(),
            next: Default::default(),
            notify: Arc::new(notify),
        }
    }

    pub async fn set(&self, tools: Vec<ExtensionTool>) {
        {
            let mut list = self.list.lock().unwrap();
            if *list == tools {
                return;
            }
            *list = tools;
        }
        let peers = {
            let mut peers = self.peers.lock().unwrap();
            peers.retain(|p| !p.is_transport_closed());
            peers.clone()
        };
        for peer in peers {
            let _ = peer.notify_tool_list_changed().await;
        }
        let sinks = self.sinks.lock().unwrap().clone();
        let mut closed = Vec::new();
        for sink in sinks {
            if sink.notify_tool_list_changed().await.is_err() {
                closed.push(sink.id().clone());
            }
        }
        self.sinks
            .lock()
            .unwrap()
            .retain(|s| !closed.contains(s.id()));
    }

    pub fn reply(&self, id: &str, ok: bool, text: String) -> Result<(), String> {
        let tx = self
            .pending
            .lock()
            .unwrap()
            .remove(id)
            .ok_or("This call no longer waits for an answer.")?;
        let _ = tx.send((ok, text));
        Ok(())
    }
}

impl ServerHandler for Tools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        self.peers.lock().unwrap().push(context.peer);
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(requested.clone())
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        self.sinks.lock().unwrap().push(context.sink().clone());
        context.cancelled().await;
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let empty = Map::from_iter([("type".into(), Value::from("object"))]);
        let tools = self
            .list
            .lock()
            .unwrap()
            .iter()
            .map(|t| {
                Tool::new(
                    t.name.clone(),
                    t.description.clone(),
                    Arc::new(t.input.clone().unwrap_or_else(|| empty.clone())),
                )
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let header = |name: &str| {
            context
                .extensions
                .get::<http::request::Parts>()
                .and_then(|p| p.headers.get(name))
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        let (ok, text) = if header("x-squadron-token") != self.token {
            (false, "Refused: invalid Squadron token.".into())
        } else {
            let id = self.next.fetch_add(1, Ordering::SeqCst).to_string();
            let (tx, rx) = oneshot::channel();
            self.pending.lock().unwrap().insert(id.clone(), tx);
            (self.notify)(Call {
                id,
                agent: header("x-squadron-agent"),
                tool: request.name.to_string(),
                args: Value::Object(request.arguments.unwrap_or_default()),
            });
            rx.await
                .unwrap_or_else(|_| (false, "Squadron did not answer.".into()))
        };
        let content = vec![ContentBlock::text(text)];
        Ok(if ok {
            CallToolResult::success(content)
        } else {
            CallToolResult::error(content)
        }
        .into())
    }
}

fn shell(command: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut cmd = crate::process::command("cmd");
        cmd.args(["/C", command]);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = crate::process::command("sh");
        cmd.args(["-c", command]);
        cmd
    }
}

fn kill(pid: u32) {
    #[cfg(windows)]
    let _ = crate::process::command("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .status();
    #[cfg(not(windows))]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
}

pub struct Captured {
    pub status: Option<std::process::ExitStatus>,
    pub stdout: String,
    pub stderr: String,
}

pub fn capture(mut cmd: std::process::Command) -> Result<Captured, String> {
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut out);
            }
            out
        })
    };
    let stdout = read(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = read(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        if start.elapsed() > TIMEOUT {
            kill(child.id());
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = |out: Vec<u8>| tail(String::from_utf8_lossy(&out).into_owned());
    Ok(Captured {
        status,
        stdout: text(stdout.join().unwrap_or_default()),
        stderr: text(stderr.join().unwrap_or_default()),
    })
}

fn tail(mut out: String) -> String {
    if out.len() > MAX_OUTPUT {
        let mut cut = out.len() - MAX_OUTPUT;
        while !out.is_char_boundary(cut) {
            cut += 1;
        }
        out = format!("[…]{}", &out[cut..]);
    }
    out
}

pub fn exec(dir: &Path, command: &str, env: &[(String, String)]) -> Result<(bool, String), String> {
    let mut cmd = shell(command);
    cmd.current_dir(dir).envs(env.iter().map(|(k, v)| (k, v)));
    let Captured {
        status,
        stdout,
        stderr,
    } = capture(cmd)?;
    let out = tail(stdout + &stderr);
    Ok(match status {
        None => (
            false,
            format!("{out}\nStopped after {} s.", TIMEOUT.as_secs()),
        ),
        Some(s) if s.success() => (true, out),
        Some(s) => (
            false,
            format!("{out}\nExit code {}.", s.code().unwrap_or(-1)),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_in_the_folder_and_reports_failures() {
        let dir = std::env::temp_dir();
        let (ok, out) = exec(&dir, "pwd && echo oops >&2", &[]).unwrap();
        assert!(ok);
        assert!(
            out.starts_with(dir.canonicalize().unwrap().to_str().unwrap()) || out.contains("tmp")
        );
        assert!(out.contains("oops"));
        let (ok, out) = exec(&dir, "echo half; exit 3", &[]).unwrap();
        assert!(!ok && out.contains("half") && out.ends_with("Exit code 3."));
    }

    #[test]
    fn keeps_the_end_of_a_long_output() {
        let (ok, out) = exec(&std::env::temp_dir(), "yes line | head -n 20000", &[]).unwrap();
        assert!(ok);
        assert!(out.starts_with("[…]") && out.len() <= MAX_OUTPUT + 8);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "runs the real claude"]
    async fn real_claude_sees_and_calls_a_tool_added_during_the_session() {
        use crate::claude::{self, Event, Options, Session};
        use crate::permissions::{Gate, mcp_config, serve_with_pilot};
        use serde_json::json;
        let (asks_tx, mut asks_rx) = tokio::sync::mpsc::unbounded_channel();
        let gate = Gate::new("token".into(), move |ask| asks_tx.send(ask).unwrap());
        let (calls_tx, mut calls_rx) = tokio::sync::mpsc::unbounded_channel::<Call>();
        let tools = Tools::new("token".into(), move |call| calls_tx.send(call).unwrap());
        let pilot = crate::pilot::Pilot::new("token".into(), |_| {});
        let port = serve_with_pilot(gate.clone(), pilot, tools.clone(), axum::Router::new())
            .await
            .unwrap();
        tokio::spawn(async move {
            while let Some(ask) = asks_rx.recv().await {
                let ask: crate::permissions::Ask = ask;
                gate.answer(
                    &ask.id,
                    json!({"behavior": "allow", "updatedInput": ask.input}),
                )
                .unwrap();
            }
        });
        let seen = Arc::new(Mutex::new(Vec::<Call>::new()));
        let (record, replier) = (seen.clone(), tools.clone());
        tokio::spawn(async move {
            while let Some(call) = calls_rx.recv().await {
                record.lock().unwrap().push(call.clone());
                replier
                    .reply(&call.id, true, "The secret word is pelican.".into())
                    .unwrap();
            }
        });
        let dir = std::env::temp_dir().join("squadron-extension-tools-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
            agent: None,
            advisor: None,
        };
        let config = mcp_config(port, "token", "agent-3", false);
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
        session.send("Reply with the word ready.").unwrap();
        rx.recv_timeout(Duration::from_secs(240)).unwrap();
        tools
            .set(vec![ExtensionTool {
                name: "probe_secret".into(),
                description: "Returns the secret word.".into(),
                input: None,
            }])
            .await;
        session.send("Call the tool mcp__squadron-extensions__probe_secret (load it with ToolSearch first if it is deferred) and tell me the secret word.").unwrap();
        let result = rx.recv_timeout(Duration::from_secs(240)).unwrap();
        println!(
            "RESULT {}",
            serde_json::from_str::<Value>(&result).unwrap()["result"]
        );
        session.stop().unwrap();
        let calls = seen.lock().unwrap().clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            (calls[0].agent.as_str(), calls[0].tool.as_str()),
            ("agent-3", "probe_secret")
        );
        assert!(result.contains("pelican"));
    }
}
