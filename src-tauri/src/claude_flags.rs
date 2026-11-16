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

pub fn with_flag(text: &str, key: &str, value: bool) -> Result<String, String> {
    let parsed: Value = if text.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        serde_json::from_str(text).map_err(|e| e.to_string())?
    };
    if !parsed.is_object() {
        return Err("Claude Code settings are not a JSON object.".into());
    }
    if let Some((start, end)) = bool_at(text, key) {
        return Ok(format!("{}{value}{}", &text[..start], &text[end..]));
    }
    if parsed.get(key).is_some() || text.trim().is_empty() {
        let mut map = parsed.as_object().cloned().unwrap_or_default();
        map.insert(key.into(), Value::Bool(value));
        return serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string());
    }
    let open = text
        .find('{')
        .ok_or("Claude Code settings are not a JSON object.")?;
    let empty = parsed.as_object().is_some_and(|m| m.is_empty());
    let entry = format!("\n  \"{key}\": {value}{}", if empty { "\n" } else { "," });
    Ok(format!("{}{entry}{}", &text[..=open], &text[open + 1..]))
}

fn bool_at(text: &str, key: &str) -> Option<(usize, usize)> {
    let quoted = format!("\"{key}\"");
    let after = text.find(&quoted)? + quoted.len();
    let rest = text[after..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let start = text.len() - rest.len();
    let word = ["true", "false"]
        .into_iter()
        .find(|w| rest.starts_with(w))?;
    Some((start, start + word.len()))
}

pub fn write_flag(key: &str, value: bool) -> Result<(), String> {
    if key != "autoUploadSessions" {
        return Err(format!("Squadron does not write {key}."));
    }
    let dir = crate::sessions::claude_dir().ok_or("No Claude Code folder.")?;
    let path = dir.join("settings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let next = with_flag(&text, key, value)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, next).map_err(|e| e.to_string())
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
        assert_eq!(flags["agentPushNotifEnabled"], true);
        assert_eq!(flags["inputNeededNotifEnabled"], true);
        assert_eq!(flags["autoUploadSessions"], false);
    }

    #[test]
    fn replaces_a_flag_in_place() {
        let text = "{\n  \"model\": \"opus\",\n  \"autoUploadSessions\": false\n}\n";
        assert_eq!(
            with_flag(text, "autoUploadSessions", true).unwrap(),
            "{\n  \"model\": \"opus\",\n  \"autoUploadSessions\": true\n}\n"
        );
    }

    #[test]
    fn adds_a_missing_flag_first_and_keeps_the_rest() {
        let text = "{\n  \"zeta\": 1,\n  \"alpha\": [1, 2]\n}\n";
        let next = with_flag(text, "autoUploadSessions", true).unwrap();
        assert_eq!(
            next,
            "{\n  \"autoUploadSessions\": true,\n  \"zeta\": 1,\n  \"alpha\": [1, 2]\n}\n"
        );
        let parsed: Value = serde_json::from_str(&next).unwrap();
        assert_eq!(parsed["alpha"], serde_json::json!([1, 2]));
    }

    #[test]
    fn writes_an_empty_file() {
        let next = with_flag("", "autoUploadSessions", true).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&next).unwrap()["autoUploadSessions"],
            true
        );
        let next = with_flag("{}", "autoUploadSessions", false).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&next).unwrap()["autoUploadSessions"],
            false
        );
    }

    #[test]
    fn refuses_invalid_json() {
        assert!(with_flag("{ nope", "autoUploadSessions", true).is_err());
    }
}
