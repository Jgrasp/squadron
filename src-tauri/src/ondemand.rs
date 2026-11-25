use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
struct Server {
    config: Value,
    #[serde(default)]
    description: String,
}

pub struct OnDemand {
    file: PathBuf,
    servers: Mutex<BTreeMap<String, Server>>,
    connectors: Mutex<Connectors>,
}

#[derive(Default, Serialize, Deserialize)]
struct Connectors {
    #[serde(default)]
    on_demand: BTreeMap<String, String>,
    #[serde(default)]
    off: BTreeMap<String, BTreeSet<String>>,
}

pub fn is_connector(name: &str) -> bool {
    name.starts_with("claude.ai ")
}

impl Connectors {
    fn plan(
        &mut self,
        key: &str,
        wanted: &[String],
        added: &[String],
        disabled: &BTreeSet<String>,
    ) -> Result<Vec<(String, bool)>, String> {
        if let Some(theirs) = added.iter().find(|c| {
            self.on_demand.contains_key(*c)
                && disabled.contains(*c)
                && !self.off.get(key).is_some_and(|ours| ours.contains(*c))
        }) {
            return Err(format!(
                "{theirs} is turned off in this folder by the user, outside Squadron: they can turn it back on with /mcp."
            ));
        }
        let ours = self.off.entry(key.to_string()).or_default();
        ours.retain(|c| disabled.contains(c));
        let mut toggles = Vec::new();
        ours.retain(|c| {
            let keep = self.on_demand.contains_key(c);
            if !keep {
                toggles.push((c.clone(), true));
            }
            keep
        });
        for c in self.on_demand.keys() {
            match (wanted.contains(c), disabled.contains(c), ours.contains(c)) {
                (false, false, _) => {
                    toggles.push((c.clone(), false));
                    ours.insert(c.clone());
                }
                (true, true, true) => {
                    toggles.push((c.clone(), true));
                    ours.remove(c);
                }
                _ => {}
            }
        }
        if ours.is_empty() {
            self.off.remove(key);
        }
        Ok(toggles)
    }
}

#[derive(Serialize)]
pub struct Listed {
    name: String,
    description: String,
    command: String,
}

#[derive(Serialize)]
pub struct Servers {
    on_demand: Vec<Listed>,
    everywhere: Vec<Listed>,
}

impl OnDemand {
    pub fn load(file: PathBuf) -> Self {
        let servers = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let connectors = std::fs::read_to_string(connectors_file(&file))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let found = Self {
            file,
            servers: Mutex::new(servers),
            connectors: Mutex::new(connectors),
        };
        found.announce();
        found
    }

    fn save(&self, servers: &BTreeMap<String, Server>) -> Result<(), String> {
        let text = serde_json::to_string_pretty(servers).map_err(|e| e.to_string())?;
        let tmp = self.file.with_extension("json.tmp");
        write_private(&tmp, &text)?;
        std::fs::rename(&tmp, &self.file).map_err(|e| e.to_string())
    }

    fn save_connectors(&self, connectors: &Connectors) -> Result<(), String> {
        let text = serde_json::to_string_pretty(connectors).map_err(|e| e.to_string())?;
        let file = connectors_file(&self.file);
        let tmp = file.with_extension("json.tmp");
        write_private(&tmp, &text)?;
        std::fs::rename(&tmp, &file).map_err(|e| e.to_string())
    }

    fn announce(&self) {
        let servers = self.servers.lock().unwrap();
        let connectors = self.connectors.lock().unwrap();
        *crate::claude::ON_DEMAND.lock().unwrap() = prompt(&servers, &connectors.on_demand);
    }

    /// Which claude.ai connectors to switch off or back on with Claude Code's own `mcp_toggle` in `dir`, leaving alone the ones the user turned off there.
    pub fn connector_toggles(
        &self,
        dir: &Path,
        wanted: &[String],
        added: &[String],
    ) -> Result<Vec<(String, bool)>, String> {
        let mut connectors = self.connectors.lock().unwrap();
        if connectors.on_demand.is_empty() && connectors.off.is_empty() {
            return Ok(Vec::new());
        }
        let key = project_key(dir);
        let toggles = connectors.plan(&key, wanted, added, &disabled_in(&key))?;
        self.save_connectors(&connectors)?;
        Ok(toggles)
    }

    pub fn configs(&self, names: &[String]) -> Map<String, Value> {
        let servers = self.servers.lock().unwrap();
        names
            .iter()
            .filter_map(|n| Some((n.clone(), servers.get(n)?.config.clone())))
            .collect()
    }
}

fn connectors_file(file: &Path) -> PathBuf {
    file.with_file_name("mcp-connectors.json")
}

fn project_key(dir: &Path) -> String {
    let root = crate::process::command("git")
        .current_dir(dir)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| {
            Path::new(String::from_utf8_lossy(&out.stdout).trim())
                .parent()
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| dir.to_path_buf());
    root.canonicalize()
        .unwrap_or(root)
        .to_string_lossy()
        .into_owned()
}

