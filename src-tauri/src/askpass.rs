use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::oneshot;

pub static HELPER: OnceLock<(PathBuf, u16, String)> = OnceLock::new();

const VARIABLE: &str = "SQUADRON_ASKPASS";

#[derive(Clone, Debug, Serialize)]
pub struct Question {
    pub id: String,
    pub agent: String,
    pub prompt: String,
}

type Pending = Arc<Mutex<HashMap<String, (String, oneshot::Sender<Option<String>>)>>>;

#[derive(Clone)]
pub struct Askpass {
    token: String,
    pending: Pending,
    next: Arc<AtomicU64>,
    notify: Arc<dyn Fn(Question) + Send + Sync>,
}

impl Askpass {
    pub fn new(token: String, notify: impl Fn(Question) + Send + Sync + 'static) -> Self {
        Self {
            token,
            pending: Default::default(),
            next: Default::default(),
            notify: Arc::new(notify),
        }
    }

    pub fn router(self) -> axum::Router {
        axum::Router::new().route("/", post(ask)).with_state(self)
    }

    pub fn answer(&self, id: &str, secret: Option<String>) -> Result<(), String> {
        let (_, tx) = self
            .pending
            .lock()
            .unwrap()
            .remove(id)
            .ok_or("This request no longer waits for an answer.")?;
        let _ = tx.send(secret);
        Ok(())
    }

    pub fn cancel(&self, agent: &str) {
        self.pending
            .lock()
            .unwrap()
            .retain(|_, (owner, _)| owner != agent);
    }
}

async fn ask(
    State(gate): State<Askpass>,
    headers: HeaderMap,
    prompt: String,
) -> (StatusCode, String) {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string()
    };
    if header("x-squadron-token") != gate.token {
        return (StatusCode::FORBIDDEN, String::new());
    }
    let agent = header("x-squadron-agent");
    let id = gate.next.fetch_add(1, Ordering::SeqCst).to_string();
    let (tx, rx) = oneshot::channel();
    gate.pending
        .lock()
        .unwrap()
        .insert(id.clone(), (agent.clone(), tx));
    (gate.notify)(Question { id, agent, prompt });
    match rx.await {
        Ok(Some(secret)) => (StatusCode::OK, secret),
        _ => (StatusCode::GONE, String::new()),
    }
}

fn quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

pub fn install(dir: &Path, exe: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let script = dir.join("askpass");
    std::fs::write(
        &script,
        format!("#!/bin/sh\nexec {} --askpass \"$1\"\n", quoted(exe)),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(script)
}

pub fn env(cmd: &mut Command, agent: &str) {
    let Some((script, port, token)) = HELPER.get() else {
        return;
    };
    cmd.env(VARIABLE, format!("{port}:{token}:{agent}"))
        .env("SUDO_ASKPASS", script)
        .env("SSH_ASKPASS", script)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("GIT_ASKPASS", script);
    if std::env::var_os("DISPLAY").is_none() {
        cmd.env("DISPLAY", ":0");
    }
}

pub fn client(prompt: &str) -> i32 {
    let Some(secret) = std::env::var(VARIABLE)
        .ok()
        .and_then(|v| request(&v, prompt))
    else {
        return 1;
    };
    println!("{secret}");
    0
}

fn request(variable: &str, prompt: &str) -> Option<String> {
    let mut parts = variable.splitn(3, ':');
    let (port, token, agent) = (parts.next()?, parts.next()?, parts.next()?);
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port.parse::<u16>().ok()?)).ok()?;
    write!(
        stream,
        "POST /askpass HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Squadron-Token: {token}\r\nX-Squadron-Agent: {agent}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{prompt}",
        prompt.len()
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let (head, body) = response.split_once("\r\n\r\n")?;
    (head.split(' ').nth(1) == Some("200")).then(|| body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn hands_the_answer_to_the_helper() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let gate = Askpass::new("secret-token".into(), move |q| tx.send(q).unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = axum::Router::new().nest_service("/askpass", gate.clone().router());
        tokio::spawn(async move { axum::serve(listener, router).await });
        let answering = gate.clone();
        tokio::spawn(async move {
            let q = rx.recv().await.unwrap();
            assert_eq!(q.agent, "agent-1");
            assert_eq!(q.prompt, "[sudo] password for jgrasp: ");
            answering.answer(&q.id, Some("hunter2".into())).unwrap();
            let q = rx.recv().await.unwrap();
            answering.answer(&q.id, None).unwrap();
        });
        let good = format!("{port}:secret-token:agent-1");
        let asked = tokio::task::spawn_blocking(move || {
            (
                request(&good, "[sudo] password for jgrasp: "),
                request(&good, "again"),
            )
        });
        assert_eq!(asked.await.unwrap(), (Some("hunter2".into()), None));
        let forged = format!("{port}:wrong:agent-1");
        let refused = tokio::task::spawn_blocking(move || request(&forged, "x"));
        assert_eq!(refused.await.unwrap(), None);
    }

    #[test]
    fn writes_a_helper_that_calls_squadron() {
        let dir = std::env::temp_dir().join(format!("squadron-askpass-{}", std::process::id()));
        let script = install(&dir, Path::new("/opt/it's/squadron")).unwrap();
        let text = std::fs::read_to_string(&script).unwrap();
        assert_eq!(
            text,
            "#!/bin/sh\nexec '/opt/it'\\''s/squadron' --askpass \"$1\"\n"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
