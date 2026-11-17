use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::OnceLock;

pub const ACTIVITY: &str = "agents:activity";
pub const SHOW_AGENTS: &str = "agents:show";
pub const READ_PROJECTS: &str = "projects:read";
pub const READ_USAGE: &str = "usage:read";

const STATUSES: [&str; 5] = ["working", "waiting", "done", "interrupted", "error"];
const WAITING: [&str; 2] = ["permission", "question"];
const FAILURES: [&str; 2] = ["crash", "api"];
const API: [&str; 5] = ["ok", "overloaded", "outage", "offline", "signed_out"];
const MAX_ID: usize = 128;
const MAX_NAME: usize = 300;
pub const MAX_FED: usize = 4096;

static SALT: OnceLock<Vec<u8>> = OnceLock::new();

fn fresh_salt() -> Vec<u8> {
    [uuid::Uuid::new_v4(), uuid::Uuid::new_v4()]
        .iter()
        .flat_map(|u| *u.as_bytes())
        .collect()
}

pub fn keep_salt(file: &Path) {
    let salt = std::fs::read(file)
        .ok()
        .filter(|s| s.len() == 32)
        .unwrap_or_else(|| {
            let salt = fresh_salt();
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(file, &salt);
            salt
        });
    let _ = SALT.set(salt);
}

pub fn opaque(path: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(SALT.get_or_init(fresh_salt));
    hash.update(path.as_bytes());
    hash.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn opaque_of(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(|v| opaque(&v))
}

pub fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_ID
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

fn name(s: String) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect()
}

fn one_of(value: Option<String>, set: &[&str]) -> Option<String> {
    value.filter(|v| set.contains(&v.as_str()))
}

fn color(value: Option<String>) -> Option<String> {
    value.filter(|c| {
        c.strip_prefix('#').is_some_and(|h| {
            matches!(h.len(), 3 | 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit())
        })
    })
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub group: Option<String>,
    pub archived: bool,
    pub active: u64,
    pub week_agents: u32,
    pub agents: u32,
}

