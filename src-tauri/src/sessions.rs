use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Found {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub here: bool,
    pub updated: u64,
    pub size: u64,
    pub app: bool,
    pub archived: bool,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub mode: Option<String>,
}

fn home() -> Option<PathBuf> {
    std::env::home_dir()
}

pub fn claude_dir() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| home().map(|h| h.join(".claude")))
}

fn encode(path: &Path) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

pub fn carry(from: &Path, to: &Path) {
    if let Some(root) = claude_dir().map(|d| d.join("projects")) {
        carry_in(&root, from, to);
    }
}

fn carry_in(root: &Path, from: &Path, to: &Path) {
    let (old, new) = (encode(from), encode(to));
    for dir in folders(root, from) {
        let name = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let target = root.join(format!("{new}{}", &name[old.len()..]));
        let moved = if target.exists() {
            merge(&dir, &target)
        } else {
            fs::rename(&dir, &target)
        };
        if let Err(e) = moved {
            log::warn!("{} not carried to {}: {e}", dir.display(), target.display());
        }
    }
}

fn merge(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type()?;
        if kind.is_dir() {
            merge(&source, &target)?;
        } else if kind.is_file() && !target.exists() {
            fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

fn valid(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

fn folders(root: &Path, project: &Path) -> Vec<PathBuf> {
    let name = encode(project);
    let Ok(dir) = fs::read_dir(root) else {
        return Vec::new();
    };
    dir.flatten()
        .filter(|entry| {
            let found = entry.file_name().to_string_lossy().into_owned();
            found == name || found.starts_with(&format!("{name}--"))
        })
        .map(|entry| entry.path())
        .collect()
}

fn app_sessions(config: &Path) -> HashMap<String, Value> {
    let mut found = HashMap::new();
    let Ok(accounts) = fs::read_dir(config.join("Claude/claude-code-sessions")) else {
        return found;
    };
    for org in accounts
        .flatten()
        .filter_map(|a| fs::read_dir(a.path()).ok())
        .flatten()
        .flatten()
    {
        let Ok(files) = fs::read_dir(org.path()) else {
            continue;
        };
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("local_") && name.ends_with(".json")) {
                continue;
            }
            let Some(meta) = fs::read_to_string(file.path())
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            else {
                continue;
            };
            if let Some(id) = meta["cliSessionId"].as_str() {
                found.insert(id.to_string(), meta.clone());
            }
        }
    }
    found
}

fn user_text(line: &Value) -> Option<String> {
    if line["type"] != "user" || line["isMeta"] == true || line["isSidechain"] == true {
        return None;
    }
    let content = &line["message"]["content"];
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|p| p["type"] == "text")
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let text = text.trim();
    let technical = [
        "<command-",
        "<local-command",
        "<task-notification",
        "<system-reminder",
        "<bash-",
        "<user-prompt-submit",
    ];
    (!text.is_empty() && !technical.iter().any(|tag| text.starts_with(tag))).then(|| {
        text.replace("</pasted_content>", "")
            .split_once('>')
            .filter(|_| text.starts_with("<pasted_content"))
            .map_or(text.to_string(), |(_, rest)| rest.trim().to_string())
    })
}

fn read(file: &Path) -> Option<(String, String)> {
    let text = String::from_utf8_lossy(&fs::read(file).ok()?).into_owned();
    let (mut first, mut named, mut cwd) = (None, None, None);
    for line in text.lines() {
        let wanted = (first.is_none() && line.contains("\"user\""))
            || line.contains("\"customTitle\"")
            || (named.is_none() && line.contains("\"summary\""))
            || (cwd.is_none() && line.contains("\"cwd\""));
        if !wanted {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if cwd.is_none() {
            cwd = value["cwd"].as_str().map(str::to_string);
        }
        if let Some(title) = value["customTitle"].as_str() {
            named = Some(title.to_string());
        } else if named.is_none() && value["type"] == "summary" {
            named = value["summary"].as_str().map(str::to_string);
        }
        if first.is_none() {
            first = user_text(&value);
        }
    }
    let first = first?;
    let title = named.unwrap_or_else(|| {
        first
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(80)
            .collect()
    });
    Some((title, cwd.unwrap_or_default()))
}

pub fn list(project: &Path) -> Vec<Found> {
    let Some(root) = claude_dir().map(|d| d.join("projects")) else {
        return Vec::new();
    };
    let config = if cfg!(target_os = "macos") {
        "Library/Application Support"
    } else {
        ".config"
    };
    let app = home()
        .map(|h| app_sessions(&h.join(config)))
        .unwrap_or_default();
    let mut found: Vec<Found> = folders(&root, project)
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let id = path.file_stem()?.to_string_lossy().into_owned();
            if path.extension()? != "jsonl" || !valid(&id) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            let (title, cwd) = read(&path)?;
            let app = app.get(&id);
            let text = |key: &str| app.and_then(|m| m[key].as_str()).map(str::to_string);
            let cwd = text("cwd").unwrap_or(cwd);
            Some(Found {
                here: !cwd.is_empty() && Path::new(&cwd).is_dir(),
                title: text("title").filter(|t| !t.is_empty()).unwrap_or(title),
                updated: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_millis() as u64),
                size: meta.len(),
                app: app.is_some(),
                archived: app.is_some_and(|m| m["isArchived"] == true),
                model: text("model"),
                effort: text("effort"),
                mode: text("permissionMode"),
                cwd,
                id,
            })
        })
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.updated));
    found
}

