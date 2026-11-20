use crate::{azure, git, projects};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Host {
    Azure,
}

#[derive(Debug, PartialEq)]
struct Repo {
    host: Host,
    slug: String,
}

fn repo(url: &str) -> Option<Repo> {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let (host, path) = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?,
        None => url.split_once(':')?,
    };
    let host = host.rsplit('@').next()?.split(':').next()?.to_lowercase();
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let valid = |s: &&str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    let azure = |org: &str, project: &str, name: &str| {
        let named = |s: &str| !s.is_empty() && !s.contains(['?', '#']);
        (valid(&org) && named(project) && named(name)).then(|| Repo {
            host: Host::Azure,
            slug: format!("{org}/{project}/{name}").to_lowercase(),
        })
    };
    match (host.as_str(), parts.as_slice()) {
        ("dev.azure.com", [org, project, "_git", name]) => azure(org, project, name),
        ("ssh.dev.azure.com" | "vs-ssh.visualstudio.com", ["v3", org, project, name]) => {
            azure(org, project, name)
        }
        (site, [.., project, "_git", name]) if site.ends_with(".visualstudio.com") => {
            azure(site.trim_end_matches(".visualstudio.com"), project, name)
        }
        _ => None,
    }
}

pub fn built_in(url: &str) -> Option<&'static str> {
    match repo(url)?.host {
        Host::Azure => Some("azure"),
    }
}

fn repo_of(dir: &Path) -> Result<Repo, String> {
    git::origin(dir)
        .and_then(|url| repo(&url))
        .ok_or_else(|| NOT_HOSTED.into())
}

const NOT_HOSTED: &str = "The project is not on Azure DevOps.";

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

#[derive(Serialize)]
pub struct Accounts {
    pub azure: Option<String>,
}

pub(crate) fn client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}

pub(crate) fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

pub(crate) fn checks<'a>(states: impl Iterator<Item = &'a str>) -> Option<String> {
    let states: Vec<&str> = states.collect();
    let summary = if states.is_empty() {
        return None;
    } else if states.contains(&"fail") {
        "failing"
    } else if states.contains(&"pending") {
        "pending"
    } else {
        "passing"
    };
    Some(summary.into())
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

#[derive(Serialize, Deserialize)]
pub(crate) struct Token {
    pub site: String,
    pub token: String,
    pub name: String,
}

pub(crate) fn token(name: &str, missing: &str) -> Result<Token, String> {
    let secret = read_secret(name)?.ok_or(missing)?;
    serde_json::from_str(&secret).map_err(|e| e.to_string())
}

pub(crate) fn keep_token(name: &str, token: &Token) -> Result<(), String> {
    let secret = serde_json::to_string(token).map_err(|e| e.to_string())?;
    write_secret(name, Some(&secret))
}

pub(crate) async fn send(
    request: reqwest::RequestBuilder,
    host: &str,
    refused: &str,
) -> Result<Value, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::NON_AUTHORITATIVE_INFORMATION
    {
        return Err(refused.into());
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    if !status.is_success() {
        let reason = [
            &value["error"]["message"],
            &value["message"],
            &value["error"],
        ]
        .into_iter()
        .find_map(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| body.chars().take(200).collect());
        return Err(format!("{host} answered {status}: {reason}"));
    }
    Ok(value)
}

pub fn sign_out(name: &str) -> Result<(), String> {
    if name != "azure" {
        return Err(format!("Unknown account: {name}"));
    }
    write_secret(name, None)
}

pub fn accounts() -> Accounts {
    Accounts {
        azure: azure::account(),
    }
}

pub async fn pull(dir: &Path, branch: Option<String>) -> Result<Option<Pull>, String> {
    let repo = repo_of(dir)?;
    let Some(branch) = branch.or_else(|| projects::branch(dir)) else {
        return Ok(None);
    };
    match repo.host {
        Host::Azure => azure::pull(&repo.slug, &branch).await,
    }
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

fn pull_link(url: &str) -> Option<(Repo, u64)> {
    let rest = url.trim().split(['?', '#']).next()?.trim_end_matches('/');
    let rest = rest
        .strip_prefix("https://")
        .or_else(|| rest.strip_prefix("http://"))?;
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    [("/pullrequest/", Host::Azure)]
        .into_iter()
        .find_map(|(marker, host)| {
            let (left, right) = rest.split_once(marker)?;
            let number = right.split('/').next()?.parse().ok()?;
            let found = repo(&format!("https://{left}"))?;
            (found.host == host).then_some((found, number))
        })
}

pub async fn linked(url: &str, projects: Vec<PathBuf>) -> Result<Linked, String> {
    let (repo, number) = pull_link(url).ok_or("Not a pull request link.")?;
    let project = projects
        .into_iter()
        .find(|dir| repo_of(dir).is_ok_and(|r| r == repo));
    by_number(repo, number, project).await
}

pub async fn numbered(dir: &Path, number: u64) -> Result<Pull, String> {
    Ok(by_number(repo_of(dir)?, number, None).await?.pull)
}

async fn by_number(repo: Repo, number: u64, project: Option<PathBuf>) -> Result<Linked, String> {
    let found = match repo.host {
        Host::Azure => azure::linked(&repo.slug, number).await?,
    };
    Ok(Linked {
        repo: repo.slug,
        project,
        ..found
    })
}

pub async fn lists(dir: &Path) -> Result<Lists, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Azure => azure::lists(&repo.slug).await,
    }
}