impl Project {
    pub fn clean(self) -> Option<Self> {
        (!self.id.is_empty()).then(|| Self {
            id: opaque(&self.id),
            name: name(self.name),
            color: color(self.color),
            group: opaque_of(self.group),
            ..self
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
}

impl Group {
    pub fn clean(self) -> Option<Self> {
        (!self.id.is_empty()).then(|| Self {
            id: opaque(&self.id),
            name: name(self.name),
            parent: opaque_of(self.parent),
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Agent {
    pub id: String,
    pub project: String,
    pub name: String,
    pub status: String,
    pub waiting: Option<String>,
    pub tool: Option<String>,
    pub error: Option<String>,
    pub by: Option<String>,
    pub archived: bool,
    pub since: u64,
}

impl Agent {
    pub fn clean(self) -> Option<Self> {
        if !id(&self.id) || self.project.is_empty() || !STATUSES.contains(&self.status.as_str()) {
            return None;
        }
        let at = |status: &str| self.status == status;
        let (waiting, tool, failed) = (at("waiting"), at("working"), at("error"));
        Some(Self {
            project: opaque(&self.project),
            name: name(self.name),
            waiting: one_of(self.waiting, &WAITING).filter(|_| waiting),
            tool: self.tool.filter(|t| tool && id(t)),
            error: self.error.filter(|e| {
                if failed {
                    FAILURES.contains(&e.as_str())
                } else {
                    e == "mcp"
                }
            }),
            by: self.by.filter(|b| id(b) && *b != self.id),
            ..self
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub utilization: f64,
    pub resets_at: Option<u64>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Usage {
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub api: String,
    pub retries: u32,
    pub since: u64,
}

impl Usage {
    pub fn clean(self) -> Option<Self> {
        let window = |w: Option<Window>| {
            w.filter(|w| w.utilization.is_finite()).map(|w| Window {
                utilization: w.utilization.clamp(0.0, 1.0),
                ..w
            })
        };
        let api = one_of(Some(self.api), &API)?;
        Some(Self {
            five_hour: window(self.five_hour),
            seven_day: window(self.seven_day),
            retries: if api == "overloaded" { self.retries } else { 0 },
            api,
            ..self
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct Fed {
    pub name: String,
    pub data: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Admitted {
    pub key: String,
    pub access: &'static str,
    pub name: String,
    pub data: Value,
}

fn parsed<T: for<'de> Deserialize<'de>>(data: Value) -> Option<T> {
    serde_json::from_value(data).ok()
}

pub fn admit(fed: Fed) -> Option<Admitted> {
    let name = fed.name;
    let (key, access, data) = match name.as_str() {
        "agent.status" | "agent.started" | "agent.archived" => {
            let agent = parsed::<Agent>(fed.data)?.clean()?;
            (agent.id.clone(), ACTIVITY, json!(agent))
        }
        "agent.message" => {
            let from = fed.data.get("from")?.as_str()?.to_string();
            let to = fed.data.get("to")?.as_str()?.to_string();
            if !id(&from) || !id(&to) || from == to {
                return None;
            }
            (
                format!("{from}>{to}"),
                ACTIVITY,
                json!({"from": from, "to": to, "count": 1}),
            )
        }
        "project.added" | "project.changed" | "project.archived" | "project.removed" => {
            let project = parsed::<Project>(fed.data)?.clean()?;
            (project.id.clone(), READ_PROJECTS, json!(project))
        }
        "groups.changed" => {
            let groups: Vec<Group> = parsed(fed.data.get("groups")?.clone())?;
            let groups: Vec<Group> = groups.into_iter().filter_map(Group::clean).collect();
            (String::new(), READ_PROJECTS, json!({"groups": groups}))
        }
        "usage" => (
            String::new(),
            READ_USAGE,
            json!(parsed::<Usage>(fed.data)?.clean()?),
        ),
        _ => return None,
    };
    Some(Admitted {
        key: format!("{name}\n{key}"),
        access,
        name,
        data,
    })
}

pub fn merge(pending: &mut Vec<Admitted>, next: Admitted) {
    if let Some(known) = pending.iter_mut().find(|p| p.key == next.key) {
        if next.name == "agent.message" {
            let count = known.data["count"].as_u64().unwrap_or(1);
            known.data["count"] = json!(count.saturating_add(1));
        } else {
            known.data = next.data;
        }
    } else if pending.len() < MAX_FED {
        pending.push(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(data: Value) -> Option<Admitted> {
        admit(Fed {
            name: "agent.status".into(),
            data,
        })
    }

    #[test]
    fn keeps_only_what_an_agent_does() {
        let full = json!({
            "id": "a1", "project": "/home/me/ring", "name": "RING-1 Card\u{7}payment", "status": "working",
            "waiting": "permission", "tool": "mcp__jira__search", "error": "crash", "by": "a0",
            "archived": false, "since": 5, "dir": "/home/me/secret", "prompt": "the user's prompt", "cost": 3.2
        });
        let admitted = agent(full).unwrap();
        assert_eq!(admitted.access, ACTIVITY);
        assert_eq!(
            admitted.data,
            json!({"id": "a1", "project": opaque("/home/me/ring"), "name": "RING-1 Cardpayment", "status": "working", "waiting": null, "tool": "mcp__jira__search", "error": null, "by": "a0", "archived": false, "since": 5})
        );
        assert!(!admitted.data.to_string().contains("/home"));
        let waiting = json!({"id": "a1", "project": "/p", "name": "", "status": "waiting", "waiting": "Allow rm -rf /?", "tool": "Bash: rm -rf /", "error": "mcp", "by": "a1", "archived": false, "since": 5});
        let data = agent(waiting).unwrap().data;
        assert_eq!(
            [&data["waiting"], &data["tool"], &data["by"], &data["error"]],
            [&Value::Null, &Value::Null, &Value::Null, &json!("mcp")]
        );
        for bad in [
            json!({"id": "../x", "project": "/p", "name": "", "status": "done", "archived": false, "since": 0}),
            json!({"id": "a", "project": "/p", "name": "", "status": "sleeping", "archived": false, "since": 0}),
            json!({"id": "a", "project": "", "name": "", "status": "done", "archived": false, "since": 0}),
        ] {
            assert!(agent(bad).is_none());
        }
        let error = |kind: &str| {
            agent(json!({"id": "a1", "project": "/p", "name": "", "status": "error", "error": kind, "archived": true, "since": 1})).unwrap().data["error"].clone()
        };
        assert_eq!(error("api"), "api");
        assert_eq!(error("hook"), Value::Null);
    }

    #[test]
    fn hides_project_folders_and_bad_colors() {
        let project = |color: &str| {
            admit(Fed {
                name: "project.changed".into(),
                data: json!({"id": "/home/me/squadron", "name": "Squadron", "color": color, "group": "g1", "archived": false, "active": 1, "week_agents": 2, "agents": 9, "path": "/home/me/squadron"}),
            })
            .unwrap()
        };
        let p = project("#d08a4f");
        assert_eq!(p.access, READ_PROJECTS);
        assert_eq!(p.data["id"], opaque("/home/me/squadron"));
        assert_eq!(p.data["id"].as_str().unwrap().len(), 16);
        assert_eq!(p.data["color"], "#d08a4f");
        assert_eq!(p.data["group"], opaque("g1"));
        assert!(!p.data.to_string().contains("/home"));
        assert_eq!(project("red;background:url(x)").data["color"], Value::Null);
        assert!(admit(Fed { name: "project.deleted".into(), data: json!({"id": "/p", "name": "", "archived": false, "active": 0, "week_agents": 0, "agents": 0}) }).is_none());
        let groups = admit(Fed {
            name: "groups.changed".into(),
            data: json!({"groups": [{"id": "g1", "name": "Work", "parent": ""}, {"id": "g2", "name": "Client", "parent": "g1"}]}),
        })
        .unwrap();
        assert_eq!(groups.data["groups"][0]["parent"], Value::Null);
        assert_eq!(groups.data["groups"][1]["parent"], opaque("g1"));
    }

    #[test]
    fn tells_the_usage_with_its_known_states_only() {
        let usage = |api: &str| {
            admit(Fed {
                name: "usage".into(),
                data: json!({"five_hour": {"utilization": 1.4, "resets_at": 9}, "seven_day": null, "api": api, "retries": 3, "since": 1}),
            })
        };
        let ok = usage("ok").unwrap();
        assert_eq!(ok.access, READ_USAGE);
        assert_eq!(ok.data["five_hour"]["utilization"], 1.0);
        assert_eq!(ok.data["retries"], 0);
        assert_eq!(usage("overloaded").unwrap().data["retries"], 3);
        assert!(usage("melted").is_none());
    }

    #[test]
    fn keeps_the_salt_across_restarts() {
        let file = std::env::temp_dir().join(format!("squadron-salt-{}", uuid::Uuid::new_v4()));
        keep_salt(&file);
        let salt = std::fs::read(&file).unwrap();
        assert_eq!(salt.len(), 32);
        keep_salt(&file);
        assert_eq!(std::fs::read(&file).unwrap(), salt);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn keeps_the_latest_state_and_counts_messages() {
        let mut pending = Vec::new();
        let status = |s: &str| {
            agent(json!({"id": "a1", "project": "/p", "name": "", "status": s, "archived": false, "since": 0})).unwrap()
        };
        let message = || {
            admit(Fed {
                name: "agent.message".into(),
                data: json!({"from": "a1", "to": "a2", "text": "secret"}),
            })
            .unwrap()
        };
        for next in [
            status("working"),
            message(),
            status("waiting"),
            message(),
            status("done"),
        ] {
            merge(&mut pending, next);
        }
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].data["status"], "done");
        assert_eq!(
            pending[1].data,
            json!({"from": "a1", "to": "a2", "count": 2})
        );
        assert!(
            admit(Fed {
                name: "agent.message".into(),
                data: json!({"from": "a1", "to": "a1"})
            })
            .is_none()
        );
        for i in 0..MAX_FED + 10 {
            merge(
                &mut pending,
                agent(json!({"id": format!("x{i}"), "project": "/p", "name": "", "status": "done", "archived": false, "since": 0})).unwrap(),
            );
        }
        assert_eq!(pending.len(), MAX_FED);
    }
}
