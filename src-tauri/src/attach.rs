use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const IMAGE_LIMIT: u64 = 5 * 1024 * 1024;

fn media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

pub fn content(text: &str, files: &[PathBuf]) -> Result<Value, String> {
    if files.is_empty() {
        return Ok(json!(text));
    }
    let mut images = Vec::new();
    let mut others = Vec::new();
    for file in files {
        let size = fs::metadata(file)
            .map_err(|_| format!("{} cannot be read.", file.display()))?
            .len();
        match media_type(file) {
            Some(kind) if size <= IMAGE_LIMIT => {
                let data = STANDARD.encode(fs::read(file).map_err(|e| e.to_string())?);
                images.push(json!({"type": "image", "source": {"type": "base64", "media_type": kind, "data": data}}));
            }
            _ => others.push(format!("- {}", file.display())),
        }
    }
    let mut said = text.trim().to_string();
    if !others.is_empty() {
        said = format!(
            "{said}\n\nAttached files, read them where they are:\n{}",
            others.join("\n")
        )
        .trim()
        .to_string();
    }
    let mut blocks = Vec::new();
    if !said.is_empty() {
        blocks.push(json!({"type": "text", "text": said}));
    }
    blocks.extend(images);
    Ok(Value::Array(blocks))
}

pub fn save(dir: &Path, name: &str, data: &str) -> Result<PathBuf, String> {
    let bytes = STANDARD.decode(data).map_err(|e| e.to_string())?;
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    let folder = dir.join(uuid::Uuid::new_v4().simple().to_string());
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let file = folder.join(if name.trim_matches('.').is_empty() {
        "file".into()
    } else {
        name
    });
    fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(file)
}

fn folders(text: &str) -> HashSet<String> {
    text.match_indices("/attachments/")
        .filter_map(|(at, mark)| text.get(at + mark.len()..at + mark.len() + 32))
        .filter(|id| id.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_string)
        .collect()
}

fn referenced(agents: &Path) -> HashSet<String> {
    [agents.to_path_buf(), agents.join("archived")]
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .flat_map(|text| folders(&text))
        .collect()
}

pub fn forget(attachments: &Path, agents: &Path, gone: &str) {
    let kept = referenced(agents);
    for id in folders(gone).difference(&kept) {
        let _ = fs::remove_dir_all(attachments.join(id));
    }
}

pub fn sweep(attachments: &Path, agents: &Path, older: Duration) {
    let kept = referenced(agents);
    let Ok(dir) = fs::read_dir(attachments) else {
        return;
    };
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|at| SystemTime::now().duration_since(at).ok())
            .is_some_and(|age| age > older);
        if old && !kept.contains(&name) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

pub fn capture(dir: &Path, data: &str) -> Result<PathBuf, String> {
    let bytes = STANDARD.decode(data).map_err(|e| e.to_string())?;
    let ext = match bytes.get(..12) {
        Some([0x89, b'P', b'N', b'G', ..]) => "png",
        Some([0xff, 0xd8, 0xff, ..]) => "jpg",
        Some([b'G', b'I', b'F', b'8', ..]) => "gif",
        Some([b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P']) => "webp",
        _ => return Err("Not an image.".into()),
    };
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("Image too large.".into());
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4().simple()));
    fs::write(&file, bytes).map_err(|e| e.to_string())?;
    Ok(file)
}

pub fn expire(dir: &Path, older: Duration) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|at| SystemTime::now().duration_since(at).ok())
            .is_some_and(|age| age > older);
        if old {
            let path = entry.path();
            let _ = if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            };
        }
    }
}

