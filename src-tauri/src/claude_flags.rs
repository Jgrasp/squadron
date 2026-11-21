use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FLAGS: [&str; 3] = [
    "inputNeededNotifEnabled",
    "agentPushNotifEnabled",
    "autoUploadSessions",
];

fn global_config() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(std::env::home_dir)
        .map(|dir| dir.join(".claude.json"))
}

fn read(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn flags_in(settings: Option<&Value>, global: Option<&Value>) -> BTreeMap<String, bool> {
    FLAGS
        .iter()
        .map(|key| {
            let value = settings
                .and_then(|s| s.get(key))
                .or_else(|| global.and_then(|g| g.get(key)))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            (key.to_string(), value)
        })
        .collect()
}

pub fn read_flags() -> BTreeMap<String, bool> {
    let settings = crate::sessions::claude_dir().and_then(|d| read(&d.join("settings.json")));
    let global = global_config().and_then(|p| read(&p));
    flags_in(settings.as_ref(), global.as_ref())
}

pub fn values_in(settings: Option<&Value>, global: Option<&Value>) -> BTreeMap<String, String> {
    let get = |key: &str| {
        settings
            .and_then(|s| s.get(key))
            .or_else(|| global.and_then(|g| g.get(key)))
    };
    let flag = |key: &str| get(key).and_then(Value::as_bool);
    let text = |key: &str| get(key).and_then(Value::as_str).map(str::to_string);
    let unless_off = |key: &str| (flag(key) != Some(false)).to_string();
    let mut values = BTreeMap::from([
        (
            "outputStyle",
            text("outputStyle").unwrap_or_else(|| "default".into()),
        ),
        ("thinking", unless_off("alwaysThinkingEnabled")),
        ("recap", unless_off("awaySummaryEnabled")),
        (
            "promptSuggestionEnabled",
            unless_off("promptSuggestionEnabled"),
        ),
        (
            "workflowKeywordTriggerEnabled",
            unless_off("workflowKeywordTriggerEnabled"),
        ),
        ("useAutoModeDuringPlan", unless_off("useAutoModeDuringPlan")),
        ("autoCompact", unless_off("autoCompactEnabled")),
        ("checkpoints", unless_off("fileCheckpointingEnabled")),
        (
            "worktreeBaseRef",
            get("worktree")
                .and_then(|w| w.get("baseRef"))
                .and_then(Value::as_str)
                .unwrap_or("fresh")
                .to_string(),
        ),
        (
            "remoteControl",
            flag("remoteControlAtStartup").map_or("default".into(), |b| b.to_string()),
        ),
    ]);
    if let Some(language) = text("language") {
        values.insert("language", language);
    }
    if let Some(switch) = flag("switchModelsOnFlag") {
        let label = if switch {
            "Switch automatically"
        } else {
            "Ask each time"
        };
        values.insert("switchModelsOnFlag", label.into());
    }
    let workflows = if flag("disableWorkflows") == Some(true) {
        Some(false)
    } else {
        flag("enableWorkflows")
    };
    if let Some(on) = workflows {
        values.insert("workflows", on.to_string());
    }
    if let Some(size) = text("workflowSizeGuideline") {
        values.insert("workflowSizeGuideline", size);
    }
    values
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

pub fn read_values() -> BTreeMap<String, String> {
    let settings = crate::sessions::claude_dir().and_then(|d| read(&d.join("settings.json")));
    let global = global_config().and_then(|p| read(&p));
    values_in(settings.as_ref(), global.as_ref())
}

pub fn with_value(text: &str, key: &str, value: &Value) -> Result<String, String> {
    let parsed: Value = if text.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        serde_json::from_str(text).map_err(|e| e.to_string())?
    };
    if !parsed.is_object() {
        return Err("Claude Code settings are not a JSON object.".into());
    }
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if parsed.get(key).is_some()
        && let Some((start, end)) = value_at(text, key)
    {
        return Ok(format!("{}{json}{}", &text[..start], &text[end..]));
    }
    if parsed.get(key).is_some() || text.trim().is_empty() {
        let mut map = parsed.as_object().cloned().unwrap_or_default();
        map.insert(key.into(), value.clone());
        return serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string());
    }
    let open = text
        .find('{')
        .ok_or("Claude Code settings are not a JSON object.")?;
    let empty = parsed.as_object().is_some_and(|m| m.is_empty());
    let entry = format!("\n  \"{key}\": {json}{}", if empty { "\n" } else { "," });
    Ok(format!("{}{entry}{}", &text[..=open], &text[open + 1..]))
}

