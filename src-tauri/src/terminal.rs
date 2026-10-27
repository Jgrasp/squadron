use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

struct Term {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Default)]
pub struct Terminals {
    next: AtomicU32,
    open: Mutex<HashMap<u32, Term>>,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(2),
        cols: cols.max(10),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn decode(pending: &mut Vec<u8>) -> String {
    let complete = match std::str::from_utf8(pending) {
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        _ => pending.len(),
    };
    let text = String::from_utf8_lossy(&pending[..complete]).into_owned();
    pending.drain(..complete);
    text
}

impl Terminals {
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &self,
        dir: &Path,
        shell: Option<&str>,
        command: Option<&str>,
        cols: u16,
        rows: u16,
        on_output: impl Fn(u32, String) + Send + 'static,
        on_exit: impl FnOnce(u32) + Send + 'static,
    ) -> Result<u32, String> {
        let pair = native_pty_system()
            .openpty(size(cols, rows))
            .map_err(|e| e.to_string())?;
        let mut cmd = match shell {
            Some(program) => CommandBuilder::new(program),
            None if command.is_some() => CommandBuilder::new("/bin/sh"),
            None => CommandBuilder::new_default_prog(),
        };
        if let Some(line) = command {
            cmd.args(["-ic", line]);
        }
        cmd.cwd(dir);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.open.lock().unwrap().insert(
            id,
            Term {
                master: pair.master,
                writer,
                child,
            },
        );
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut pending = Vec::new();
            while let Ok(n @ 1..) = reader.read(&mut buf) {
                pending.extend_from_slice(&buf[..n]);
                let text = decode(&mut pending);
                if !text.is_empty() {
                    on_output(id, text);
                }
            }
            on_exit(id);
        });
        Ok(id)
    }

    pub fn write(&self, id: u32, data: &str) -> Result<(), String> {
        let mut open = self.open.lock().unwrap();
        let term = open.get_mut(&id).ok_or("Terminal closed.")?;
        term.writer
            .write_all(data.as_bytes())
            .and_then(|_| term.writer.flush())
            .map_err(|e| e.to_string())
    }

    pub fn resize(&self, id: u32, cols: u16, rows: u16) -> Result<(), String> {
        let open = self.open.lock().unwrap();
        let term = open.get(&id).ok_or("Terminal closed.")?;
        term.master
            .resize(size(cols, rows))
            .map_err(|e| e.to_string())
    }

    pub fn finish(&self, id: u32) -> Option<u32> {
        let mut term = self.open.lock().unwrap().remove(&id)?;
        term.child.wait().ok().map(|status| status.exit_code())
    }

    pub fn close(&self, id: u32) {
        if let Some(mut term) = self.open.lock().unwrap().remove(&id) {
            let _ = term.child.kill();
            let _ = term.child.wait();
        }
    }

    pub fn close_all(&self) {
        let ids: Vec<u32> = self.open.lock().unwrap().keys().copied().collect();
        for id in ids {
            self.close(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn keeps_split_utf8_for_the_next_read() {
        let mut pending = "é".as_bytes()[..1].to_vec();
        assert_eq!(decode(&mut pending), "");
        pending.extend_from_slice(&"é".as_bytes()[1..]);
        pending.extend_from_slice(b"!");
        assert_eq!(decode(&mut pending), "é!");
        assert!(pending.is_empty());
    }

    #[test]
    fn runs_a_shell_in_the_project_folder() {
        let dir = std::env::temp_dir().join(format!("squadron-term-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let terminals = Terminals::default();
        let (out, received) = mpsc::channel();
        let (done, exited) = mpsc::channel();
        let id = terminals
            .open(
                &dir,
                Some("/bin/sh"),
                None,
                80,
                24,
                move |_, text| {
                    let _ = out.send(text);
                },
                move |id| {
                    let _ = done.send(id);
                },
            )
            .unwrap();
        terminals.resize(id, 100, 30).unwrap();
        terminals.write(id, "pwd; echo squad$((1+1))\n").unwrap();
        let mut seen = String::new();
        while !seen.contains("squad2") {
            seen += &received.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert!(seen.contains(dir.file_name().unwrap().to_str().unwrap()));
        terminals.write(id, "exit\n").unwrap();
        assert_eq!(exited.recv_timeout(Duration::from_secs(5)).unwrap(), id);
        terminals.close(id);
        assert!(terminals.write(id, "x").is_err());
    }

    #[test]
    fn runs_one_command_and_gives_its_exit_code() {
        let dir = std::env::temp_dir();
        let terminals = std::sync::Arc::new(Terminals::default());
        let (out, received) = mpsc::channel();
        let (done, exited) = mpsc::channel();
        let finished = terminals.clone();
        terminals
            .open(
                &dir,
                Some("/bin/sh"),
                Some("echo squad$((2+1)); exit 3"),
                80,
                24,
                move |_, text| {
                    let _ = out.send(text);
                },
                move |id| {
                    let _ = done.send(finished.finish(id));
                },
            )
            .unwrap();
        assert_eq!(
            exited.recv_timeout(Duration::from_secs(5)).unwrap(),
            Some(3)
        );
        let seen: String = received.try_iter().collect();
        assert!(seen.contains("squad3"));
    }
}