fn claude_json() -> Value {
    std::env::home_dir()
        .and_then(|home| std::fs::read_to_string(home.join(".claude.json")).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn names_in(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect()
}

fn disabled_in(key: &str) -> BTreeSet<String> {
    names_in(&claude_json()["projects"][key]["disabledMcpServers"])
}

fn prompt(servers: &BTreeMap<String, Server>, connectors: &BTreeMap<String, String>) -> String {
    if servers.is_empty() && connectors.is_empty() {
        return String::new();
    }
    let list = servers
        .iter()
        .map(|(name, s)| (name, s.description.as_str()))
        .chain(connectors.iter().map(|(name, d)| (name, d.as_str())))
        .map(|(name, d)| match d.trim() {
            "" => name.clone(),
            d => format!("{name} ({d})"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "These MCP servers stay off until an agent needs them: {list}. When your task needs one, start it for yourself with use_mcp from squadron-pilot, then load its tools with ToolSearch; stop it with use_mcp once you no longer need it."
    )
}

#[cfg(unix)]
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes()).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| e.to_string())
}

fn command_of(config: &Value) -> String {
    if let Some(url) = config["url"].as_str() {
        return url
            .split("://")
            .nth(1)
            .unwrap_or(url)
            .split(['/', '?'])
            .next()
            .and_then(|host| host.rsplit('@').next())
            .unwrap_or_default()
            .to_string();
    }
    let program = config["command"].as_str().unwrap_or_default();
    let program = Path::new(program)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let package = config["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|a| !a.starts_with('-') && !a.contains('=') && !a.contains("://"));
    match (program.as_str(), package) {
        ("npx" | "uvx" | "bunx" | "pnpx", Some(package)) => format!("{program} {package}"),
        _ => program,
    }
}

fn user_servers() -> Map<String, Value> {
    claude_json()["mcpServers"]
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn valid(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('-')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("Invalid server name.".into());
    }
    Ok(())
}

async fn claude_mcp(program: String, args: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let out = crate::process::command(program)
            .current_dir(std::env::temp_dir())
            .arg("mcp")
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn mcp_servers(state: State<OnDemand>) -> Servers {
    let servers = state.servers.lock().unwrap();
    let on_demand: Vec<Listed> = servers
        .iter()
        .map(|(name, s)| Listed {
            name: name.clone(),
            description: s.description.clone(),
            command: command_of(&s.config),
        })
        .collect();
    let connectors = state.connectors.lock().unwrap();
    let on_demand = on_demand
        .into_iter()
        .chain(connectors.on_demand.iter().map(|(name, d)| Listed {
            name: name.clone(),
            description: d.clone(),
            command: "claude.ai".into(),
        }))
        .collect();
    let everywhere = user_servers()
        .iter()
        .filter(|(name, _)| !servers.contains_key(*name))
        .map(|(name, config)| Listed {
            name: name.clone(),
            description: String::new(),
            command: command_of(config),
        })
        .chain(
            names_in(&claude_json()["claudeAiMcpEverConnected"])
                .into_iter()
                .filter(|name| is_connector(name) && !connectors.on_demand.contains_key(name))
                .map(|name| Listed {
                    name,
                    description: String::new(),
                    command: "claude.ai".into(),
                }),
        )
        .collect();
    Servers {
        on_demand,
        everywhere,
    }
}

#[tauri::command]
pub(crate) async fn mcp_on_demand(
    state: State<'_, OnDemand>,
    settings: State<'_, crate::Settings>,
    name: String,
) -> Result<(), String> {
    if is_connector(&name) {
        let mut connectors = state.connectors.lock().unwrap();
        connectors.on_demand.entry(name).or_default();
        state.save_connectors(&connectors)?;
        drop(connectors);
        state.announce();
        return Ok(());
    }
    valid(&name)?;
    let config = user_servers()
        .remove(&name)
        .ok_or("This server is not in Claude Code's user configuration.")?;
    {
        let mut servers = state.servers.lock().unwrap();
        servers.insert(
            name.clone(),
            Server {
                config,
                description: String::new(),
            },
        );
        state.save(&servers)?;
    }
    let removed = claude_mcp(
        settings.claude(),
        vec!["remove".into(), name.clone(), "-s".into(), "user".into()],
    )
    .await;
    if removed.is_err() {
        let mut servers = state.servers.lock().unwrap();
        servers.remove(&name);
        state.save(&servers)?;
    }
    state.announce();
    removed
}

#[tauri::command]
pub(crate) async fn mcp_everywhere(
    state: State<'_, OnDemand>,
    settings: State<'_, crate::Settings>,
    name: String,
) -> Result<(), String> {
    if is_connector(&name) {
        let mut connectors = state.connectors.lock().unwrap();
        connectors.on_demand.remove(&name);
        state.save_connectors(&connectors)?;
        drop(connectors);
        state.announce();
        return Ok(());
    }
    valid(&name)?;
    let config = state
        .servers
        .lock()
        .unwrap()
        .get(&name)
        .map(|s| s.config.to_string())
        .ok_or("Unknown server.")?;
    claude_mcp(
        settings.claude(),
        vec![
            "add-json".into(),
            name.clone(),
            config,
            "-s".into(),
            "user".into(),
        ],
    )
    .await?;
    let mut servers = state.servers.lock().unwrap();
    servers.remove(&name);
    state.save(&servers)?;
    drop(servers);
    state.announce();
    Ok(())
}

#[tauri::command]
pub fn mcp_describe(
    state: State<OnDemand>,
    name: String,
    description: String,
) -> Result<(), String> {
    let description = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if is_connector(&name) {
        let mut connectors = state.connectors.lock().unwrap();
        *connectors
            .on_demand
            .get_mut(&name)
            .ok_or("Unknown server.")? = description;
        state.save_connectors(&connectors)?;
    } else {
        let mut servers = state.servers.lock().unwrap();
        servers.get_mut(&name).ok_or("Unknown server.")?.description = description;
        state.save(&servers)?;
    }
    state.announce();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shows_the_program_or_the_host_and_never_the_secrets() {
        assert_eq!(
            command_of(
                &json!({"command": "npx", "args": ["-y", "@sentry/mcp-server@latest", "--access-token=abc"]})
            ),
            "npx @sentry/mcp-server@latest"
        );
        assert_eq!(
            command_of(&json!({"command": "/usr/bin/bitbucket-mcp", "env": {"TOKEN": "abc"}})),
            "bitbucket-mcp"
        );
        assert_eq!(
            command_of(
                &json!({"type": "http", "url": "https://user:pw@mcp.example.com/sse?key=abc"})
            ),
            "mcp.example.com"
        );
    }

    #[test]
    fn tells_claude_which_servers_it_can_start() {
        let mut servers = BTreeMap::new();
        let mut connectors = BTreeMap::new();
        assert_eq!(prompt(&servers, &connectors), "");
        servers.insert(
            "bitbucket".into(),
            Server {
                config: json!({}),
                description: String::new(),
            },
        );
        servers.insert(
            "sentry".into(),
            Server {
                config: json!({}),
                description: "production errors".into(),
            },
        );
        connectors.insert("claude.ai Gmail".to_string(), "mail".to_string());
        assert!(
            prompt(&servers, &connectors)
                .contains("bitbucket, sentry (production errors), claude.ai Gmail (mail).")
        );
    }

    #[test]
    fn keeps_the_servers_in_a_private_file() {
        let dir = std::env::temp_dir().join(format!("squadron-ondemand-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mcp-on-demand.json");
        let state = OnDemand::load(file.clone());
        let mut servers = BTreeMap::new();
        servers.insert(
            "a".into(),
            Server {
                config: json!({"command": "x"}),
                description: String::new(),
            },
        );
        state.save(&servers).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            OnDemand::load(file)
                .configs(&["a".into(), "b".into()])
                .len(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn never_touches_the_connectors_the_user_turned_off() {
        let mut c = Connectors::default();
        for name in ["claude.ai Gmail", "claude.ai Notion", "claude.ai Meetings"] {
            c.on_demand.insert(name.into(), String::new());
        }
        let key = "/home/q/shop";
        let theirs = set(&["claude.ai Gmail", "claude.ai Asana"]);
        let toggles = c.plan(key, &[], &[], &theirs).unwrap();
        assert_eq!(
            toggles,
            vec![
                ("claude.ai Meetings".to_string(), false),
                ("claude.ai Notion".to_string(), false)
            ]
        );
        assert_eq!(c.off[key], set(&["claude.ai Meetings", "claude.ai Notion"]));

        let file = set(&[
            "claude.ai Gmail",
            "claude.ai Asana",
            "claude.ai Meetings",
            "claude.ai Notion",
        ]);
        assert!(c.plan(key, &[], &[], &file).unwrap().is_empty());

        let gmail = vec!["claude.ai Gmail".to_string()];
        assert!(
            c.plan(key, &gmail, &gmail, &file)
                .unwrap_err()
                .contains("claude.ai Gmail")
        );
        assert_eq!(c.off[key], set(&["claude.ai Meetings", "claude.ai Notion"]));

        let notion = vec!["claude.ai Notion".to_string()];
        assert_eq!(
            c.plan(key, &notion, &notion, &file).unwrap(),
            vec![("claude.ai Notion".to_string(), true)]
        );
        assert_eq!(c.off[key], set(&["claude.ai Meetings"]));
    }

    #[test]
    fn gives_back_a_connector_set_to_everywhere_and_forgets_one_the_user_turned_on() {
        let mut c = Connectors::default();
        c.on_demand.insert("claude.ai Notion".into(), String::new());
        c.off.insert(
            "/k".into(),
            set(&["claude.ai Meetings", "claude.ai Notion"]),
        );
        let toggles = c
            .plan("/k", &[], &[], &set(&["claude.ai Meetings"]))
            .unwrap();
        assert_eq!(
            toggles,
            vec![
                ("claude.ai Meetings".to_string(), true),
                ("claude.ai Notion".to_string(), false)
            ]
        );
        assert_eq!(c.off["/k"], set(&["claude.ai Notion"]));
        c.on_demand.clear();
        c.plan("/k", &[], &[], &set(&["claude.ai Notion"])).unwrap();
        assert!(c.off.is_empty());
    }
}