pub async fn remarks(dir: &Path, number: u64) -> Result<Vec<Remark>, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Azure => azure::remarks(&repo.slug, number).await,
    }
}

pub async fn me(dir: &Path) -> Result<Person, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Azure => azure::me(&repo.slug).await,
    }
}

pub async fn reply(dir: &Path, number: u64, to: u64, body: String) -> Result<(), String> {
    let repo = repo_of(dir)?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err("The reply is empty.".into());
    }
    match repo.host {
        Host::Azure => azure::reply(&repo.slug, number, to, body).await,
    }
}

pub async fn resolve(dir: &Path, number: u64, to: u64, resolved: bool) -> Result<(), String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Azure => azure::resolve(&repo.slug, number, to, resolved).await,
    }
}

pub async fn post(
    dir: &Path,
    number: u64,
    drafts: Vec<Draft>,
    summary: String,
    verdict: String,
) -> Result<Posted, String> {
    let repo = repo_of(dir)?;
    let summary = summary.trim().to_string();
    match repo.host {
        Host::Azure => azure::post(&repo.slug, number, drafts, summary, verdict).await,
    }
}

pub async fn merge(dir: &Path, number: u64) -> Result<(), String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Azure => azure::merge(&repo.slug, number).await,
    }
}

pub async fn update(
    dir: &Path,
    number: u64,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    let repo = repo_of(dir)?;
    let title = title.map(|t| t.trim().to_string());
    if title.as_deref() == Some("") {
        return Err("The title is empty.".into());
    }
    let mut changes = serde_json::Map::new();
    if let Some(title) = title {
        changes.insert("title".into(), title.into());
    }
    if let Some(description) = description {
        changes.insert("description".into(), description.into());
    }
    if changes.is_empty() {
        return Err("Give a new title, a new description, or both.".into());
    }
    match repo.host {
        Host::Azure => azure::update(&repo.slug, number, Value::Object(changes)).await,
    }
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

pub(crate) fn pull_in(reference: &str, prefix: &str) -> Option<u64> {
    reference
        .strip_prefix(prefix)?
        .split('/')
        .next()?
        .parse()
        .ok()
}

fn repo_given(dir: &Path, repository: &str) -> Result<Repo, String> {
    let given = repository.trim();
    if given.is_empty() {
        return repo_of(dir);
    }
    let url = if given.contains("://") || given.contains('@') {
        given.to_string()
    } else {
        format!("https://{given}")
    };
    repo(&url).ok_or_else(|| "The repository is not an Azure DevOps address.".into())
}

pub async fn runs(dir: &Path, repository: &str) -> Result<Vec<Run>, String> {
    let repo = repo_given(dir, repository)?;
    match repo.host {
        Host::Azure => azure::runs(&repo.slug).await,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_pull_request_links_of_both_hosts() {
        assert!(pull_link("https://github.com/agence/orbit/pull/2108").is_none());
        assert!(
            pull_link("https://bitbucket.org/steel-shed/bmc/pull-requests/412/overview").is_none()
        );
        assert!(pull_link("https://gitlab.com/grp/sub/proj/-/merge_requests/7767/diffs").is_none());
        let (repo, number) =
            pull_link("https://dev.azure.com/Org/My%20Proj/_git/Shop/pullrequest/42?_a=files")
                .unwrap();
        assert_eq!(
            (repo.host, repo.slug.as_str(), number),
            (Host::Azure, "org/my%20proj/shop", 42)
        );
    }

    use super::*;

    #[test]
    fn reads_the_repository_of_a_remote() {
        assert_eq!(repo("git@bitbucket.org:akki-team/fileas.git"), None);
        assert_eq!(
            repo("https://jgrasp@bitbucket.org/akki-team/fileas.git"),
            None
        );
        assert_eq!(repo("git@github.com:Jgrasp/squadron.git"), None);
        assert_eq!(repo("git@gitlab.com:grp/sub/proj.git"), None);
        let azure = Some(Repo {
            host: Host::Azure,
            slug: "org/my%20proj/shop".into(),
        });
        for url in [
            "https://org@dev.azure.com/Org/My%20Proj/_git/Shop",
            "git@ssh.dev.azure.com:v3/org/My%20Proj/Shop",
            "https://org.visualstudio.com/DefaultCollection/My%20Proj/_git/Shop",
            "org@vs-ssh.visualstudio.com:v3/org/My%20Proj/Shop",
        ] {
            assert_eq!(repo(url), azure, "{url}");
        }
        assert_eq!(repo("git@github.com:a/b/c.git"), None);
        assert_eq!(repo("git@github.com:a/b?x=1"), None);
    }

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

    #[test]
    fn reads_a_repository_given_by_hand() {
        let dir = Path::new("/nowhere");
        assert!(repo_given(dir, "github.com/Jgrasp/squadron").is_err());
        assert!(repo_given(dir, "git@bitbucket.org:akki-team/fileas.git").is_err());
        assert!(repo_given(dir, "Jgrasp/squadron").is_err());
        assert!(repo_given(dir, "gitlab.com/grp/proj").is_err());
        assert_eq!(
            repo_given(dir, "dev.azure.com/org/proj/_git/shop")
                .unwrap()
                .host,
            Host::Azure
        );
    }
}
