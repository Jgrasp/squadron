use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Definition {
    pub path: String,
    pub source: String,
    pub plugin: Option<String>,
    pub description: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub tools: Option<Vec<String>>,
    pub skills: Vec<String>,
    pub body: String,
}

pub fn find(project: &Path, name: &str) -> Option<Definition> {
    let claude = crate::sessions::claude_dir();
    let (plugin, short) = match name.split_once(':') {
        Some((plugin, rest)) => (Some(plugin), rest.rsplit(':').next().unwrap_or(rest)),
        None => (None, name),
    };
    let dirs: Vec<(PathBuf, &str)> = match plugin {
        Some(plugin) => plugin_dirs(claude.as_deref()?, plugin)
            .into_iter()
            .map(|d| (d.join("agents"), "plugin"))
            .collect(),
        None => [
            Some((project.join(".claude/agents"), "project")),
            claude.map(|c| (c.join("agents"), "user")),
        ]
        .into_iter()
        .flatten()
        .collect(),
    };
    for (dir, source) in dirs {
        let mut files = Vec::new();
        markdown(&dir, 3, &mut files);
        for file in files {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            let (head, body) = split(&text);
            let fields = fields(head);
            let named = field(&fields, "name")
                .map(str::to_string)
                .or_else(|| file.file_stem().map(|s| s.to_string_lossy().into_owned()));
            if named.as_deref() != Some(short) {
                continue;
            }
            return Some(Definition {
                path: file.to_string_lossy().into_owned(),
                source: source.into(),
                plugin: plugin.map(str::to_string),
                description: field(&fields, "description").unwrap_or_default().into(),
                model: field(&fields, "model")
                    .filter(|m| *m != "inherit")
                    .map(str::to_string),
                effort: field(&fields, "effort").map(str::to_string),
                tools: list(&fields, "tools"),
                skills: list(&fields, "skills").unwrap_or_default(),
                body: body.trim().into(),
            });
        }
    }
    None
}

fn plugin_dirs(claude: &Path, plugin: &str) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(claude.join("plugins/installed_plugins.json")) else {
        return Vec::new();
    };
    let Ok(installed) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let prefix = format!("{plugin}@");
    installed["plugins"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| key.starts_with(&prefix))
        .flat_map(|(_, installs)| installs.as_array().cloned().unwrap_or_default())
        .filter_map(|install| install["installPath"].as_str().map(PathBuf::from))
        .collect()
}

fn markdown(dir: &Path, depth: u8, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() && depth > 0 {
            markdown(&path, depth - 1, files);
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
}

fn split(text: &str) -> (&str, &str) {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return ("", text);
    };
    let mut at = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (&rest[..at], &rest[at + line.len()..]);
        }
        at += line.len();
    }
    ("", text)
}

fn fields(head: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in head.lines() {
        if line.starts_with([' ', '\t', '-']) {
            if let Some((_, value)) = fields.last_mut() {
                let line = line.trim();
                match line.strip_prefix("- ") {
                    Some(item) => {
                        value.push('\n');
                        value.push_str(item);
                    }
                    None => {
                        if !value.is_empty() {
                            value.push(' ');
                        }
                        value.push_str(line);
                    }
                }
            }
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            let value = if matches!(value, "|" | ">" | "|-" | ">-") {
                ""
            } else {
                value
            };
            fields.push((key.trim().to_string(), unquote(value).to_string()));
        }
    }
    fields
}

fn field<'a>(fields: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

fn list(fields: &[(String, String)], key: &str) -> Option<Vec<String>> {
    let value = field(fields, key)?;
    let value = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    Some(
        value
            .split([',', '\n'])
            .map(|item| unquote(item.trim()).to_string())
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn unquote(value: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_agent_of_the_project() {
        let dir = std::env::temp_dir().join(format!("squadron-def-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(dir.join(".claude/agents/team")).unwrap();
        fs::write(
            dir.join(".claude/agents/team/arch.md"),
            "---\nname: architect\ndescription: \"Plans the work.\"\nmodel: opus\neffort: high\ntools: Read, Grep, mcp__codegraph__explore\nskills:\n  - codegraph:explorer\n---\n\n# Role\n\nYou plan.\n",
        )
        .unwrap();
        fs::write(
            dir.join(".claude/agents/reviewer.md"),
            "---\ndescription: >\n  Reviews\n  the diff.\nmodel: inherit\ntools: [Read, \"Bash\"]\n---\nReview.",
        )
        .unwrap();
        let found = find(&dir, "architect").unwrap();
        assert_eq!(
            found,
            Definition {
                path: dir
                    .join(".claude/agents/team/arch.md")
                    .to_string_lossy()
                    .into(),
                source: "project".into(),
                plugin: None,
                description: "Plans the work.".into(),
                model: Some("opus".into()),
                effort: Some("high".into()),
                tools: Some(vec![
                    "Read".into(),
                    "Grep".into(),
                    "mcp__codegraph__explore".into()
                ]),
                skills: vec!["codegraph:explorer".into()],
                body: "# Role\n\nYou plan.".into(),
            }
        );
        let reviewer = find(&dir, "reviewer").unwrap();
        assert_eq!(reviewer.description, "Reviews the diff.");
        assert_eq!(reviewer.model, None);
        assert_eq!(reviewer.tools, Some(vec!["Read".into(), "Bash".into()]));
        assert!(find(&dir, "Explore").is_none());
        fs::remove_dir_all(dir).unwrap();
    }
}
