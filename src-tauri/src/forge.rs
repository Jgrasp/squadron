use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pull {
    pub host: String,
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

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reviewer {
    pub name: String,
    pub avatar: String,
    pub verdict: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Lists {
    pub review: Vec<Pull>,
    pub mine: Vec<Pull>,
    pub others: Vec<Pull>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Person {
    pub name: String,
    pub avatar: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
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
    pub resolved: Option<bool>,
    pub commit: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Draft {
    pub path: String,
    pub line: u32,
    pub removed: bool,
    pub body: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Posted {
    pub done: usize,
    pub said: bool,
    pub error: Option<String>,
}

pub(crate) fn client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}

fn entry(name: &str) -> Result<keyring::Entry, String> {
    let service = if cfg!(debug_assertions) {
        "squadron-dev"
    } else {
        "squadron"
    };
    keyring::Entry::new(service, name).map_err(|e| e.to_string())
}

const KEYRING_BUSY: &str = "The system keyring is waiting to be unlocked.";

static SECRETS: Mutex<BTreeMap<String, Option<String>>> = Mutex::new(BTreeMap::new());
static READING: Mutex<()> = Mutex::new(());

fn cached(user: &str) -> Option<Option<String>> {
    SECRETS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(user)
        .cloned()
}

fn remember(user: &str, secret: Option<String>) {
    SECRETS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(user.into(), secret);
}

fn blocking_in_place<T>(f: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current().map(|h| h.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(f),
        _ => f(),
    }
}

pub(crate) fn read_secret(user: &str) -> Result<Option<String>, String> {
    if let Some(found) = cached(user) {
        return Ok(found);
    }
    let started = std::time::Instant::now();
    let _reading = loop {
        match READING.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(e)) => break e.into_inner(),
            Err(_) if started.elapsed() > std::time::Duration::from_secs(2) => {
                return Err(KEYRING_BUSY.into());
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    if let Some(found) = cached(user) {
        return Ok(found);
    }
    let found = blocking_in_place(|| match entry(user)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    })?;
    remember(user, found.clone());
    Ok(found)
}

pub(crate) fn write_secret(user: &str, secret: Option<&str>) -> Result<(), String> {
    let entry = entry(user)?;
    match secret {
        Some(secret) => entry.set_password(secret).map_err(|e| e.to_string())?,
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.to_string()),
        },
    }
    remember(user, secret.map(str::to_string));
    Ok(())
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Linked {
    pub repo: String,
    pub pull: Pull,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub files: Option<u64>,
    pub project: Option<PathBuf>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    pub host: String,
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
    pub pull: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git;
    use std::path::Path;

    #[test]
    #[ignore = "fetches the Fileas repository"]
    fn real_pull_request_diff_matches_the_forge() {
        let dir = Path::new("/home/jgrasp/Projects/Symfony/fileas");
        let pulled =
            git::pull_changes(dir, "feature/FIL-671", "dev/FIL-671", "1094c23e6813", None).unwrap();
        assert_eq!(pulled.changes.files.len(), 60);
        assert_eq!(
            pulled
                .changes
                .files
                .iter()
                .map(|f| f.add.unwrap_or(0))
                .sum::<u32>(),
            2856
        );
        let parent = String::from_utf8(
            std::process::Command::new("git")
                .current_dir(dir)
                .args(["rev-parse", "1094c23e6813~1"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let since = git::pull_changes(
            dir,
            "feature/FIL-671",
            "dev/FIL-671",
            "1094c23e6813",
            Some(parent.trim()),
        )
        .unwrap();
        assert_eq!(since.added, 1);
        assert!(since.changes.files.len() < pulled.changes.files.len());
        let rewritten = git::pull_changes(
            dir,
            "feature/FIL-671",
            "dev/FIL-671",
            "1094c23e6813",
            Some("21782a2a10b2"),
        );
        assert!(rewritten.is_err());
        let by_time = git::pull_changes(
            dir,
            "feature/FIL-671",
            "dev/FIL-671",
            "1094c23e6813",
            Some("2026-10-23T12:00:00Z"),
        )
        .unwrap();
        assert_eq!((by_time.added, by_time.changes.files.len()), (0, 0));
    }

    #[test]
    #[ignore = "uses the system keyring"]
    fn real_keyring_keeps_a_secret() {
        let entry = keyring::Entry::new("squadron", "keyring-check").unwrap();
        entry.set_password("dummy").unwrap();
        assert_eq!(entry.get_password().unwrap(), "dummy");
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
    }

    #[test]
    fn a_read_gives_up_while_another_waits_on_the_keyring_unless_cached() {
        remember("cached-check", Some("kept".into()));
        let _waiting = READING.lock().unwrap();
        assert_eq!(
            read_secret("cached-check").unwrap().as_deref(),
            Some("kept")
        );
        assert_eq!(read_secret("uncached-check").unwrap_err(), KEYRING_BUSY);
    }
}
