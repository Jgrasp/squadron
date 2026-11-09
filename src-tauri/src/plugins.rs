use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const VERBS: [&str; 6] = [
    "install",
    "uninstall",
    "update",
    "enable",
    "disable",
    "marketplace",
];

pub fn run(program: &str, args: &[String], dir: Option<&Path>) -> Result<String, String> {
    if !args
        .first()
        .is_some_and(|verb| VERBS.contains(&verb.as_str()))
    {
        return Err("Not a plugin command.".into());
    }
    output(program, args, dir)
}

fn output(
    program: &str,
    args: &[impl AsRef<std::ffi::OsStr>],
    dir: Option<&Path>,
) -> Result<String, String> {
    let output = crate::process::command(program)
        .current_dir(
            dir.map(Path::to_path_buf)
                .unwrap_or_else(std::env::temp_dir),
        )
        .arg("plugin")
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() { text } else { err });
    }
    Ok(text)
}

fn json(program: &str, args: &[&str]) -> Result<Value, String> {
    serde_json::from_str(&output(program, args, None)?).map_err(|e| e.to_string())
}

pub fn installed(program: &str) -> Result<Value, String> {
    let mut list = json(program, &["list", "--json"])?;
    for plugin in list.as_array_mut().into_iter().flatten() {
        let manifest = plugin["installPath"]
            .as_str()
            .map(|dir| Path::new(dir).join(".claude-plugin/plugin.json"))
            .and_then(|file| fs::read_to_string(file).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or_default();
        plugin["description"] = manifest["description"].clone();
        plugin["name"] = manifest["name"].clone();
    }
    Ok(list)
}

pub fn catalog(program: &str) -> Result<Value, String> {
    let all = json(program, &["list", "--json", "--available"])?;
    Ok(Value::Array(
        all["available"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| {
                serde_json::json!({
                    "id": p["pluginId"],
                    "name": p["name"],
                    "description": p["description"],
                    "marketplace": p["marketplaceName"],
                    "installs": p["installCount"],
                })
            })
            .collect(),
    ))
}

pub fn marketplaces(program: &str) -> Result<Value, String> {
    json(program, &["marketplace", "list", "--json"])
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Component {
    pub kind: String,
    pub name: String,
    pub always: u32,
    pub invoke: u32,
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Details {
    pub always: u32,
    pub components: Vec<Component>,
}

pub fn details(program: &str, id: &str) -> Result<Details, String> {
    Ok(parse_details(&output(program, &["details", id], None)?))
}

fn tokens(text: &str) -> u32 {
    let text = text.trim().trim_start_matches('~');
    match text.strip_suffix('k') {
        Some(k) => (k.parse::<f32>().unwrap_or(0.0) * 1000.0).round() as u32,
        None => text.parse().unwrap_or(0),
    }
}

pub fn parse_details(text: &str) -> Details {
    let mut details = Details::default();
    let mut kinds: BTreeMap<String, String> = BTreeMap::new();
    let mut table = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Always-on:") {
            details.always = tokens(rest.split_whitespace().next().unwrap_or(""));
            continue;
        }
        for (label, kind) in [
            ("Skills (", "skill"),
            ("Agents (", "agent"),
            ("MCP servers (", "mcp"),
        ] {
            if let Some(rest) = trimmed.strip_prefix(label) {
                let names = rest.split_once(')').map(|(_, n)| n).unwrap_or("");
                for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    kinds.insert(name.into(), kind.into());
                }
            }
        }
        if trimmed.starts_with("component") {
            table = true;
            continue;
        }
        if table {
            let cells: Vec<&str> = trimmed.split_whitespace().collect();
            if cells.len() != 3 || !cells[1].starts_with('~') {
                table = false;
                continue;
            }
            details.components.push(Component {
                kind: kinds
                    .get(cells[0])
                    .cloned()
                    .unwrap_or_else(|| "skill".into()),
                name: cells[0].into(),
                always: tokens(cells[1]),
                invoke: tokens(cells[2]),
            });
        }
    }
    details
}

fn enabled(file: &Path) -> Map<String, Value> {
    fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v["enabledPlugins"].as_object().cloned())
        .unwrap_or_default()
}

fn flags(map: Map<String, Value>) -> BTreeMap<String, bool> {
    map.into_iter()
        .filter_map(|(id, v)| v.as_bool().map(|on| (id, on)))
        .collect()
}

#[derive(Debug, Default, Serialize)]
pub struct Flags {
    pub user: BTreeMap<String, bool>,
    pub projects: BTreeMap<String, BTreeMap<String, bool>>,
}

pub fn read_flags(claude: Option<PathBuf>, projects: &[PathBuf]) -> Flags {
    Flags {
        user: claude
            .map(|dir| flags(enabled(&dir.join("settings.json"))))
            .unwrap_or_default(),
        projects: projects
            .iter()
            .map(|path| {
                let mut merged = flags(enabled(&path.join(".claude/settings.json")));
                merged.extend(flags(enabled(&path.join(".claude/settings.local.json"))));
                (path.to_string_lossy().into_owned(), merged)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DETAILS: &str = "impeccable 4.6.0
  Description: Design fluency.
  Source: impeccable@impeccable

Component inventory
  Skills (1)  impeccable
  Agents (2)  impeccable-documenter, impeccable-finish-reviewer
  Hooks (3)  SessionStart, PostToolUse, Stop  (harness-only — no model context cost)
  MCP servers (0)
  LSP servers (0)

Projected token cost
  Always-on:   ~439 tok   added to every session

Per-component (rounded)
  component                       always-on  on-invoke
  impeccable                           ~220      ~3.7k
  impeccable-documenter                 ~60       ~960
  impeccable-finish-reviewer            ~70      ~5.2k

  On-invoke cost is paid each time a skill or agent fires.
";

    #[test]
    fn parses_details() {
        let d = parse_details(DETAILS);
        assert_eq!(d.always, 439);
        assert_eq!(d.components.len(), 3);
        assert_eq!(
            d.components[0],
            Component {
                kind: "skill".into(),
                name: "impeccable".into(),
                always: 220,
                invoke: 3700
            }
        );
        assert_eq!(d.components[2].kind, "agent");
        assert_eq!(d.components[2].invoke, 5200);
    }

    #[test]
    fn local_flags_win_over_shared() {
        let dir = std::env::temp_dir().join(format!("squadron-plugins-{}", std::process::id()));
        let project = dir.join("p");
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::write(
            dir.join("settings.json"),
            r#"{"enabledPlugins":{"a@m":true,"b@m":false}}"#,
        )
        .unwrap();
        fs::write(
            project.join(".claude/settings.json"),
            r#"{"enabledPlugins":{"a@m":false,"b@m":true}}"#,
        )
        .unwrap();
        fs::write(
            project.join(".claude/settings.local.json"),
            r#"{"permissions":{},"enabledPlugins":{"a@m":true}}"#,
        )
        .unwrap();
        let f = read_flags(Some(dir.clone()), &[project.clone()]);
        assert_eq!(f.user.get("a@m"), Some(&true));
        let p = &f.projects[&project.to_string_lossy().into_owned()];
        assert_eq!(p.get("a@m"), Some(&true));
        assert_eq!(p.get("b@m"), Some(&true));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_other_commands() {
        assert!(run("claude", &["eval".into()], None).is_err());
        assert!(run("claude", &[], None).is_err());
    }
}
