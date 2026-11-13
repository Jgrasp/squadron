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

pub static HELPER: OnceLock<(PathBuf, u16)> = OnceLock::new();

const VARIABLE: &str = "SQUADRON_ASKPASS";

#[derive(Clone, Debug, Serialize)]
pub struct Question {
    pub id: String,
    pub agent: String,
    pub prompt: String,
    pub command: String,
}

type Pending = Arc<Mutex<HashMap<String, (String, oneshot::Sender<bool>)>>>;

#[derive(Clone)]
pub struct Askpass {
    pending: Pending,
    next: Arc<AtomicU64>,
    notify: Arc<dyn Fn(Question) + Send + Sync>,
}

impl Askpass {
    pub fn new(notify: impl Fn(Question) + Send + Sync + 'static) -> Self {
        Self {
            pending: Default::default(),
            next: Default::default(),
            notify: Arc::new(notify),
        }
    }

    pub fn router(self) -> axum::Router {
        crate::keys::guarded(
            axum::Router::new().route("/", post(ask)).with_state(self),
            crate::keys::Route::Askpass,
        )
    }

    pub fn answer(&self, id: &str, yes: bool) -> Result<(), String> {
        let (_, tx) = self
            .pending
            .lock()
            .unwrap()
            .remove(id)
            .ok_or("This request no longer waits for an answer.")?;
        let _ = tx.send(yes);
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
    if !question(&prompt) {
        return (StatusCode::FORBIDDEN, String::new());
    }
    let agent = header("x-squadron-agent");
    let command = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        header("x-squadron-caller"),
    )
    .ok()
    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    .unwrap_or_default();
    let id = gate.next.fetch_add(1, Ordering::SeqCst).to_string();
    let (tx, rx) = oneshot::channel();
    gate.pending
        .lock()
        .unwrap()
        .insert(id.clone(), (agent.clone(), tx));
    (gate.notify)(Question {
        id,
        agent,
        prompt,
        command,
    });
    match rx.await {
        Ok(yes) => (StatusCode::OK, if yes { "yes" } else { "no" }.into()),
        Err(_) => (StatusCode::GONE, String::new()),
    }
}

fn question(prompt: &str) -> bool {
    prompt.contains("(yes/no")
}

fn passphrase_for(prompt: &str) -> Option<&str> {
    let rest = prompt.trim().strip_prefix("Enter passphrase for ")?;
    let rest = rest
        .strip_prefix("key ")
        .unwrap_or(rest)
        .trim_end_matches(':');
    Some(rest.trim_matches('\''))
}

fn refusal(prompt: &str) -> String {
    match passphrase_for(prompt) {
        Some(path) => format!(
            "Squadron does not hand passphrases to agents. Call the add_ssh_key tool of squadron-pilot with path {path}: \
the user types the passphrase in Squadron, which loads the key into the SSH agent. Then run the command again."
        ),
        None => "Squadron does not hand passwords to agents. For a command that needs sudo, use the run_sudo tool of squadron-pilot. \
For anything else, such as an SSH host that asks for a password or git over HTTPS, ask the user to run the command themselves in Squadron with !<command>."
            .into(),
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
    let Some((script, port)) = HELPER.get() else {
        return;
    };
    let token = crate::keys::KEYS.issue(agent, crate::keys::Route::Askpass);
    cmd.env(VARIABLE, format!("{port}:{token}:{agent}"))
        .env("SSH_ASKPASS", script)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env_remove("SUDO_ASKPASS")
        .env_remove("GIT_ASKPASS");
    if let Some(socket) = crate::ssh::OWN_AGENT.get() {
        cmd.env("SSH_AUTH_SOCK", socket);
    }
    if std::env::var_os("DISPLAY").is_none() {
        cmd.env("DISPLAY", ":0");
    }
}

pub fn client(prompt: &str) -> i32 {
    if !question(prompt) {
        eprintln!("{}", refusal(prompt));
        return 1;
    }
    let Some(answer) = std::env::var(VARIABLE)
        .ok()
        .and_then(|v| request(&v, prompt))
    else {
        return 1;
    };
    println!("{answer}");
    0
}

fn caller() -> String {
    #[cfg(target_os = "linux")]
    {
        let ppid = unsafe { libc::getppid() };
        if let Ok(raw) = std::fs::read(format!("/proc/{ppid}/cmdline")) {
            let line = String::from_utf8_lossy(&raw).replace('\0', " ");
            return line.trim().to_string();
        }
    }
    String::new()
}

fn request(variable: &str, prompt: &str) -> Option<String> {
    let mut parts = variable.splitn(3, ':');
    let (port, token, agent) = (parts.next()?, parts.next()?, parts.next()?);
    let caller = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, caller());
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port.parse::<u16>().ok()?)).ok()?;
    write!(
        stream,
        "POST /askpass HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Squadron-Token: {token}\r\nX-Squadron-Agent: {agent}\r\nX-Squadron-Caller: {caller}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{prompt}",
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

    const HOST: &str = "Are you sure you want to continue connecting (yes/no/[fingerprint])? ";

    #[tokio::test(flavor = "multi_thread")]
    async fn answers_only_yes_or_no() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let gate = Askpass::new(move |q| tx.send(q).unwrap());
        let token = crate::keys::KEYS.issue("agent-1", crate::keys::Route::Askpass);
        let other = crate::keys::KEYS.issue("agent-2", crate::keys::Route::Gate);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = axum::Router::new().nest_service("/askpass", gate.clone().router());
        tokio::spawn(async move { axum::serve(listener, router).await });
        let answering = gate.clone();
        tokio::spawn(async move {
            let q = rx.recv().await.unwrap();
            assert_eq!(q.agent, "agent-1");
            assert_eq!(q.prompt, HOST);
            assert_eq!(q.command, caller());
            answering.answer(&q.id, true).unwrap();
            let q = rx.recv().await.unwrap();
            answering.answer(&q.id, false).unwrap();
            assert!(rx.recv().await.is_none());
        });
        let good = format!("{port}:{token}:agent-2");
        let asked = tokio::task::spawn_blocking(move || {
            (
                request(&good, HOST),
                request(&good, HOST),
                request(&good, "[sudo] password for jgrasp: "),
            )
        });
        assert_eq!(
            asked.await.unwrap(),
            (Some("yes".into()), Some("no".into()), None)
        );
        let forged = format!("{port}:wrong:agent-1");
        let elsewhere = format!("{port}:{other}:agent-2");
        let refused = tokio::task::spawn_blocking(move || {
            (request(&forged, HOST), request(&elsewhere, HOST))
        });
        assert_eq!(refused.await.unwrap(), (None, None));
    }

    #[test]
    fn sends_the_agent_to_the_right_tool() {
        assert!(
            refusal("Enter passphrase for key '/home/u/.ssh/id_ed25519': ")
                .contains("add_ssh_key tool of squadron-pilot with path /home/u/.ssh/id_ed25519:")
        );
        assert!(
            refusal("Enter passphrase for /home/u/.ssh/id_rsa: ")
                .contains("path /home/u/.ssh/id_rsa:")
        );
        assert!(refusal("[sudo] password for jgrasp: ").contains("run_sudo"));
        assert!(refusal("git@host's password: ").contains("!<command>"));
        assert_eq!(client("Password for 'https://u@bitbucket.org': "), 1);
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
