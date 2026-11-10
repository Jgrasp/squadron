use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
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
        let found = Self {
            file,
            servers: Mutex::new(servers),
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

    fn announce(&self) {
        let servers = self.servers.lock().unwrap();
        *crate::claude::ON_DEMAND.lock().unwrap() = prompt(&servers);
    }

    pub fn configs(&self, names: &[String]) -> Map<String, Value> {
        let servers = self.servers.lock().unwrap();
        names
            .iter()
            .filter_map(|n| Some((n.clone(), servers.get(n)?.config.clone())))
            .collect()
    }
}

fn prompt(servers: &BTreeMap<String, Server>) -> String {
    if servers.is_empty() {
        return String::new();
    }
    let list = servers
        .iter()
        .map(|(name, s)| match s.description.trim() {
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
    std::env::home_dir()
        .and_then(|home| std::fs::read_to_string(home.join(".claude.json")).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|config| config["mcpServers"].as_object().cloned())
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
    let on_demand = servers
        .iter()
        .map(|(name, s)| Listed {
            name: name.clone(),
            description: s.description.clone(),
            command: command_of(&s.config),
        })
        .collect();
    let everywhere = user_servers()
        .iter()
        .filter(|(name, _)| !servers.contains_key(*name))
        .map(|(name, config)| Listed {
            name: name.clone(),
            description: String::new(),
            command: command_of(config),
        })
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
    let mut servers = state.servers.lock().unwrap();
    servers.get_mut(&name).ok_or("Unknown server.")?.description =
        description.split_whitespace().collect::<Vec<_>>().join(" ");
    state.save(&servers)?;
    drop(servers);
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
        assert_eq!(prompt(&servers), "");
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
        assert!(prompt(&servers).contains("bitbucket, sentry (production errors)."));
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
}