fn value_at(text: &str, key: &str) -> Option<(usize, usize)> {
    let quoted = format!("\"{key}\"");
    let after = text.find(&quoted)? + quoted.len();
    let rest = text[after..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let start = text.len() - rest.len();
    let mut values = serde_json::Deserializer::from_str(rest).into_iter::<Value>();
    values.next()?.ok()?;
    Some((start, start + values.byte_offset()))
}

pub fn write_setting(key: &str, value: Value) -> Result<(), String> {
    if !["autoUploadSessions", "outputStyle"].contains(&key) {
        return Err(format!("Squadron does not write {key}."));
    }
    let dir = crate::sessions::claude_dir().ok_or("No Claude Code folder.")?;
    let path = dir.join("settings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let next = with_value(&text, key, &value)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, next).map_err(|e| e.to_string())
}

pub fn write_flag(key: &str, value: bool) -> Result<(), String> {
    write_setting(key, Value::Bool(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_win_over_the_global_config() {
        let settings = serde_json::json!({ "agentPushNotifEnabled": true });
        let global =
            serde_json::json!({ "agentPushNotifEnabled": false, "inputNeededNotifEnabled": true });
        let flags = flags_in(Some(&settings), Some(&global));
        assert_eq!(flags["agentPushNotifEnabled"], serde_json::json!(true));
        assert_eq!(flags["inputNeededNotifEnabled"], serde_json::json!(true));
        assert_eq!(flags["autoUploadSessions"], serde_json::json!(false));
    }

    #[test]
    fn replaces_a_flag_in_place() {
        let text = "{\n  \"model\": \"opus\",\n  \"autoUploadSessions\": false\n}\n";
        assert_eq!(
            with_value(text, "autoUploadSessions", &Value::Bool(true)).unwrap(),
            "{\n  \"model\": \"opus\",\n  \"autoUploadSessions\": true\n}\n"
        );
    }

    #[test]
    fn adds_a_missing_flag_first_and_keeps_the_rest() {
        let text = "{\n  \"zeta\": 1,\n  \"alpha\": [1, 2]\n}\n";
        let next = with_value(text, "autoUploadSessions", &Value::Bool(true)).unwrap();
        assert_eq!(
            next,
            "{\n  \"autoUploadSessions\": true,\n  \"zeta\": 1,\n  \"alpha\": [1, 2]\n}\n"
        );
        let parsed: Value = serde_json::from_str(&next).unwrap();
        assert_eq!(parsed["alpha"], serde_json::json!([1, 2]));
    }

    #[test]
    fn writes_an_empty_file() {
        let next = with_value("", "autoUploadSessions", &Value::Bool(true)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&next).unwrap()["autoUploadSessions"],
            true
        );
        let next = with_value("{}", "autoUploadSessions", &Value::Bool(false)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&next).unwrap()["autoUploadSessions"],
            false
        );
    }

    #[test]
    fn refuses_invalid_json() {
        assert!(with_value("{ nope", "autoUploadSessions", &Value::Bool(true)).is_err());
    }

    #[test]
    fn replaces_a_string_in_place() {
        let text = "{\n  \"outputStyle\": \"Learning \\\"x\\\"\",\n  \"model\": \"opus\"\n}\n";
        assert_eq!(
            with_value(text, "outputStyle", &Value::from("Concise")).unwrap(),
            "{\n  \"outputStyle\": \"Concise\",\n  \"model\": \"opus\"\n}\n"
        );
    }

    #[test]
    fn adds_a_missing_string() {
        let next = with_value(
            "{\n  \"model\": \"opus\"\n}\n",
            "outputStyle",
            &Value::from("Concise"),
        )
        .unwrap();
        assert_eq!(
            next,
            "{\n  \"outputStyle\": \"Concise\",\n  \"model\": \"opus\"\n}\n"
        );
    }

    #[test]
    fn reads_config_values_with_their_defaults() {
        let values = values_in(None, None);
        assert_eq!(values["outputStyle"], "default");
        assert_eq!(values["thinking"], "true");
        assert_eq!(values["autoCompact"], "true");
        assert_eq!(values["worktreeBaseRef"], "fresh");
        assert_eq!(values["remoteControl"], "default");
        for unknown in [
            "language",
            "switchModelsOnFlag",
            "workflows",
            "workflowSizeGuideline",
        ] {
            assert!(!values.contains_key(unknown), "{unknown}");
        }
    }

    #[test]
    fn reads_config_values_under_their_stored_names() {
        let settings = serde_json::json!({
            "alwaysThinkingEnabled": false,
            "awaySummaryEnabled": false,
            "autoCompactEnabled": false,
            "switchModelsOnFlag": true,
            "enableWorkflows": true,
            "disableWorkflows": true,
            "worktree": { "baseRef": "head" },
            "language": "French",
        });
        let global = serde_json::json!({
            "fileCheckpointingEnabled": false,
            "autoCompactEnabled": true,
            "workflowSizeGuideline": "small",
            "remoteControlAtStartup": true,
        });
        let values = values_in(Some(&settings), Some(&global));
        assert_eq!(values["thinking"], "false");
        assert_eq!(values["recap"], "false");
        assert_eq!(values["autoCompact"], "false");
        assert_eq!(values["checkpoints"], "false");
        assert_eq!(values["switchModelsOnFlag"], "Switch automatically");
        assert_eq!(values["workflows"], "false");
        assert_eq!(values["workflowSizeGuideline"], "small");
        assert_eq!(values["worktreeBaseRef"], "head");
        assert_eq!(values["remoteControl"], "true");
        assert_eq!(values["language"], "French");
    }
}
