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

#[cfg(test)]
mod tests {
    use super::*;

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
}
