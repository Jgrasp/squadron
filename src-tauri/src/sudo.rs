use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub const WRONG: &str = "Wrong password.";

#[derive(Default)]
pub struct Sudo {
    next: AtomicU32,
    running: Mutex<HashMap<u32, u32>>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Exited(Option<i32>),
    Wrong,
}

fn spawn(
    program: &Path,
    dir: &Path,
    line: &str,
    password: Option<&str>,
    mark: &str,
) -> std::io::Result<Child> {
    let mut cmd = Command::new(program);
    match password {
        Some(_) => cmd.args(["-k", "-S", "-p", mark]),
        None => cmd.args(["-k", "-n"]),
    };
    cmd.args(["--", "sh", "-c", line])
        .current_dir(dir)
        .env_remove("LD_LIBRARY_PATH")
        .stdin(if password.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    cmd.spawn()
}

fn pump(mut from: impl Read, out: impl Fn(String)) {
    let mut buf = [0u8; 8192];
    let mut pending = Vec::new();
    while let Ok(n @ 1..) = from.read(&mut buf) {
        pending.extend_from_slice(&buf[..n]);
        let complete = match std::str::from_utf8(&pending) {
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            _ => pending.len(),
        };
        out(String::from_utf8_lossy(&pending[..complete]).into_owned());
        pending.drain(..complete);
    }
}

fn answer(
    mut stderr: impl Read,
    mut stdin: Option<ChildStdin>,
    password: Option<String>,
    mark: &str,
    wrong: &Mutex<bool>,
    child: &Mutex<Child>,
    out: impl Fn(String),
) {
    let mut buf = [0u8; 4096];
    let mut seen = Vec::new();
    let mut prompts = 0;
    while let Ok(n @ 1..) = stderr.read(&mut buf) {
        seen.extend_from_slice(&buf[..n]);
        while let Some(at) = seen.windows(mark.len()).position(|w| w == mark.as_bytes()) {
            let before: Vec<u8> = seen.drain(..at + mark.len()).take(at).collect();
            if !before.is_empty() {
                out(String::from_utf8_lossy(&before).into_owned());
            }
            prompts += 1;
            match (prompts, stdin.take(), &password) {
                (1, Some(mut pipe), Some(secret)) => {
                    let _ = pipe.write_all(format!("{secret}\n").as_bytes());
                }
                _ => {
                    *wrong.lock().unwrap() = true;
                    let _ = child.lock().unwrap().kill();
                    return;
                }
            }
        }
        let keep = seen.len().saturating_sub(mark.len() - 1);
        if keep > 0 && prompts > 0 {
            out(String::from_utf8_lossy(&seen[..keep]).into_owned());
            seen.drain(..keep);
        }
    }
    if !seen.is_empty() {
        out(String::from_utf8_lossy(&seen).into_owned());
    }
}

pub fn run(
    program: &Path,
    dir: &Path,
    line: &str,
    password: Option<String>,
    out: impl Fn(String) + Send + Sync + 'static,
) -> std::io::Result<(u32, std::thread::JoinHandle<Outcome>)> {
    let mark = format!("[squadron:{}]", uuid::Uuid::new_v4().simple());
    let mut child = spawn(program, dir, line, password.as_deref(), &mark)?;
    let pid = child.id();
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let child = Arc::new(Mutex::new(child));
    let wrong = Arc::new(Mutex::new(false));
    let out = Arc::new(out);
    let handle = std::thread::spawn({
        let child = child.clone();
        move || {
            let printed = std::thread::spawn({
                let out = out.clone();
                move || pump(stdout, |text| out(text))
            });
            let quiet = password.is_none();
            if quiet {
                pump(stderr, |text| out(text));
            } else {
                answer(stderr, stdin, password, &mark, &wrong, &child, |text| {
                    out(text)
                });
            }
            let _ = printed.join();
            let status = child.lock().unwrap().wait().ok().and_then(|s| s.code());
            if *wrong.lock().unwrap() {
                Outcome::Wrong
            } else {
                Outcome::Exited(status)
            }
        }
    });
    Ok((pid, handle))
}

pub fn program() -> PathBuf {
    PathBuf::from("sudo")
}

pub fn needs_password(program: &Path) -> Result<bool, String> {
    let status = Command::new(program)
        .args(["-k", "-n", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "sudo is not installed on this computer.".to_string())?;
    Ok(!status.success())
}

pub fn check(program: &Path, password: &str) -> Result<bool, String> {
    let (_, handle) = run(
        program,
        Path::new("/"),
        "true",
        Some(password.into()),
        |_| {},
    )
    .map_err(|e| e.to_string())?;
    Ok(handle.join().map_err(|_| "sudo stopped unexpectedly.")? == Outcome::Exited(Some(0)))
}

#[tauri::command]
pub async fn sudo_needs_password() -> Result<serde_json::Value, String> {
    if cfg!(windows) {
        return Err("sudo is not available on Windows.".into());
    }
    let needed = tauri::async_runtime::spawn_blocking(|| needs_password(&program()))
        .await
        .map_err(|e| e.to_string())??;
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default();
    Ok(serde_json::json!({ "needed": needed, "user": user }))
}

#[tauri::command]
pub async fn sudo_run(
    app: AppHandle,
    sudo: State<'_, Sudo>,
    dir: PathBuf,
    command: String,
    password: Option<String>,
) -> Result<u32, String> {
    if let Some(secret) = password.clone() {
        let fine = tauri::async_runtime::spawn_blocking(move || check(&program(), &secret))
            .await
            .map_err(|e| e.to_string())??;
        if !fine {
            return Err(WRONG.into());
        }
    }
    let id = sudo.next.fetch_add(1, Ordering::Relaxed) + 1;
    let out = app.clone();
    let (pid, handle) = run(&program(), &dir, &command, password, move |data| {
        let _ = out.emit("sudo-out", serde_json::json!({ "id": id, "data": data }));
    })
    .map_err(|e| e.to_string())?;
    sudo.running.lock().unwrap().insert(id, pid);
    std::thread::spawn(move || {
        let code = match handle.join() {
            Ok(Outcome::Exited(code)) => code,
            _ => None,
        };
        tauri::Manager::state::<Sudo>(&app)
            .running
            .lock()
            .unwrap()
            .remove(&id);
        let _ = app.emit("sudo-exit", serde_json::json!({ "id": id, "code": code }));
    });
    Ok(id)
}

#[tauri::command]
pub fn sudo_stop(sudo: State<'_, Sudo>, id: u32) {
    #[cfg(unix)]
    if let Some(pid) = sudo.running.lock().unwrap().get(&id) {
        unsafe { libc::kill(-(*pid as i32), libc::SIGTERM) };
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn fake(dir: &Path, asks: bool) -> PathBuf {
        let path = dir.join(if asks { "sudo" } else { "sudo-nopasswd" });
        let ask = r#"
if [ -n "$mark" ]; then
  while :; do
    printf '%s' "$mark" >&2
    IFS= read -r pw || exit 1
    [ "$pw" = "hunter2" ] && break
    echo "Sorry, try again." >&2
  done
fi"#;
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\nmark=\"\"\nwhile [ $# -gt 0 ]; do case \"$1\" in -p) mark=\"$2\"; shift 2;; --) shift; break;; *) shift;; esac; done\n{}\nexec \"$@\"\n",
                if asks { ask } else { "" }
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn collect(program: &Path, line: &str, password: Option<&str>) -> (Outcome, String) {
        let text = Arc::new(Mutex::new(String::new()));
        let sink = text.clone();
        let (_, handle) = run(
            program,
            Path::new("/"),
            line,
            password.map(String::from),
            move |t| sink.lock().unwrap().push_str(&t),
        )
        .unwrap();
        let outcome = handle.join().unwrap();
        let text = text.lock().unwrap().clone();
        (outcome, text)
    }

    #[test]
    fn writes_the_password_only_to_sudo() {
        let dir = std::env::temp_dir().join(format!("squadron-sudo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sudo = fake(&dir, true);
        let (outcome, text) = collect(&sudo, "echo out; echo err >&2; cat", Some("hunter2"));
        assert_eq!(outcome, Outcome::Exited(Some(0)));
        assert!(text.contains("out") && text.contains("err"), "{text}");
        assert!(
            !text.contains("hunter2") && !text.contains("squadron:"),
            "{text}"
        );
        let (outcome, text) = collect(&sudo, "exit 3", Some("hunter2"));
        assert_eq!((outcome, text.as_str()), (Outcome::Exited(Some(3)), ""));
        assert_eq!(collect(&sudo, "echo never", Some("nope")).0, Outcome::Wrong);
        assert!(check(&sudo, "hunter2").unwrap());
        assert!(!check(&sudo, "nope").unwrap());
        let open = fake(&dir, false);
        let (outcome, text) = collect(&open, "echo done", Some("hunter2"));
        assert_eq!(
            (outcome, text.as_str()),
            (Outcome::Exited(Some(0)), "done\n")
        );
        let (outcome, text) = collect(&open, "cat; echo done", None);
        assert_eq!(
            (outcome, text.as_str()),
            (Outcome::Exited(Some(0)), "done\n")
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
