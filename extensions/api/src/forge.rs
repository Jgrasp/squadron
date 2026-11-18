use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Input<A> {
    pub host: String,
    pub repo: String,
    pub groups: BTreeMap<String, String>,
    pub args: A,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NoArgs {}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BranchArgs {
    pub branch: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NumberArgs {
    pub number: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplyArgs {
    pub number: u64,
    pub to: String,
    pub kind: String,
    pub body: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PostArgs {
    pub number: u64,
    pub head: String,
    pub drafts: Vec<Draft>,
    pub summary: String,
    pub verdict: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateArgs {
    pub number: u64,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pull {
    pub number: u64,
    pub title: String,
    pub description: String,
    pub url: String,
    pub state: String,
    pub draft: bool,
    pub approvals: u32,
    pub changes_requested: bool,
    pub comments: u32,
    pub checks: Option<String>,
    pub branch: String,
    pub base: String,
    pub head: String,
    pub author: String,
    pub author_avatar: String,
    pub updated: String,
    pub reviewed: Option<String>,
    pub reviewed_head: Option<String>,
    pub reviewed_at: Option<String>,
    pub head_at: Option<String>,
    pub reviewers: Vec<Reviewer>,
    pub conflicts: bool,
    pub mergeable: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reviewer {
    pub name: String,
    pub avatar: String,
    pub verdict: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Numbered {
    pub pull: Pull,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub files: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lists {
    pub review: Vec<Pull>,
    pub mine: Vec<Pull>,
    pub others: Vec<Pull>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Remark {
    pub id: String,
    pub author: String,
    pub avatar: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub removed: bool,
    pub reply_to: Option<String>,
    pub at: String,
    pub kind: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Draft {
    pub path: String,
    pub line: u32,
    pub removed: bool,
    pub body: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Posted {
    pub done: usize,
    pub said: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Person {
    pub name: String,
    pub avatar: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    pub id: String,
    pub number: u64,
    pub name: String,
    pub title: String,
    pub event: String,
    pub branch: String,
    pub status: String,
    pub url: String,
    pub started: String,
    pub finished: Option<String>,
    pub author: String,
    pub author_avatar: String,
}

pub fn input<A: DeserializeOwned + Default>(input: &str) -> Result<Input<A>, String> {
    serde_json::from_str(input).map_err(|e| format!("The forge input is not valid: {e}"))
}

pub fn answer<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

pub fn failure(code: Option<&str>, title: Value, detail: Option<Value>) -> String {
    let mut error = json!({ "title": title });
    if let Some(code) = code {
        error["code"] = json!(code);
    }
    if let Some(detail) = detail {
        error["detail"] = detail;
    }
    error.to_string()
}

pub fn unsupported(call: &str) -> String {
    failure(
        Some("unsupported"),
        json!({
            "en": format!("This extension does not answer {call}."),
            "fr": format!("Cette extension ne répond pas à {call}."),
        }),
        None,
    )
}

pub fn no_account(title: Value) -> String {
    failure(Some("account"), title, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_input_with_missing_and_unknown_fields() {
        let found: Input<NumberArgs> =
            input(r#"{"host":"github.com","repo":"acme/shop","args":{"number":7},"later":1}"#)
                .unwrap();
        assert_eq!((found.repo.as_str(), found.args.number), ("acme/shop", 7));
        assert!(found.groups.is_empty());
        let pull: Pull = serde_json::from_str(r#"{"number":3,"soon":true}"#).unwrap();
        assert_eq!((pull.number, pull.checks), (3, None));
    }

    #[test]
    fn writes_errors_the_core_reads() {
        let error: Value = serde_json::from_str(&unsupported("runs")).unwrap();
        assert_eq!(error["code"], "unsupported");
        assert_eq!(error["title"]["en"], "This extension does not answer runs.");
        let error: Value = serde_json::from_str(&no_account(json!("Add a GitHub account"))).unwrap();
        assert_eq!((error["code"].as_str(), error.get("detail")), (Some("account"), None));
    }
}
