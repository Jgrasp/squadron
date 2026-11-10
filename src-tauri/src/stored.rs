use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use serde_json::value::{RawValue, to_raw_value};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct History<'a> {
    #[serde(borrow)]
    entries: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Dated {
    at: Option<f64>,
    kind: Option<String>,
}

pub fn header(text: &str) -> Option<String> {
    let mut fields: BTreeMap<String, &RawValue> = serde_json::from_str(text).ok()?;
    let entries: Vec<Dated> = match fields.remove("entries") {
        Some(raw) => serde_json::from_str(raw.get()).ok()?,
        None => Vec::new(),
    };
    let stored = to_raw_value(&serde_json::json!({
        "count": entries.len(),
        "created": entries.iter().find_map(|e| e.at),
        "users": entries.iter().filter(|e| e.kind.as_deref() == Some("user")).count(),
    }))
    .ok()?;
    fields.insert("stored".into(), &stored);
    serde_json::to_string(&fields).ok()
}

pub fn entries(text: &str) -> Option<String> {
    let history = serde_json::from_str::<History>(text).ok()?;
    Some(
        history
            .entries
            .map_or_else(|| "[]".into(), |raw| raw.get().to_string()),
    )
}

pub fn keep_entries(data: String, old: impl FnOnce() -> Option<String>) -> Result<String, String> {
    let mut fields: BTreeMap<String, &RawValue> =
        serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if fields.contains_key("entries") {
        return Ok(data);
    }
    fields.remove("stored");
    let old = old();
    if let Some(raw) = old
        .as_deref()
        .and_then(|o| serde_json::from_str::<History>(o).ok())
        .and_then(|h| h.entries)
    {
        fields.insert("entries".into(), raw);
    }
    serde_json::to_string(&fields).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct Line {
    kind: String,
    text: Option<Text>,
    #[serde(default)]
    blocks: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    text: Option<Text>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Text {
    Plain(String),
    Other(IgnoredAny),
}

impl Text {
    fn plain(&self) -> &str {
        match self {
            Text::Plain(s) => s,
            Text::Other(_) => "",
        }
    }
}

#[derive(Deserialize)]
struct Shelved {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    project: String,
    #[serde(default)]
    since: f64,
    #[serde(default)]
    entries: Vec<Line>,
}

#[derive(Serialize, Clone)]
pub struct Found {
    pub id: String,
    name: String,
    project: String,
    since: f64,
    pub text: String,
}

pub fn found(text: &str) -> Option<Found> {
    let shelved = serde_json::from_str::<Shelved>(text).ok()?;
    Some(Found {
        text: said_text(&shelved.entries),
        id: shelved.id,
        name: shelved.name,
        project: shelved.project,
        since: shelved.since,
    })
}

fn said_text(entries: &[Line]) -> String {
    entries
        .iter()
        .map(|line| match line.kind.as_str() {
            "user" => line.text.as_ref().map_or("", Text::plain).to_string(),
            "agent" => line
                .blocks
                .iter()
                .map(|b| {
                    if b.kind == "text" {
                        b.text.as_ref().map_or("", Text::plain)
                    } else {
                        ""
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENT: &str = r#"{"id":"a","name":"Panier","entries":[{"kind":"agent","blocks":[{"type":"text","text":"Salut"},{"type":"tool_use","input":{"text":3}}]},{"kind":"user","text":"Go","at":12},{"kind":"note","at":15,"text":"x"}]}"#;

    #[test]
    fn header_drops_entries_and_counts_them() {
        let head: serde_json::Value = serde_json::from_str(&header(AGENT).unwrap()).unwrap();
        assert!(head.get("entries").is_none());
        assert_eq!(head["name"], "Panier");
        assert_eq!(head["stored"]["count"], 3);
        assert_eq!(head["stored"]["created"], 12.0);
        assert_eq!(head["stored"]["users"], 1);
    }

    #[test]
    fn saving_a_header_keeps_the_history_on_disk() {
        let saved = keep_entries(
            r#"{"id":"a","name":"Renommé","stored":{"count":3}}"#.into(),
            || Some(AGENT.into()),
        )
        .unwrap();
        let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(saved["name"], "Renommé");
        assert_eq!(saved["entries"].as_array().unwrap().len(), 3);
        assert!(saved.get("stored").is_none());
    }

    #[test]
    fn saving_a_full_agent_replaces_the_history() {
        let data = r#"{"id":"a","entries":[]}"#.to_string();
        let unread = || -> Option<String> { panic!("the old file is not needed") };
        assert_eq!(keep_entries(data.clone(), unread).unwrap(), data);
    }

    #[test]
    fn entries_read_the_history() {
        assert_eq!(
            serde_json::from_str::<Vec<serde_json::Value>>(&entries(AGENT).unwrap())
                .unwrap()
                .len(),
            3
        );
        assert_eq!(entries(r#"{"id":"a"}"#).unwrap(), "[]");
        assert_eq!(entries(r#"{"id":"a","entries":[{"kind""#), None);
    }

    #[test]
    fn found_keeps_the_header_and_the_text() {
        let found = serde_json::to_value(found(AGENT).unwrap()).unwrap();
        assert_eq!(found["id"], "a");
        assert_eq!(found["name"], "Panier");
        assert_eq!(found["text"], "Salut\n\nGo\n");
        assert!(found.get("entries").is_none());
    }
}