pub fn transcript(project: &Path, id: &str) -> Result<String, String> {
    if !valid(id) {
        return Err("Unknown conversation.".into());
    }
    let root = claude_dir()
        .ok_or("No Claude Code folder.")?
        .join("projects");
    folders(&root, project)
        .iter()
        .map(|dir| dir.join(format!("{id}.jsonl")))
        .find(|file| file.is_file())
        .ok_or_else(|| "This conversation is not in the project.".to_string())
        .and_then(|file| fs::read(file).map_err(|e| e.to_string()))
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn dimensions(b: &[u8]) -> Option<(u32, u32)> {
    let be = |i: usize| Some(u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as u32);
    let le = |i: usize| Some(u16::from_le_bytes([*b.get(i)?, *b.get(i + 1)?]) as u32);
    let le24 = |i: usize| {
        Some(u32::from_le_bytes([
            *b.get(i)?,
            *b.get(i + 1)?,
            *b.get(i + 2)?,
            0,
        ]))
    };
    if b.starts_with(b"\x89PNG") {
        let n = |i: usize| Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?));
        return Some((n(16)?, n(20)?));
    }
    if b.starts_with(b"GIF8") {
        return Some((le(6)?, le(8)?));
    }
    if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        return match b.get(12..16)? {
            b"VP8X" => Some((le24(24)? + 1, le24(27)? + 1)),
            b"VP8 " => Some((le(26)? & 0x3fff, le(28)? & 0x3fff)),
            b"VP8L" => {
                let bits = u32::from_le_bytes(b.get(21..25)?.try_into().ok()?);
                Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
            }
            _ => None,
        };
    }
    if !b.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut i = 2;
    while *b.get(i)? == 0xff {
        let marker = *b.get(i + 1)?;
        if marker == 0xff {
            i += 1;
            continue;
        }
        if (0xc0..=0xcf).contains(&marker) && ![0xc4, 0xc8, 0xcc].contains(&marker) {
            return Some((be(i + 7)?, be(i + 5)?));
        }
        i += 2 + be(i + 2)? as usize;
    }
    None
}

fn image_tokens(width: u32, height: u32) -> u64 {
    let scale = (2576.0 / width.max(height).max(1) as f64).min(1.0);
    ((width as f64 * scale) * (height as f64 * scale) / 750.0).ceil() as u64
}

pub fn image_excess(transcript: &str) -> HashMap<String, u64> {
    use base64::Engine;
    let mut names = HashMap::new();
    let mut excess: HashMap<String, u64> = HashMap::new();
    for line in transcript.lines() {
        let Ok(d) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if d["subtype"] == "compact_boundary" {
            excess.clear();
        }
        if d["isSidechain"] == true {
            continue;
        }
        let Some(content) = d["message"]["content"].as_array() else {
            continue;
        };
        for block in content {
            if block["type"] == "tool_use" {
                names.insert(
                    block["id"].as_str().unwrap_or_default().to_string(),
                    block["name"].as_str().unwrap_or_default().to_string(),
                );
            }
            if block["type"] != "tool_result" {
                continue;
            }
            let Some(name) = block["tool_use_id"].as_str().and_then(|id| names.get(id)) else {
                continue;
            };
            for image in block["content"].as_array().into_iter().flatten() {
                let Some(data) = image["source"]["data"]
                    .as_str()
                    .filter(|_| image["type"] == "image")
                else {
                    continue;
                };
                let Some((w, h)) = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .ok()
                    .as_deref()
                    .and_then(dimensions)
                else {
                    continue;
                };
                *excess.entry(name.clone()).or_default() +=
                    (data.len() as u64 / 4).saturating_sub(image_tokens(w, h));
            }
        }
    }
    excess
}

