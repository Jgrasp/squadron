use ssh_encoding::Encode;
use ssh_key::PrivateKey;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const ADD_IDENTITY: u8 = 17;
const SUCCESS: u8 = 6;

pub static OWN_AGENT: OnceLock<PathBuf> = OnceLock::new();

pub const WRONG: &str = "Wrong passphrase.";

pub enum Key {
    Plain,
    Encrypted(Box<PrivateKey>),
}

pub fn read(path: &Path) -> Result<Key, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let key = PrivateKey::from_openssh(&text).map_err(|_| {
        format!(
            "{} is not a private key in the OpenSSH format, which Squadron reads. The user can load it with !ssh-add {}",
            path.display(),
            path.display()
        )
    })?;
    Ok(if key.is_encrypted() {
        Key::Encrypted(Box::new(key))
    } else {
        Key::Plain
    })
}

pub fn unlock(key: &PrivateKey, passphrase: &str) -> Option<PrivateKey> {
    key.decrypt(passphrase).ok()
}

fn message(key: &PrivateKey) -> Result<Vec<u8>, String> {
    let mut body = vec![ADD_IDENTITY];
    key.key_data()
        .encode(&mut body)
        .map_err(|e| e.to_string())?;
    key.comment().encode(&mut body).map_err(|e| e.to_string())?;
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend(body);
    Ok(out)
}

#[cfg(unix)]
pub fn add(socket: &Path, key: &PrivateKey) -> Result<(), String> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket)
        .map_err(|e| format!("Cannot reach the SSH agent at {}: {e}", socket.display()))?;
    stream
        .write_all(&message(key)?)
        .map_err(|e| e.to_string())?;
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).map_err(|e| e.to_string())?;
    let mut reply = vec![0u8; u32::from_be_bytes(len) as usize];
    stream.read_exact(&mut reply).map_err(|e| e.to_string())?;
    match reply.first() {
        Some(&SUCCESS) => Ok(()),
        _ => Err("The SSH agent refused the key. Some agents, such as 1Password's, do not accept keys added from outside: add it in the agent's own app.".into()),
    }
}

#[cfg(not(unix))]
pub fn add(_: &Path, _: &PrivateKey) -> Result<(), String> {
    Err("Squadron loads SSH keys on Linux and macOS only.".into())
}

pub fn socket() -> Option<PathBuf> {
    OWN_AGENT
        .get()
        .cloned()
        .or_else(|| std::env::var_os("SSH_AUTH_SOCK").map(PathBuf::from))
}

#[cfg(unix)]
pub fn start_own_agent(dir: &Path) {
    if std::env::var_os("SSH_AUTH_SOCK").is_some() {
        return;
    }
    let socket = dir.join("ssh-agent.sock");
    if std::os::unix::net::UnixStream::connect(&socket).is_err() {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::remove_file(&socket);
        let started = std::process::Command::new("ssh-agent")
            .arg("-a")
            .arg(&socket)
            .stdin(std::process::Stdio::null())
            .output();
        if !started.is_ok_and(|o| o.status.success()) {
            log::warn!("ssh-agent could not start");
            return;
        }
    }
    let _ = OWN_AGENT.set(socket);
}

#[cfg(not(unix))]
pub fn start_own_agent(_: &Path) {}

fn expand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

#[tauri::command]
pub fn ssh_key_state(path: String) -> Result<&'static str, String> {
    Ok(match read(&expand(&path))? {
        Key::Plain => "plain",
        Key::Encrypted(_) => "encrypted",
    })
}

#[tauri::command]
pub async fn ssh_add_key(path: String, passphrase: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Key::Encrypted(key) = read(&expand(&path))? else {
            return Ok(());
        };
        let key = unlock(&key, &passphrase).ok_or(WRONG)?;
        let socket = socket().ok_or("No SSH agent is running.")?;
        add(&socket, &key)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn loads_a_key_with_a_passphrase_into_an_agent() {
        let dir = std::env::temp_dir().join(format!("squadron-ssh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("agent.sock");
        let mut agent = Command::new("ssh-agent")
            .arg("-D")
            .arg("-a")
            .arg(&socket)
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        while !socket.exists() {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        for kind in ["ed25519", "rsa", "ecdsa"] {
            let file = dir.join(format!("id_{kind}"));
            let made = Command::new("ssh-keygen")
                .args([
                    "-q",
                    "-t",
                    kind,
                    "-N",
                    "correct horse",
                    "-C",
                    &format!("test-{kind}"),
                    "-f",
                ])
                .arg(&file)
                .status()
                .unwrap();
            assert!(made.success());
            let Key::Encrypted(key) = read(&file).unwrap() else {
                panic!("the key should be encrypted")
            };
            assert!(unlock(&key, "wrong").is_none());
            add(&socket, &unlock(&key, "correct horse").unwrap()).unwrap();
        }
        let listed = Command::new("ssh-add")
            .arg("-l")
            .env("SSH_AUTH_SOCK", &socket)
            .output()
            .unwrap();
        let _ = agent.kill();
        let _ = agent.wait();
        let _ = std::fs::remove_dir_all(&dir);
        let listed = String::from_utf8_lossy(&listed.stdout);
        for kind in ["ED25519", "RSA", "ECDSA"] {
            assert!(
                listed.contains(&format!("test-{} ({kind})", kind.to_lowercase())),
                "{listed}"
            );
        }
    }
}
