use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "oauthAccount")]
    account: Option<Account>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Account {
    full_name: Option<String>,
    display_name: Option<String>,
}

pub fn first_name(config: &str) -> Option<String> {
    let account = serde_json::from_str::<Config>(config).ok()?.account?;
    [account.full_name, account.display_name]
        .into_iter()
        .flatten()
        .find_map(|name| name.split_whitespace().next().map(String::from))
}

pub fn read_first_name() -> Option<String> {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(std::env::home_dir)?;
    first_name(&std::fs::read_to_string(dir.join(".claude.json")).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_first_word_of_the_full_name() {
        let config = r#"{"oauthAccount":{"fullName":"Quentin Briais","displayName":"Jgrasp"}}"#;
        assert_eq!(first_name(config).as_deref(), Some("Quentin"));
    }

    #[test]
    fn falls_back_on_the_display_name() {
        assert_eq!(
            first_name(r#"{"oauthAccount":{"fullName":" ","displayName":"Jgrasp"}}"#).as_deref(),
            Some("Jgrasp")
        );
        assert_eq!(first_name(r#"{"oauthAccount":{}}"#), None);
        assert_eq!(first_name(r#"{"numStartups":3}"#), None);
    }
}
