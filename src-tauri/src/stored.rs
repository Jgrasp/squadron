use serde::Deserialize;
use serde::de::IgnoredAny;
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
struct Said {
    #[serde(default)]
    entries: Vec<Line>,
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

pub fn texts(text: &str) -> String {
    let Ok(said) = serde_json::from_str::<Said>(text) else {
        return String::new();
    };
    said.entries
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
    fn entries_and_texts_read_the_history() {
        assert_eq!(
            serde_json::from_str::<Vec<serde_json::Value>>(&entries(AGENT).unwrap())
                .unwrap()
                .len(),
            3
        );
        assert_eq!(entries(r#"{"id":"a"}"#).unwrap(), "[]");
        assert_eq!(entries(r#"{"id":"a","entries":[{"kind""#), None);
        assert_eq!(texts(AGENT), "Salut\n\nGo\n");
    }
}