pub fn preview(path: &Path) -> Result<String, String> {
    let kind = media_type(path).ok_or("Not an image.")?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("Image too large to preview.".into());
    }
    Ok(format!("data:{kind};base64,{}", STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_images_inline_and_other_files_by_path() {
        let dir = std::env::temp_dir().join(format!("squadron-attach-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let shot = save(
            &dir,
            "Capture d'écran.png",
            &STANDARD.encode([137u8, 80, 78, 71]),
        )
        .unwrap();
        assert!(shot.ends_with("Capture-d--cran.png"));
        let notes = dir.join("notes.pdf");
        fs::write(&notes, "%PDF").unwrap();

        assert_eq!(content("hello", &[]).unwrap(), json!("hello"));
        let sent = content("Look", &[shot.clone(), notes.clone()]).unwrap();
        assert_eq!(sent[0]["type"], "text");
        assert_eq!(
            sent[0]["text"],
            format!(
                "Look\n\nAttached files, read them where they are:\n- {}",
                notes.display()
            )
        );
        assert_eq!(sent[1]["source"]["media_type"], "image/png");
        assert_eq!(
            sent[1]["source"]["data"],
            STANDARD.encode([137u8, 80, 78, 71])
        );
        assert_eq!(
            content("", std::slice::from_ref(&shot))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            preview(&shot)
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert!(preview(&notes).is_err());
        assert!(content("x", &[dir.join("missing.png")]).is_err());
    }

    #[test]
    fn deletes_the_captures_no_agent_uses_anymore() {
        let root = std::env::temp_dir().join(format!("squadron-sweep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (store, agents) = (root.join("attachments"), root.join("agents"));
        fs::create_dir_all(agents.join("archived")).unwrap();
        let shot = |n: &str| save(&store, n, "AA==").unwrap();
        let (kept, archived, dropped, draft) =
            (shot("a.png"), shot("b.png"), shot("c.png"), shot("d.png"));
        let id = |p: &Path| {
            p.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        };
        let quoted = |p: &Path| format!("{{\"files\":[\"{}\"]}}", p.display());
        fs::write(agents.join("one.json"), quoted(&kept)).unwrap();
        fs::write(agents.join("archived/two.json"), quoted(&archived)).unwrap();
        assert_eq!(folders(&quoted(&kept)), HashSet::from([id(&kept)]));

        forget(
            &store,
            &agents,
            &format!("{}{}", quoted(&dropped), quoted(&kept)),
        );
        assert!(!dropped.exists() && kept.exists());

        let old = SystemTime::now() - Duration::from_secs(8 * 24 * 3600);
        for p in [&kept, &archived, &draft] {
            fs::File::open(p.parent().unwrap())
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        sweep(&store, &agents, Duration::from_secs(7 * 24 * 3600));
        assert!(kept.exists() && archived.exists() && !draft.exists());
    }

    #[test]
    fn keeps_a_tool_screenshot_by_its_real_format() {
        let dir = std::env::temp_dir().join(format!("squadron-capture-{}", std::process::id()));
        let png = STANDARD.encode([0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13]);
        let saved = capture(&dir, &png).unwrap();
        assert_eq!(saved.extension().unwrap(), "png");
        assert!(capture(&dir, &STANDARD.encode("#!/bin/sh echo hi")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn expires_the_week_old_screenshots_agents_saved() {
        let dir = std::env::temp_dir().join(format!("squadron-expire-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("run")).unwrap();
        let (old, fresh, folder) = (dir.join("old.png"), dir.join("new.png"), dir.join("run"));
        fs::write(&old, "x").unwrap();
        fs::write(&fresh, "x").unwrap();
        let week = SystemTime::now() - Duration::from_secs(8 * 24 * 3600);
        for p in [&old, &folder] {
            fs::File::open(p).unwrap().set_modified(week).unwrap();
        }
        expire(&dir, Duration::from_secs(7 * 24 * 3600));
        assert!(!old.exists() && !folder.exists() && fresh.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "runs the real claude"]
    fn real_claude_sees_an_attached_screenshot() {
        use crate::claude::{self, Event, Options, Session};
        let dir = std::env::temp_dir().join(format!("squadron-attach-real-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let shot = save(&dir, "screenshot.png", "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAIAAADYYG7QAAAASklEQVR4nO3OIQEAIQAAMSAI/UMRBv/mJC+2BJtn7/En63XgS6gIFaEiVISKUBEqQkWoCBWhIlSEilARKkJFqAgVoSJUhIpQESoX8TwBeNF89b0AAAAASUVORK5CYII=").unwrap();
        let options = Options {
            model: "haiku".into(),
            effort: "low".into(),
            mode: "default".into(),
        };
        let cmd = claude::command("claude", &dir, &options, None, false, None, None);
        let (tx, rx) = std::sync::mpsc::channel();
        let mut session = Session::start(
            cmd,
            move |line, event| {
                if let Event::Result { .. } = event {
                    tx.send(line.to_string()).unwrap();
                }
            },
            |_| {},
        )
        .unwrap();
        session.control(json!({"subtype": "initialize"})).unwrap();
        session
            .send_content(
                content(
                    "What colour fills this screenshot? Answer with one English word.",
                    &[shot],
                )
                .unwrap(),
            )
            .unwrap();
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(120))
            .unwrap();
        session.stop().unwrap();
        println!("RESULT {}", &result[..result.len().min(300)]);
        let said: Value = serde_json::from_str(&result).unwrap();
        assert!(
            said["result"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("red")
        );
    }
}
