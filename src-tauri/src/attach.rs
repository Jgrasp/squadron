use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

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