pub fn correct_usage(line: &str, excess: &HashMap<String, u64>) -> Option<String> {
    let mut d: Value = serde_json::from_str(line).ok()?;
    let r = &mut d["response"]["response"];
    let mut cut = 0;
    for tool in r["messageBreakdown"]["toolCallsByType"].as_array_mut()? {
        let Some(over) = tool["name"].as_str().and_then(|n| excess.get(n)) else {
            continue;
        };
        let tokens = tool["resultTokens"].as_u64().unwrap_or(0);
        let less = (*over).min(tokens);
        tool["resultTokens"] = (tokens - less).into();
        cut += less;
    }
    (cut > 0).then(|| d.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn carries_the_conversations_of_a_moved_project_and_its_worktrees() {
        let root = std::env::temp_dir().join(format!("squadron-carry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let main = root.join("-home-me-old-app");
        let worktree = root.join("-home-me-old-app--claude-worktrees-T70");
        let other = root.join("-home-me-old-app2");
        let landed = root.join("-home-me-new-app");
        for dir in [&main, &worktree, &other, &landed] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(main.join("a.jsonl"), "a").unwrap();
        fs::write(worktree.join("b.jsonl"), "b").unwrap();
        fs::write(other.join("c.jsonl"), "c").unwrap();
        fs::write(landed.join("d.jsonl"), "d").unwrap();
        carry_in(
            &root,
            Path::new("/home/me/old/app"),
            Path::new("/home/me/new/app"),
        );
        assert_eq!(fs::read_to_string(landed.join("a.jsonl")).unwrap(), "a");
        assert_eq!(fs::read_to_string(landed.join("d.jsonl")).unwrap(), "d");
        assert_eq!(
            fs::read_to_string(root.join("-home-me-new-app--claude-worktrees-T70/b.jsonl"))
                .unwrap(),
            "b"
        );
        assert!(!worktree.exists());
        assert!(other.join("c.jsonl").exists());
        assert!(!root.join("-home-me-new-app2").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_the_conversations_of_a_project_and_its_worktrees() {
        let root = std::env::temp_dir().join(format!("squadron-sessions-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let project = Path::new("/home/me/agence/orbit.app");
        let main = root.join("-home-me-agence-orbit-app");
        let worktree = root.join("-home-me-agence-orbit-app--worktrees-T70");
        let other = root.join("-home-me-agence-orbit-app2");
        for dir in [&main, &worktree, &other] {
            fs::create_dir_all(dir).unwrap();
        }
        let line = |v: Value| v.to_string();
        fs::write(
            main.join("aaaa-1.jsonl"),
            [
                line(serde_json::json!({"type": "user", "cwd": "/home/me/agence/orbit.app", "message": {"content": "<command-name>/clear</command-name>"}})),
                line(serde_json::json!({"type": "user", "message": {"content": [{"type": "text", "text": "Fix the export\nof segments"}]}})),
                line(serde_json::json!({"type": "custom-title", "customTitle": "Export fix"})),
            ]
            .join("\n"),
        )
        .unwrap();
        fs::write(
            worktree.join("bbbb-2.jsonl"),
            line(serde_json::json!({"type": "user", "message": {"content": "Plan T70"}})),
        )
        .unwrap();
        fs::write(
            main.join("cccc-3.jsonl"),
            line(serde_json::json!({"type": "summary", "summary": "No user message"})),
        )
        .unwrap();
        fs::write(
            other.join("dddd-4.jsonl"),
            line(serde_json::json!({"type": "user", "message": {"content": "Other project"}})),
        )
        .unwrap();

        let dirs = folders(&root, project);
        assert_eq!(dirs.len(), 2);
        let (title, cwd) = read(&main.join("aaaa-1.jsonl")).unwrap();
        assert_eq!(
            (title.as_str(), cwd.as_str()),
            ("Export fix", "/home/me/agence/orbit.app")
        );
        assert_eq!(read(&worktree.join("bbbb-2.jsonl")).unwrap().0, "Plan T70");
        let pasted = main.join("eeee-5.jsonl");
        fs::write(&pasted, line(serde_json::json!({"type": "user", "message": {"content": "<pasted_content id=\"x\">\nTask T5F: migrate the permissions\n</pasted_content>"}}))).unwrap();
        assert_eq!(
            read(&pasted).unwrap().0,
            "Task T5F: migrate the permissions"
        );
        assert_eq!(read(&main.join("cccc-3.jsonl")), None);
        assert!(!valid("../etc/passwd") && valid("271982a4-ba81-4f9b-ac49-2c87f1ef6e88"));
    }

    #[test]
    #[ignore = "reads your Claude Code conversations"]
    fn lists_the_conversations_of_bmc() {
        let found = list(std::path::Path::new(
            "/home/jgrasp/Projects/Sylius/mercure2",
        ));
        println!("{} conversations", found.len());
        for f in found.iter().take(6) {
            println!(
                "{} | {} | app={} archived={} | {} KB | here={}",
                f.id,
                f.title.chars().take(60).collect::<String>(),
                f.app,
                f.archived,
                f.size / 1024,
                f.here
            );
        }
        assert!(
            found
                .iter()
                .any(|f| f.id == "271982a4-ba81-4f9b-ac49-2c87f1ef6e88")
        );
        assert!(
            found
                .iter()
                .any(|f| f.id == "f8323701-6f0f-4443-a94b-ab9530cc9d68" && f.title.contains("T5F"))
        );
    }

    fn png(width: u32, height: u32, size: usize) -> String {
        use base64::Engine;
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        b.extend(width.to_be_bytes());
        b.extend(height.to_be_bytes());
        b.resize(size, 0);
        base64::engine::general_purpose::STANDARD.encode(b)
    }

    fn screenshot(id: &str, data: &str, sidechain: bool) -> String {
        let call = json!({"type": "assistant", "isSidechain": sidechain, "message": {"content": [{"type": "tool_use", "id": id, "name": "mcp__chrome-devtools__take_screenshot"}]}});
        let result = json!({"type": "user", "isSidechain": sidechain, "message": {"content": [{"type": "tool_result", "tool_use_id": id, "content": [{"type": "text", "text": "Took a screenshot"}, {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": data}}]}]}});
        format!("{call}\n{result}\n")
    }

    #[test]
    fn reads_the_size_of_png_gif_jpeg_and_webp_images() {
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 17, 8];
        jpeg.extend(982u16.to_be_bytes());
        jpeg.extend(2000u16.to_be_bytes());
        let mut gif = b"GIF89a".to_vec();
        gif.extend([0xd0, 0x07, 0xd6, 0x03]);
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\0\0\0\0\0\0\0\0".to_vec();
        webp.extend([0xcf, 0x07, 0, 0xd5, 0x03, 0]);
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD
            .decode(png(2000, 982, 40))
            .unwrap();
        for image in [png, gif, jpeg, webp] {
            assert_eq!(dimensions(&image), Some((2000, 982)));
        }
        assert_eq!(dimensions(b"not an image"), None);
    }

    #[test]
    fn an_image_costs_its_pixels_not_its_base64() {
        assert_eq!(image_tokens(2000, 982), 2619);
        assert_eq!(image_tokens(5152, 2000), 3435);
    }

    #[test]
    fn counts_only_the_images_still_in_the_main_context() {
        let shot = png(2000, 982, 240_000);
        let one = shot.len() as u64 / 4 - 2619;
        let before = screenshot("a", &shot, false);
        let boundary = json!({"type": "system", "subtype": "compact_boundary"});
        let after = format!(
            "{}{}",
            screenshot("b", &shot, false),
            screenshot("c", &shot, true)
        );
        let tool = "mcp__chrome-devtools__take_screenshot";
        assert_eq!(image_excess(&format!("{before}{after}"))[tool], 2 * one);
        assert_eq!(
            image_excess(&format!("{before}{boundary}\n{after}"))[tool],
            one
        );
    }

    #[test]
    fn takes_the_image_excess_out_of_the_context_usage() {
        let line = json!({"type": "control_response", "response": {"response": {
            "totalTokens": 400_000, "maxTokens": 1_000_000, "percentage": 40,
            "categories": [{"id": "system_prompt", "tokens": 10_000}, {"id": "messages", "tokens": 390_000}, {"id": "free_space", "tokens": 567_000}],
            "messageBreakdown": {"toolCallsByType": [
                {"name": "Read", "callTokens": 100, "resultTokens": 5_000},
                {"name": "mcp__chrome-devtools__take_screenshot", "callTokens": 200, "resultTokens": 300_000}
            ]}
        }}});
        let excess = HashMap::from([
            ("mcp__chrome-devtools__take_screenshot".to_string(), 290_000),
            ("Gone".to_string(), 9),
        ]);
        let fixed: Value =
            serde_json::from_str(&correct_usage(&line.to_string(), &excess).unwrap()).unwrap();
        let r = &fixed["response"]["response"];
        assert_eq!(r["totalTokens"], 400_000);
        assert_eq!(r["categories"][1]["tokens"], 390_000);
        assert_eq!(
            r["messageBreakdown"]["toolCallsByType"][0]["resultTokens"],
            5_000
        );
        assert_eq!(
            r["messageBreakdown"]["toolCallsByType"][1]["resultTokens"],
            10_000
        );
        assert_eq!(correct_usage(&line.to_string(), &HashMap::new()), None);
    }
}
