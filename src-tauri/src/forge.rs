use crate::{azure, git, gitlab, projects};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

const BITBUCKET: &str = "https://api.bitbucket.org/2.0";
const NO_TOKEN: &str = "Add a Bitbucket API token in Settings › Code hosting.";
const NO_USER_SCOPE: &str =
    "Add the read:user:bitbucket scope to the Bitbucket API token to list your pull requests.";
const BITBUCKET_FIELDS: &str = "values.id,values.title,values.description,values.state,values.draft,values.comment_count,values.links.html.href,values.source.branch.name,values.source.commit.hash,values.destination.branch.name,values.author.display_name,values.author.links.avatar.href,values.updated_on,values.participants.approved,values.participants.state,values.participants.role,values.participants.participated_on,values.participants.user.uuid,values.participants.user.display_name,values.participants.user.links.avatar.href,values.reviewers.uuid,values.reviewers.display_name,values.reviewers.links.avatar.href";
const GITHUB_FIELDS: &str = "number,title,body,url,state,isDraft,reviewDecision,latestReviews,reviewRequests,comments,statusCheckRollup,baseRefName,headRefName,headRefOid,author,updatedAt,mergeable,mergeStateStatus";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Host {
    Bitbucket,
    GitHub,
    GitLab,
    Azure,
}

#[derive(Debug, PartialEq)]
struct Repo {
    host: Host,
    slug: String,
}

fn repo(url: &str) -> Option<Repo> {
    parse(url, gitlab::host)
}

fn parse(url: &str, gitlab: impl Fn() -> Option<String>) -> Option<Repo> {
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
        ("bitbucket.org" | "ssh.bitbucket.org", [owner, name]) if valid(owner) && valid(name) => {
            Some(Repo {
                host: Host::Bitbucket,
                slug: format!("{owner}/{name}"),
            })
        }
        ("github.com", [owner, name]) if valid(owner) && valid(name) => Some(Repo {
            host: Host::GitHub,
            slug: format!("{owner}/{name}"),
        }),
        ("dev.azure.com", [org, project, "_git", name]) => azure(org, project, name),
        ("ssh.dev.azure.com" | "vs-ssh.visualstudio.com", ["v3", org, project, name]) => {
            azure(org, project, name)
        }
        (site, [.., project, "_git", name]) if site.ends_with(".visualstudio.com") => {
            azure(site.trim_end_matches(".visualstudio.com"), project, name)
        }
        (site, [_, _, ..])
            if parts.iter().all(valid)
                && (site == "gitlab.com" || gitlab().is_some_and(|saved| saved == site)) =>
        {
            Some(Repo {
                host: Host::GitLab,
                slug: format!("{site}/{}", parts.join("/")),
            })
        }
        _ => None,
    }
}

fn repo_of(dir: &Path) -> Result<Repo, String> {
    git::origin(dir)
        .and_then(|url| repo(&url))
        .ok_or_else(|| NOT_HOSTED.into())
}

const NOT_HOSTED: &str = "The project is not on Bitbucket, GitHub, GitLab or Azure DevOps.";

#[derive(Debug, Default, PartialEq, Serialize)]
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

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Reviewer {
    pub name: String,
    pub avatar: String,
    pub verdict: String,
}

#[derive(Serialize)]
pub struct Lists {
    pub review: Vec<Pull>,
    pub mine: Vec<Pull>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Person {
    pub name: String,
    pub avatar: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Remark {
    pub id: u64,
    pub author: String,
    pub avatar: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub removed: bool,
    pub reply_to: Option<u64>,
    pub at: String,
    pub kind: String,
}

#[derive(Deserialize)]
pub struct Draft {
    pub path: String,
    pub line: u32,
    pub removed: bool,
    pub body: String,
}

#[derive(Serialize)]
pub struct Posted {
    pub done: usize,
    pub said: bool,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Accounts {
    pub bitbucket: Option<String>,
    pub bitbucket_user: bool,
    pub github: Option<String>,
    pub gitlab: Option<String>,
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

#[derive(Serialize, Deserialize)]
struct Account {
    email: String,
    token: String,
    #[serde(default)]
    uuid: Option<String>,
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
    SECRETS.lock().unwrap_or_else(|e| e.into_inner()).get(user).cloned()
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

fn account() -> Result<Account, String> {
    let secret = read_secret("bitbucket")?.ok_or(NO_TOKEN)?;
    serde_json::from_str(&secret).map_err(|e| e.to_string())
}

pub async fn sign_in(email: String, token: String) -> Result<(), String> {
    let response = client()
        .get(format!("{BITBUCKET}/user"))
        .basic_auth(&email, Some(&token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Bitbucket refused this email and token.".into());
    }
    let uuid = if response.status().is_success() {
        let user: Value = response.json().await.unwrap_or_default();
        user["uuid"].as_str().map(str::to_string)
    } else {
        None
    };
    keep(&Account { email, token, uuid })
}

fn keep(account: &Account) -> Result<(), String> {
    let secret = serde_json::to_string(account).map_err(|e| e.to_string())?;
    write_secret("bitbucket", Some(&secret))
}

async fn bitbucket_me() -> Result<String, String> {
    let mut account = account()?;
    if let Some(uuid) = &account.uuid {
        return Ok(uuid.clone());
    }
    let user = bitbucket(Method::GET, "/user", &[("fields", "uuid")], None)
        .await
        .map_err(|e| {
            if e.contains("403") {
                NO_USER_SCOPE.into()
            } else {
                e
            }
        })?;
    let uuid = user["uuid"].as_str().ok_or(NO_USER_SCOPE)?.to_string();
    account.uuid = Some(uuid.clone());
    keep(&account)?;
    Ok(uuid)
}

pub fn sign_out(name: &str) -> Result<(), String> {
    if !["bitbucket", "gitlab", "azure"].contains(&name) {
        return Err(format!("Unknown account: {name}"));
    }
    write_secret(name, None)
}

pub fn accounts() -> Accounts {
    let found = account().ok();
    Accounts {
        bitbucket_user: found.as_ref().is_some_and(|a| a.uuid.is_some()),
        bitbucket: found.map(|a| a.email),
        github: gh(&["api", "user", "--jq", ".login"], None)
            .ok()
            .map(|login| login.trim().to_string())
            .filter(|login| !login.is_empty()),
        gitlab: gitlab::account(),
        azure: azure::account(),
    }
}

async fn bitbucket(
    method: Method,
    url: &str,
    query: &[(&str, &str)],
    body: Option<Value>,
) -> Result<Value, String> {
    let account = account()?;
    let url = if url.starts_with(BITBUCKET) {
        url.to_string()
    } else {
        format!("{BITBUCKET}{url}")
    };
    let url = reqwest::Url::parse_with_params(&url, query).map_err(|e| e.to_string())?;
    let mut request = client()
        .request(method, url)
        .basic_auth(account.email, Some(account.token));
    if let Some(body) = body {
        request = request.json(&body);
    }
    send(
        request,
        "Bitbucket",
        "Bitbucket refused the API token: add a new one in Settings › Code hosting.",
    )
    .await
}

static GH_TOKEN: Mutex<Option<String>> = Mutex::new(None);

fn gh_token() -> Option<String> {
    let mut token = GH_TOKEN.lock().unwrap_or_else(|e| e.into_inner());
    if token.is_none() {
        *token = crate::process::command("gh")
            .args(["auth", "token"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|token| !token.is_empty());
    }
    token.clone()
}

fn gh(args: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut command = crate::process::command("gh");
    if let Some(token) = gh_token() {
        command.env("GH_TOKEN", token);
    }
    let mut child = command
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Install the GitHub CLI and run gh auth login to see pull requests.")?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.unwrap_or_default().as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let error = gh_error(&out.stderr, &out.stdout);
        if error.contains("HTTP 401") {
            *GH_TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        return Err(error);
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn gh_error(stderr: &[u8], stdout: &[u8]) -> String {
    let said = String::from_utf8_lossy(stderr).trim().to_string();
    let body: Value = serde_json::from_slice(stdout).unwrap_or_default();
    let details: Vec<&str> = body["errors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["message"].as_str().or(e.as_str()))
        .filter(|detail| !said.contains(detail))
        .collect();
    if details.is_empty() {
        said
    } else {
        format!("{said}: {}", details.join("; "))
    }
}

fn lines(out: &str) -> Vec<Value> {
    out.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

pub async fn pull(dir: &Path, branch: Option<String>) -> Result<Option<Pull>, String> {
    let repo = repo_of(dir)?;
    let Some(branch) = branch.or_else(|| projects::branch(dir)) else {
        return Ok(None);
    };
    match repo.host {
        Host::GitLab => gitlab::pull(&repo.slug, &branch).await,
        Host::Azure => azure::pull(&repo.slug, &branch).await,
        Host::Bitbucket => {
            let q = format!(
                "source.branch.name=\"{}\"",
                branch.replace('\\', "\\\\").replace('"', "\\\"")
            );
            let list = bitbucket(
                Method::GET,
                &format!("/repositories/{}/pullrequests", repo.slug),
                &[
                    ("q", &q),
                    ("state", "OPEN"),
                    ("state", "MERGED"),
                    ("state", "DECLINED"),
                    ("sort", "-updated_on"),
                    ("pagelen", "1"),
                    ("fields", BITBUCKET_FIELDS),
                ],
                None,
            )
            .await?;
            let Some(found) = list["values"].get(0) else {
                return Ok(None);
            };
            let statuses = bitbucket(
                Method::GET,
                &format!(
                    "/repositories/{}/pullrequests/{}/statuses",
                    repo.slug, found["id"]
                ),
                &[("fields", "values.state"), ("pagelen", "50")],
                None,
            )
            .await
            .unwrap_or_default();
            let me = account().ok().and_then(|a| a.uuid).unwrap_or_default();
            Ok(Some(bitbucket_pull(found, &statuses, &me)))
        }
        Host::GitHub => {
            blocking(move || {
                let out = gh(
                    &[
                        "pr",
                        "list",
                        "--repo",
                        &repo.slug,
                        "--head",
                        &branch,
                        "--state",
                        "all",
                        "--limit",
                        "1",
                        "--json",
                        GITHUB_FIELDS,
                    ],
                    None,
                )?;
                let list: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
                Ok(list.get(0).map(|found| github_pull(found, "")))
            })
            .await
        }
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
    [
        ("/-/merge_requests/", Host::GitLab),
        ("/pullrequest/", Host::Azure),
        ("/pull-requests/", Host::Bitbucket),
        ("/pull/", Host::GitHub),
    ]
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
    match repo.host {
        Host::GitLab | Host::Azure => {
            let found = if repo.host == Host::GitLab {
                gitlab::linked(&repo.slug, number).await?
            } else {
                azure::linked(&repo.slug, number).await?
            };
            Ok(Linked {
                repo: repo.slug,
                project,
                ..found
            })
        }
        Host::Bitbucket => {
            let path = format!("/repositories/{}/pullrequests/{number}", repo.slug);
            let fields = BITBUCKET_FIELDS.replace("values.", "");
            let found = bitbucket(Method::GET, &path, &[("fields", &fields)], None).await?;
            let statuses = bitbucket(
                Method::GET,
                &format!("{path}/statuses"),
                &[("fields", "values.state"), ("pagelen", "50")],
                None,
            )
            .await
            .unwrap_or_default();
            let stat = bitbucket(
                Method::GET,
                &format!("{path}/diffstat"),
                &[
                    ("fields", "values.lines_added,values.lines_removed"),
                    ("pagelen", "500"),
                ],
                None,
            )
            .await
            .ok();
            let files = stat.as_ref().and_then(|s| s["values"].as_array());
            let sum = |key: &str| files.map(|f| f.iter().filter_map(|x| x[key].as_u64()).sum());
            let me = account().ok().and_then(|a| a.uuid).unwrap_or_default();
            Ok(Linked {
                repo: repo.slug,
                pull: bitbucket_pull(&found, &statuses, &me),
                additions: sum("lines_added"),
                deletions: sum("lines_removed"),
                files: files.map(|f| f.len() as u64),
                project,
            })
        }
        Host::GitHub => {
            blocking(move || {
                let fields = format!("{GITHUB_FIELDS},additions,deletions,changedFiles");
                let out = gh(
                    &[
                        "pr",
                        "view",
                        &number.to_string(),
                        "--repo",
                        &repo.slug,
                        "--json",
                        &fields,
                    ],
                    None,
                )?;
                let found: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
                Ok(Linked {
                    pull: github_pull(&found, ""),
                    additions: found["additions"].as_u64(),
                    deletions: found["deletions"].as_u64(),
                    files: found["changedFiles"].as_u64(),
                    repo: repo.slug,
                    project,
                })
            })
            .await
        }
    }
}

pub async fn lists(dir: &Path) -> Result<Lists, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::GitLab => gitlab::lists(&repo.slug).await,
        Host::Azure => azure::lists(&repo.slug).await,
        Host::Bitbucket => {
            let me = bitbucket_me().await?;
            let url = format!("/repositories/{}/pullrequests", repo.slug);
            let mut found = Vec::new();
            for who in ["reviewers", "author"] {
                let q = format!("{who}.uuid=\"{me}\" AND state=\"OPEN\"");
                let page = bitbucket(
                    Method::GET,
                    &url,
                    &[
                        ("q", &q),
                        ("sort", "-updated_on"),
                        ("pagelen", "50"),
                        ("fields", BITBUCKET_FIELDS),
                    ],
                    None,
                )
                .await?;
                found.push(page["values"].as_array().cloned().unwrap_or_default());
            }
            let mut mine = Vec::new();
            for pull in &found[1] {
                let statuses = bitbucket(
                    Method::GET,
                    &format!("{url}/{}/statuses", pull["id"]),
                    &[("fields", "values.state"), ("pagelen", "50")],
                    None,
                )
                .await
                .unwrap_or_default();
                mine.push(bitbucket_pull(pull, &statuses, &me));
            }
            let mut review = Vec::new();
            for found in &found[0] {
                let mut pull = bitbucket_pull(found, &Value::Null, &me);
                if pull.reviewed_at.is_some() {
                    pull.head_at = bitbucket(
                        Method::GET,
                        &format!("/repositories/{}/commit/{}", repo.slug, pull.head),
                        &[("fields", "date")],
                        None,
                    )
                    .await
                    .ok()
                    .and_then(|c| c["date"].as_str().map(str::to_string));
                }
                review.push(pull);
            }
            Ok(Lists { review, mine })
        }
        Host::GitHub => {
            blocking(move || {
                let me = gh(&["api", "user", "--jq", ".login"], None)?
                    .trim()
                    .to_string();
                let list = |query: &str| -> Result<Vec<Value>, String> {
                    let mut args = vec![
                        "pr",
                        "list",
                        "--repo",
                        repo.slug.as_str(),
                        "--state",
                        "open",
                        "--limit",
                        "50",
                        "--json",
                        GITHUB_FIELDS,
                    ];
                    if !query.is_empty() {
                        args.extend(["--search", query]);
                    }
                    let out = gh(&args, None)?;
                    let list: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
                    Ok(list.as_array().cloned().unwrap_or_default())
                };
                let search = |query: &str| -> Result<Vec<Pull>, String> {
                    Ok(list(query)?.iter().map(|p| github_pull(p, &me)).collect())
                };
                let mut review = search("review-requested:@me")?;
                // GitHub's search index can lag hours behind a new review request.
                for found in list("")?.iter().filter(|p| {
                    p["reviewRequests"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|r| r["login"] == me.as_str())
                }) {
                    if !review
                        .iter()
                        .any(|p| Some(p.number) == found["number"].as_u64())
                    {
                        review.push(github_pull(found, &me));
                    }
                }
                review.iter_mut().for_each(|p| p.reviewed = None);
                for pull in search("reviewed-by:@me -author:@me")? {
                    if !review.iter().any(|p| p.number == pull.number) {
                        review.push(pull);
                    }
                }
                for pull in review.iter_mut().filter(|p| p.reviewed.is_some()) {
                    let jq = format!(
                        "[.[] | select(.user.login == \"{me}\")] | last | .commit_id // empty"
                    );
                    pull.reviewed_head = gh(
                        &[
                            "api",
                            &format!("repos/{}/pulls/{}/reviews", repo.slug, pull.number),
                            "--paginate",
                            "--jq",
                            &jq,
                        ],
                        None,
                    )
                    .ok()
                    .map(|sha| sha.trim().to_string())
                    .filter(|sha| !sha.is_empty());
                }
                Ok(Lists {
                    review,
                    mine: search("author:@me")?,
                })
            })
            .await
        }
    }
}

fn bitbucket_verdict(person: &Value) -> &'static str {
    match (person["approved"] == true, person["state"].as_str()) {
        (true, _) => "approved",
        (_, Some("changes_requested")) => "changes_requested",
        _ if person["participated_on"].is_string() => "commented",
        _ => "waiting",
    }
}

fn bitbucket_pull(found: &Value, statuses: &Value, me: &str) -> Pull {
    let people = found["participants"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let asked = found["reviewers"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut reviewers: Vec<Reviewer> = asked
        .iter()
        .map(|user| Reviewer {
            name: text(&user["display_name"]),
            avatar: text(&user["links"]["avatar"]["href"]),
            verdict: people
                .iter()
                .find(|p| p["user"]["uuid"] == user["uuid"])
                .map_or("waiting", bitbucket_verdict)
                .into(),
        })
        .collect();
    for p in people {
        let listed = asked.iter().any(|u| u["uuid"] == p["user"]["uuid"]);
        if !listed && (p["role"] == "REVIEWER" || bitbucket_verdict(p) != "waiting") {
            reviewers.push(Reviewer {
                name: text(&p["user"]["display_name"]),
                avatar: text(&p["user"]["links"]["avatar"]["href"]),
                verdict: bitbucket_verdict(p).into(),
            });
        }
    }
    let states = statuses["values"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| match s["state"].as_str() {
            Some("SUCCESSFUL") => "pass",
            Some("INPROGRESS") => "pending",
            _ => "fail",
        });
    let checks = checks(states);
    let approvals = people.iter().filter(|p| p["approved"] == true).count() as u32;
    let changes_requested = people.iter().any(|p| p["state"] == "changes_requested");
    let draft = found["draft"].as_bool().unwrap_or(false);
    Pull {
        host: "bitbucket".into(),
        number: found["id"].as_u64().unwrap_or_default(),
        title: text(&found["title"]),
        description: text(&found["description"]),
        url: text(&found["links"]["html"]["href"]),
        state: text(&found["state"]).to_lowercase(),
        draft,
        approvals,
        changes_requested,
        comments: found["comment_count"].as_u64().unwrap_or_default() as u32,
        mergeable: found["state"] == "OPEN"
            && !draft
            && !changes_requested
            && approvals > 0
            && checks.as_deref().is_none_or(|c| c == "passing"),
        checks,
        branch: text(&found["source"]["branch"]["name"]),
        base: text(&found["destination"]["branch"]["name"]),
        head: text(&found["source"]["commit"]["hash"]),
        author: text(&found["author"]["display_name"]),
        author_avatar: text(&found["author"]["links"]["avatar"]["href"]),
        updated: text(&found["updated_on"]),
        reviewed: people
            .iter()
            .find(|p| !me.is_empty() && p["user"]["uuid"] == me)
            .map(bitbucket_verdict)
            .filter(|v| *v != "waiting")
            .map(str::to_string),
        reviewed_head: None,
        reviewed_at: people
            .iter()
            .find(|p| !me.is_empty() && p["user"]["uuid"] == me)
            .and_then(|p| p["participated_on"].as_str())
            .map(str::to_string),
        head_at: None,
        reviewers,
        conflicts: false,
    }
}

fn github_pull(found: &Value, me: &str) -> Pull {
    let states = found["statusCheckRollup"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            let state = [&c["conclusion"], &c["state"], &c["status"]]
                .into_iter()
                .filter_map(Value::as_str)
                .find(|s| !s.is_empty());
            match state {
                Some("SUCCESS" | "NEUTRAL" | "SKIPPED") => "pass",
                Some(
                    "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED"
                    | "STARTUP_FAILURE",
                ) => "fail",
                _ => "pending",
            }
        });
    let reviews = found["latestReviews"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    Pull {
        host: "github".into(),
        number: found["number"].as_u64().unwrap_or_default(),
        title: text(&found["title"]),
        description: text(&found["body"]),
        url: text(&found["url"]),
        state: text(&found["state"]).to_lowercase(),
        draft: found["isDraft"].as_bool().unwrap_or(false),
        approvals: reviews.iter().filter(|r| r["state"] == "APPROVED").count() as u32,
        changes_requested: found["reviewDecision"] == "CHANGES_REQUESTED",
        comments: found["comments"].as_array().map_or(0, Vec::len) as u32,
        checks: checks(states),
        branch: text(&found["headRefName"]),
        base: text(&found["baseRefName"]),
        head: text(&found["headRefOid"]),
        author: text(&found["author"]["login"]),
        author_avatar: found["author"]["login"]
            .as_str()
            .map(|login| format!("https://avatars.githubusercontent.com/{login}?s=64"))
            .unwrap_or_default(),
        updated: text(&found["updatedAt"]),
        reviewed: reviews
            .iter()
            .find(|r| !me.is_empty() && r["author"]["login"] == me)
            .and_then(|r| match r["state"].as_str() {
                Some("APPROVED") => Some("approved".into()),
                Some("CHANGES_REQUESTED") => Some("changes_requested".into()),
                Some("COMMENTED") => Some("commented".into()),
                _ => None,
            }),
        reviewed_head: None,
        reviewed_at: reviews
            .iter()
            .find(|r| !me.is_empty() && r["author"]["login"] == me)
            .and_then(|r| r["submittedAt"].as_str())
            .map(str::to_string),
        head_at: None,
        reviewers: github_reviewers(found),
        conflicts: found["mergeable"] == "CONFLICTING",
        mergeable: found["state"] == "OPEN"
            && found["isDraft"] != true
            && matches!(
                found["mergeStateStatus"].as_str(),
                Some("CLEAN" | "HAS_HOOKS" | "UNSTABLE")
            ),
    }
}

fn github_reviewers(found: &Value) -> Vec<Reviewer> {
    let avatar = |login: &str| format!("https://avatars.githubusercontent.com/{login}?s=64");
    let mut list: Vec<Reviewer> = Vec::new();
    for review in found["latestReviews"].as_array().into_iter().flatten() {
        let login = text(&review["author"]["login"]);
        let verdict = match review["state"].as_str() {
            Some("APPROVED") => "approved",
            Some("CHANGES_REQUESTED") => "changes_requested",
            Some("COMMENTED") => "commented",
            _ => continue,
        };
        list.push(Reviewer {
            avatar: avatar(&login),
            name: login,
            verdict: verdict.into(),
        });
    }
    for asked in found["reviewRequests"].as_array().into_iter().flatten() {
        let login = asked["login"].as_str().unwrap_or_default();
        let name = asked["name"]
            .as_str()
            .filter(|n| !n.is_empty())
            .unwrap_or(login);
        list.retain(|r| r.name != login);
        list.push(Reviewer {
            name: name.into(),
            avatar: if login.is_empty() {
                String::new()
            } else {
                avatar(login)
            },
            verdict: "waiting".into(),
        });
    }
    list
}

pub async fn remarks(dir: &Path, number: u64) -> Result<Vec<Remark>, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::GitLab => gitlab::remarks(&repo.slug, number).await,
        Host::Azure => azure::remarks(&repo.slug, number).await,
        Host::Bitbucket => {
            let mut url = format!("/repositories/{}/pullrequests/{number}/comments", repo.slug);
            let mut found = Vec::new();
            loop {
                let page = bitbucket(
                    Method::GET,
                    &url,
                    if url.starts_with(BITBUCKET) {
                        &[]
                    } else {
                        &[("pagelen", "100"), ("fields", "next,values.id,values.content.raw,values.user.display_name,values.user.links.avatar.href,values.inline,values.parent.id,values.created_on,values.deleted")]
                    },
                    None,
                )
                .await?;
                found.extend(
                    page["values"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(bitbucket_remark),
                );
                match page["next"].as_str() {
                    Some(next) if next.starts_with(BITBUCKET) && found.len() < 1000 => {
                        url = next.to_string()
                    }
                    _ => return Ok(found),
                }
            }
        }
        Host::GitHub => {
            blocking(move || {
                let slug = &repo.slug;
                let mut found: Vec<Remark> = Vec::new();
                for (kind, jq) in [
                    (
                        "pulls",
                        ".[] | {id, body, user, path, line, side, in_reply_to_id, created_at}",
                    ),
                    ("issues", ".[] | {id, body, user, created_at}"),
                    (
                        "reviews",
                        ".[] | select(.body != \"\") | {id, body, user, created_at: .submitted_at}",
                    ),
                ] {
                    let api = match kind {
                        "issues" => format!("repos/{slug}/issues/{number}/comments"),
                        "reviews" => format!("repos/{slug}/pulls/{number}/reviews"),
                        _ => format!("repos/{slug}/pulls/{number}/comments"),
                    };
                    found.extend(
                        lines(&gh(&["api", &api, "--paginate", "--jq", jq], None)?)
                            .iter()
                            .map(|found| github_remark(found, kind)),
                    );
                }
                found.sort_by(|a, b| a.at.cmp(&b.at));
                Ok(found)
            })
            .await
        }
    }
}

pub async fn me(dir: &Path) -> Result<Person, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::GitLab => gitlab::me(&repo.slug).await,
        Host::Azure => azure::me(&repo.slug).await,
        Host::Bitbucket => bitbucket(
            Method::GET,
            "/user",
            &[("fields", "display_name,links.avatar.href")],
            None,
        )
        .await
        .map(|user| person(&user, "display_name", &user["links"]["avatar"]["href"])),
        Host::GitHub => {
            blocking(|| {
                let user: Value = serde_json::from_str(&gh(&["api", "user"], None)?)
                    .map_err(|e| e.to_string())?;
                Ok(person(&user, "login", &user["avatar_url"]))
            })
            .await
        }
    }
}

fn person(user: &Value, name: &str, avatar: &Value) -> Person {
    Person {
        name: text(&user[name]),
        avatar: text(avatar),
    }
}

fn bitbucket_remark(found: &Value) -> Option<Remark> {
    if found["deleted"] == true {
        return None;
    }
    let inline = &found["inline"];
    let (to, from) = (inline["to"].as_u64(), inline["from"].as_u64());
    let current = inline["outdated"] != true;
    Some(Remark {
        id: found["id"].as_u64()?,
        author: text(&found["user"]["display_name"]),
        avatar: text(&found["user"]["links"]["avatar"]["href"]),
        body: text(&found["content"]["raw"]),
        path: inline["path"].as_str().map(str::to_string),
        line: to.or(from).filter(|_| current).map(|n| n as u32),
        removed: to.is_none() && from.is_some(),
        reply_to: found["parent"]["id"].as_u64(),
        at: text(&found["created_on"]),
        kind: if inline["path"].is_string() {
            "line"
        } else {
            "general"
        }
        .into(),
    })
}

fn github_remark(found: &Value, kind: &str) -> Remark {
    Remark {
        id: found["id"].as_u64().unwrap_or_default(),
        author: text(&found["user"]["login"]),
        avatar: text(&found["user"]["avatar_url"]),
        body: text(&found["body"]),
        path: found["path"].as_str().map(str::to_string),
        line: found["line"].as_u64().map(|n| n as u32),
        removed: found["side"] == "LEFT",
        reply_to: found["in_reply_to_id"].as_u64(),
        at: text(&found["created_at"]),
        kind: if kind == "pulls" { "line" } else { "general" }.into(),
    }
}

pub async fn reply(
    dir: &Path,
    number: u64,
    to: u64,
    kind: String,
    body: String,
) -> Result<(), String> {
    let repo = repo_of(dir)?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err("The reply is empty.".into());
    }
    match repo.host {
        Host::GitLab => gitlab::reply(&repo.slug, number, to, body).await,
        Host::Azure => azure::reply(&repo.slug, number, to, body).await,
        Host::Bitbucket => bitbucket(
            Method::POST,
            &format!("/repositories/{}/pullrequests/{number}/comments", repo.slug),
            &[],
            Some(json!({ "content": { "raw": body }, "parent": { "id": to } })),
        )
        .await
        .map(|_| ()),
        Host::GitHub => {
            blocking(move || {
                let api = if kind == "line" {
                    format!("repos/{}/pulls/{number}/comments/{to}/replies", repo.slug)
                } else {
                    format!("repos/{}/issues/{number}/comments", repo.slug)
                };
                gh(
                    &["api", "-X", "POST", &api, "--input", "-"],
                    Some(&json!({ "body": body }).to_string()),
                )
                .map(|_| ())
            })
            .await
        }
    }
}

pub async fn post(
    dir: &Path,
    number: u64,
    head: String,
    drafts: Vec<Draft>,
    summary: String,
    verdict: String,
) -> Result<Posted, String> {
    let repo = repo_of(dir)?;
    let summary = summary.trim().to_string();
    match repo.host {
        Host::GitLab => gitlab::post(&repo.slug, number, drafts, summary, verdict).await,
        Host::Azure => azure::post(&repo.slug, number, drafts, summary, verdict).await,
        Host::Bitbucket => {
            let url = format!("/repositories/{}/pullrequests/{number}/comments", repo.slug);
            let total = drafts.len();
            for (done, draft) in drafts.into_iter().enumerate() {
                let mut comment = json!({ "content": { "raw": draft.body } });
                if !draft.path.is_empty() {
                    comment["inline"] = json!({ "path": draft.path });
                    comment["inline"][if draft.removed { "from" } else { "to" }] = json!(draft.line);
                }
                let sent = bitbucket(Method::POST, &url, &[], Some(comment))
                .await;
                if let Err(error) = sent {
                    return Ok(Posted {
                        done,
                        said: false,
                        error: Some(error),
                    });
                }
            }
            let mut sent = if summary.is_empty() {
                Ok(Value::Null)
            } else {
                bitbucket(Method::POST, &url, &[], Some(json!({ "content": { "raw": summary } }))).await
            };
            let said = sent.is_ok();
            let decision = match verdict.as_str() {
                "approve" => Some("approve"),
                "request_changes" => Some("request-changes"),
                _ => None,
            };
            if let (Ok(_), Some(decision)) = (&sent, decision) {
                let pull = format!("/repositories/{}/pullrequests/{number}/{decision}", repo.slug);
                sent = bitbucket(Method::POST, &pull, &[], None).await;
            }
            Ok(Posted {
                done: total,
                said,
                error: sent.err(),
            })
        }
        Host::GitHub => blocking(move || {
            let comments: Vec<Value> = drafts
                .iter()
                .filter(|d| !d.path.is_empty())
                .map(|d| json!({ "path": d.path, "line": d.line, "side": if d.removed { "LEFT" } else { "RIGHT" }, "body": d.body }))
                .collect();
            let summary = std::iter::once(summary.as_str())
                .chain(drafts.iter().filter(|d| d.path.is_empty()).map(|d| d.body.as_str()))
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            let event = match verdict.as_str() {
                "approve" => "APPROVE",
                "request_changes" => "REQUEST_CHANGES",
                _ => "COMMENT",
            };
            if event == "COMMENT" && comments.is_empty() && summary.is_empty() {
                return Ok(Posted {
                    done: 0,
                    said: true,
                    error: None,
                });
            }
            let review = json!({ "commit_id": head, "event": event, "body": summary, "comments": comments });
            let sent = gh(
                &["api", "-X", "POST", &format!("repos/{}/pulls/{number}/reviews", repo.slug), "--input", "-"],
                Some(&review.to_string()),
            );
            Ok(Posted {
                done: if sent.is_ok() { drafts.len() } else { 0 },
                said: sent.is_ok(),
                error: sent.err(),
            })
        })
        .await,
    }
}

pub async fn merge(dir: &Path, number: u64) -> Result<(), String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::GitLab => gitlab::merge(&repo.slug, number).await,
        Host::Azure => azure::merge(&repo.slug, number).await,
        Host::Bitbucket => bitbucket(
            Method::POST,
            &format!("/repositories/{}/pullrequests/{number}/merge", repo.slug),
            &[],
            Some(json!({})),
        )
        .await
        .map(|_| ()),
        Host::GitHub => {
            blocking(move || {
                let method = gh(
                    &[
                        "repo",
                        "view",
                        &repo.slug,
                        "--json",
                        "viewerDefaultMergeMethod",
                        "--jq",
                        ".viewerDefaultMergeMethod",
                    ],
                    None,
                )?;
                let flag = match method.trim() {
                    "SQUASH" => "--squash",
                    "REBASE" => "--rebase",
                    _ => "--merge",
                };
                gh(
                    &[
                        "pr",
                        "merge",
                        &number.to_string(),
                        "--repo",
                        &repo.slug,
                        flag,
                    ],
                    None,
                )
                .map(|_| ())
            })
            .await
        }
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
        Host::GitLab => gitlab::update(&repo.slug, number, Value::Object(changes)).await,
        Host::Azure => azure::update(&repo.slug, number, Value::Object(changes)).await,
        Host::Bitbucket => {
            let path = format!("/repositories/{}/pullrequests/{number}", repo.slug);
            let found = bitbucket(
                Method::GET,
                &path,
                &[("fields", "title,reviewers.uuid")],
                None,
            )
            .await?;
            changes
                .entry("title")
                .or_insert_with(|| found["title"].clone());
            changes.insert("reviewers".into(), found["reviewers"].clone());
            bitbucket(Method::PUT, &path, &[], Some(Value::Object(changes)))
                .await
                .map(|_| ())
        }
        Host::GitHub => {
            if let Some(description) = changes.remove("description") {
                changes.insert("body".into(), description);
            }
            blocking(move || {
                gh(
                    &[
                        "api",
                        "-X",
                        "PATCH",
                        &format!("repos/{}/pulls/{number}", repo.slug),
                        "--input",
                        "-",
                    ],
                    Some(&Value::Object(changes).to_string()),
                )
                .map(|_| ())
            })
            .await
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize)]
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
}

const NO_PIPELINE_SCOPE: &str =
    "Add the read:pipeline:bitbucket scope to the Bitbucket API token to see pipelines.";

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
    repo(&url).ok_or_else(|| {
        "The repository is not a Bitbucket, GitHub, GitLab or Azure DevOps address.".into()
    })
}

pub async fn runs(dir: &Path, repository: &str) -> Result<Vec<Run>, String> {
    let repo = repo_given(dir, repository)?;
    match repo.host {
        Host::GitLab => gitlab::runs(&repo.slug).await,
        Host::Azure => azure::runs(&repo.slug).await,
        Host::Bitbucket => {
            let list = bitbucket(
                Method::GET,
                &format!("/repositories/{}/pipelines", repo.slug),
                &[
                    ("sort", "-created_on"),
                    ("pagelen", "20"),
                    ("fields", "values.uuid,values.build_number,values.state.name,values.state.result.name,values.state.stage.name,values.target.ref_name,values.target.selector.type,values.target.selector.pattern,values.target.commit.message,values.trigger.name,values.creator.display_name,values.creator.links.avatar.href,values.created_on,values.completed_on"),
                ],
                None,
            )
            .await
            .map_err(|e| if e.contains("403") { NO_PIPELINE_SCOPE.into() } else { e })?;
            Ok(list["values"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|value| bitbucket_run(value, &repo.slug))
                .collect())
        }
        Host::GitHub => {
            blocking(move || {
                let out = gh(
                    &[
                        "api",
                        &format!("repos/{}/actions/runs?per_page=20", repo.slug),
                    ],
                    None,
                )?;
                let list: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
                Ok(list["workflow_runs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(github_run)
                    .collect())
            })
            .await
        }
    }
}

fn github_run(value: &Value) -> Run {
    let status = match (
        text(&value["status"]).as_str(),
        text(&value["conclusion"]).as_str(),
    ) {
        ("completed", "success") => "success",
        ("completed", "cancelled") => "cancelled",
        ("completed", "skipped" | "neutral" | "stale") => "skipped",
        ("completed", _) => "failure",
        ("in_progress", _) => "running",
        _ => "queued",
    };
    let started = Some(text(&value["run_started_at"]))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| text(&value["created_at"]));
    Run {
        host: "github".into(),
        id: value["id"].as_u64().unwrap_or_default().to_string(),
        number: value["run_number"].as_u64().unwrap_or_default(),
        name: text(&value["name"]),
        title: text(&value["display_title"]),
        event: text(&value["event"]),
        branch: text(&value["head_branch"]),
        status: status.into(),
        url: text(&value["html_url"]),
        started,
        finished: (value["status"] == "completed").then(|| text(&value["updated_at"])),
        author: text(&value["actor"]["login"]),
        author_avatar: text(&value["actor"]["avatar_url"]),
    }
}

fn bitbucket_run(value: &Value, slug: &str) -> Run {
    let state = &value["state"];
    let status = match (
        text(&state["name"]).as_str(),
        text(&state["result"]["name"]).as_str(),
    ) {
        ("COMPLETED", "SUCCESSFUL") => "success",
        ("COMPLETED", "STOPPED" | "EXPIRED") => "cancelled",
        ("COMPLETED", _) => "failure",
        ("IN_PROGRESS", _) if state["stage"]["name"] == "PAUSED" => "queued",
        ("IN_PROGRESS", _) => "running",
        _ => "queued",
    };
    let selector = &value["target"]["selector"];
    let kind = text(&selector["type"]);
    let pattern = text(&selector["pattern"]);
    let name = if pattern.is_empty() || kind == "default" {
        kind
    } else {
        format!("{kind}: {pattern}")
    };
    let number = value["build_number"].as_u64().unwrap_or_default();
    let title = text(&value["target"]["commit"]["message"])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    Run {
        host: "bitbucket".into(),
        id: text(&value["uuid"]),
        number,
        name,
        title,
        event: text(&value["trigger"]["name"]).to_lowercase(),
        branch: text(&value["target"]["ref_name"]),
        status: status.into(),
        url: format!("https://bitbucket.org/{slug}/pipelines/results/{number}"),
        started: text(&value["created_on"]),
        finished: value["completed_on"].as_str().map(str::to_string),
        author: text(&value["creator"]["display_name"]),
        author_avatar: text(&value["creator"]["links"]["avatar"]["href"]),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_pull_request_links_of_both_hosts() {
        let (repo, number) =
            pull_link("https://github.com/agence/orbit/pull/2108/files?w=1").unwrap();
        assert_eq!(
            (repo.host, repo.slug.as_str(), number),
            (Host::GitHub, "agence/orbit", 2108)
        );
        let (repo, number) =
            pull_link("https://bitbucket.org/steel-shed/bmc/pull-requests/412/overview").unwrap();
        assert_eq!(
            (repo.host, repo.slug.as_str(), number),
            (Host::Bitbucket, "steel-shed/bmc", 412)
        );
        assert!(pull_link("https://github.com/agence/orbit/issues/12").is_none());
        assert!(pull_link("https://bitbucket.org/steel-shed/bmc/pull/412").is_none());
        assert!(pull_link("https://gitlab.com/a/b/pull/1").is_none());
        let (repo, number) =
            pull_link("https://gitlab.com/grp/sub/proj/-/merge_requests/7767/diffs").unwrap();
        assert_eq!(
            (repo.host, repo.slug.as_str(), number),
            (Host::GitLab, "gitlab.com/grp/sub/proj", 7767)
        );
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
        let bitbucket = |slug: &str| {
            Some(Repo {
                host: Host::Bitbucket,
                slug: slug.into(),
            })
        };
        assert_eq!(
            repo("git@bitbucket.org:akki-team/fileas.git"),
            bitbucket("akki-team/fileas")
        );
        assert_eq!(
            repo("https://jgrasp@bitbucket.org/akki-team/fileas.git"),
            bitbucket("akki-team/fileas")
        );
        assert_eq!(
            repo("ssh://git@bitbucket.org:22/akki-team/fileas/"),
            bitbucket("akki-team/fileas")
        );
        assert_eq!(
            repo("git@ssh.bitbucket.org:akki-team/fileas.git"),
            bitbucket("akki-team/fileas")
        );
        assert_eq!(
            repo("git@github.com:Jgrasp/squadron.git"),
            Some(Repo {
                host: Host::GitHub,
                slug: "Jgrasp/squadron".into()
            })
        );
        let gitlab = |url: &str| parse(url, || Some("gitlab.corp.fr".into()));
        assert_eq!(
            gitlab("git@gitlab.com:grp/sub/proj.git").map(|r| (r.host, r.slug)),
            Some((Host::GitLab, "gitlab.com/grp/sub/proj".into()))
        );
        assert_eq!(
            gitlab("https://Gitlab.Corp.fr/team/app.git").map(|r| r.slug),
            Some("gitlab.corp.fr/team/app".into())
        );
        assert_eq!(gitlab("git@git.other.fr:team/app.git"), None);
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
            assert_eq!(gitlab(url), azure, "{url}");
        }
        assert_eq!(gitlab("git@github.com:a/b/c.git"), None);
        assert_eq!(repo("git@github.com:a/b?x=1"), None);
    }

    #[test]
    fn sums_up_a_bitbucket_pull_request() {
        let found = json!({
            "id": 2108, "title": "feat(prevision): [FIL-671] prévision", "state": "OPEN", "draft": false, "comment_count": 3,
            "links": { "html": { "href": "https://bitbucket.org/akki-team/fileas/pull-requests/2108" } },
            "source": { "branch": { "name": "dev/FIL-671" }, "commit": { "hash": "1094c23e6813" } },
            "destination": { "branch": { "name": "feature/FIL-671" } },
            "author": { "display_name": "Quentin Briais" }, "updated_on": "2026-10-23T09:27:57+00:00",
            "reviewers": [{ "uuid": "{a}", "display_name": "Ann", "links": { "avatar": { "href": "https://a/ann.png" } } }, { "uuid": "{me}", "display_name": "Me", "links": { "avatar": { "href": "https://a/me.png" } } }, { "uuid": "{c}", "display_name": "Carl", "links": { "avatar": { "href": "https://a/carl.png" } } }],
            "participants": [
                { "approved": true, "state": "approved", "role": "REVIEWER", "user": { "uuid": "{a}", "display_name": "Ann" } },
                { "approved": false, "state": "changes_requested", "role": "REVIEWER", "user": { "uuid": "{me}", "display_name": "Me" } },
                { "approved": false, "state": null, "role": "PARTICIPANT", "user": { "uuid": "{b}", "display_name": "Bob" } }
            ]
        });
        let statuses = json!({ "values": [{ "state": "SUCCESSFUL" }, { "state": "INPROGRESS" }] });
        assert_eq!(
            bitbucket_pull(&found, &statuses, "{me}"),
            Pull {
                host: "bitbucket".into(),
                number: 2108,
                title: "feat(prevision): [FIL-671] prévision".into(),
                description: String::new(),
                url: "https://bitbucket.org/akki-team/fileas/pull-requests/2108".into(),
                state: "open".into(),
                draft: false,
                approvals: 1,
                changes_requested: true,
                comments: 3,
                checks: Some("pending".into()),
                branch: "dev/FIL-671".into(),
                base: "feature/FIL-671".into(),
                head: "1094c23e6813".into(),
                author: "Quentin Briais".into(),
                author_avatar: String::new(),
                updated: "2026-10-23T09:27:57+00:00".into(),
                reviewed: Some("changes_requested".into()),
                reviewed_head: None,
                reviewed_at: None,
                head_at: None,
                reviewers: vec![
                    Reviewer {
                        name: "Ann".into(),
                        avatar: "https://a/ann.png".into(),
                        verdict: "approved".into()
                    },
                    Reviewer {
                        name: "Me".into(),
                        avatar: "https://a/me.png".into(),
                        verdict: "changes_requested".into()
                    },
                    Reviewer {
                        name: "Carl".into(),
                        avatar: "https://a/carl.png".into(),
                        verdict: "waiting".into()
                    },
                ],
                conflicts: false,
                mergeable: false,
            }
        );
        assert_eq!(bitbucket_pull(&found, &Value::Null, "{b}").checks, None);
        assert_eq!(bitbucket_pull(&found, &Value::Null, "{b}").reviewed, None);
        let mut talked = found.clone();
        talked["participants"][2]["participated_on"] = json!("2026-10-23T10:00:00Z");
        assert_eq!(
            bitbucket_pull(&talked, &Value::Null, "{b}")
                .reviewed
                .as_deref(),
            Some("commented")
        );
        assert_eq!(
            bitbucket_pull(&found, &Value::Null, "{a}")
                .reviewed
                .as_deref(),
            Some("approved")
        );
        assert_eq!(bitbucket_pull(&found, &Value::Null, "").reviewed, None);
        let mut approved = found.clone();
        approved["participants"][1]["state"] = Value::Null;
        assert!(bitbucket_pull(&approved, &Value::Null, "").mergeable);
        assert!(!bitbucket_pull(&approved, &statuses, "").mergeable);
    }

    #[test]
    fn sums_up_a_github_pull_request() {
        let found = json!({
            "number": 12, "title": "Export", "url": "https://github.com/a/b/pull/12", "state": "MERGED", "isDraft": true,
            "reviewDecision": "APPROVED", "latestReviews": [{ "state": "APPROVED", "author": { "login": "ann" } }, { "state": "COMMENTED", "author": { "login": "octo" } }], "comments": [{}, {}],
            "statusCheckRollup": [{ "__typename": "CheckRun", "status": "COMPLETED", "conclusion": "SUCCESS" }, { "__typename": "StatusContext", "state": "FAILURE" }],
            "baseRefName": "main", "headRefName": "feat/export", "headRefOid": "abc123", "author": { "login": "octo" }, "updatedAt": "2026-10-23T10:00:00Z"
        });
        let pull = github_pull(&found, "octo");
        assert_eq!(
            (
                pull.branch.as_str(),
                pull.author.as_str(),
                pull.reviewed.as_deref()
            ),
            ("feat/export", "octo", Some("commented"))
        );
        assert_eq!(
            github_pull(&found, "ann").reviewed.as_deref(),
            Some("approved")
        );
        let asked = json!({
            "latestReviews": [{ "state": "APPROVED", "author": { "login": "ann" } }, { "state": "CHANGES_REQUESTED", "author": { "login": "bob" } }],
            "reviewRequests": [{ "__typename": "User", "login": "bob", "name": "Bob B." }, { "__typename": "Team", "name": "core" }],
            "mergeable": "CONFLICTING"
        });
        let requested = github_pull(&asked, "");
        let seen: Vec<_> = requested
            .reviewers
            .iter()
            .map(|r| (r.name.as_str(), r.verdict.as_str(), r.avatar.is_empty()))
            .collect();
        assert_eq!(
            seen,
            [
                ("ann", "approved", false),
                ("Bob B.", "waiting", false),
                ("core", "waiting", true)
            ]
        );
        assert!(requested.conflicts);
        assert!(!pull.mergeable);
        let clean = json!({ "state": "OPEN", "isDraft": false, "mergeStateStatus": "CLEAN" });
        assert!(github_pull(&clean, "").mergeable);
        let blocked = json!({ "state": "OPEN", "isDraft": false, "mergeStateStatus": "BLOCKED" });
        assert!(!github_pull(&blocked, "").mergeable);
        assert_eq!(
            (
                pull.state.as_str(),
                pull.draft,
                pull.approvals,
                pull.changes_requested,
                pull.comments
            ),
            ("merged", true, 1, false, 2)
        );
        assert_eq!(
            (
                pull.checks.as_deref(),
                pull.base.as_str(),
                pull.head.as_str()
            ),
            (Some("failing"), "main", "abc123")
        );
        let running =
            json!({ "statusCheckRollup": [{ "status": "IN_PROGRESS", "conclusion": "" }] });
        assert_eq!(
            github_pull(&running, "x").checks.as_deref(),
            Some("pending")
        );
    }

    #[test]
    fn explains_why_github_refused() {
        let body = br#"{"message":"Unprocessable Entity","errors":["Pull request review thread line must be part of the diff"],"status":"422"}"#;
        assert_eq!(
            gh_error(b"gh: Unprocessable Entity (HTTP 422)\n", body),
            "gh: Unprocessable Entity (HTTP 422): Pull request review thread line must be part of the diff"
        );
        let body = br#"{"message":"Invalid request.","status":"422"}"#;
        assert_eq!(
            gh_error(b"gh: Invalid request. (HTTP 422)", body),
            "gh: Invalid request. (HTTP 422)"
        );
    }

    #[test]
    fn reads_who_you_are_on_both_hosts() {
        let bitbucket = json!({ "display_name": "Quentin Briais", "links": { "avatar": { "href": "https://a/q.png" } } });
        assert_eq!(
            person(
                &bitbucket,
                "display_name",
                &bitbucket["links"]["avatar"]["href"]
            ),
            Person {
                name: "Quentin Briais".into(),
                avatar: "https://a/q.png".into()
            }
        );
        let github = json!({ "login": "Jgrasp", "avatar_url": "https://a/j.png" });
        assert_eq!(
            person(&github, "login", &github["avatar_url"]),
            Person {
                name: "Jgrasp".into(),
                avatar: "https://a/j.png".into()
            }
        );
    }

    #[test]
    fn reads_remarks_on_lines_and_on_the_whole_pull_request() {
        let on_line = json!({ "id": 7, "content": { "raw": "Rename this" }, "user": { "display_name": "Amandine T.", "links": { "avatar": { "href": "https://a/at.png" } } }, "inline": { "path": "src/A.php", "from": null, "to": 12 }, "created_on": "2026-10-23T09:00:00Z" });
        let removed = json!({ "id": 8, "content": { "raw": "Why?" }, "user": { "display_name": "A" }, "inline": { "path": "src/A.php", "from": 4, "to": null }, "parent": { "id": 7 }, "created_on": "t" });
        let outdated = json!({ "id": 9, "content": { "raw": "Old" }, "user": { "display_name": "A" }, "inline": { "path": "src/A.php", "to": 3, "outdated": true }, "created_on": "t" });
        let general = json!({ "id": 10, "content": { "raw": "LGTM" }, "user": { "display_name": "A" }, "created_on": "t" });
        let gone = json!({ "id": 11, "deleted": true });
        let a = bitbucket_remark(&on_line).unwrap();
        assert_eq!(
            (
                a.path.as_deref(),
                a.line,
                a.removed,
                a.author.as_str(),
                a.avatar.as_str()
            ),
            (
                Some("src/A.php"),
                Some(12),
                false,
                "Amandine T.",
                "https://a/at.png"
            )
        );
        let b = bitbucket_remark(&removed).unwrap();
        assert_eq!((b.line, b.removed, b.reply_to), (Some(4), true, Some(7)));
        assert_eq!(bitbucket_remark(&outdated).unwrap().line, None);
        assert_eq!(bitbucket_remark(&general).unwrap().path, None);
        assert_eq!(bitbucket_remark(&gone), None);
        assert_eq!(
            (
                a.kind.as_str(),
                bitbucket_remark(&general).unwrap().kind.as_str()
            ),
            ("line", "general")
        );

        let github = json!({ "id": 1, "body": "Nit", "user": { "login": "octo", "avatar_url": "https://a/octo.png" }, "path": "a.ts", "line": 5, "side": "LEFT", "in_reply_to_id": null, "created_at": "t" });
        let c = github_remark(&github, "pulls");
        assert_eq!(
            (
                c.path.as_deref(),
                c.line,
                c.removed,
                c.author.as_str(),
                c.avatar.as_str(),
                c.reply_to
            ),
            (
                Some("a.ts"),
                Some(5),
                true,
                "octo",
                "https://a/octo.png",
                None
            )
        );
    }

    #[test]
    #[ignore = "asks GitHub through gh"]
    fn real_github_pull_request_and_remarks() {
        let out = gh(&["pr", "list", "--repo", "akki-team/lrpresse", "--head", "dev/theme-quentin", "--state", "all", "--limit", "1", "--json", "number,title,url,state,isDraft,reviewDecision,latestReviews,comments,statusCheckRollup,baseRefName,headRefOid"], None).unwrap();
        let list: Value = serde_json::from_str(&out).unwrap();
        let pull = github_pull(&list[0], "");
        assert_eq!(
            (pull.host.as_str(), pull.number, pull.state.as_str()),
            ("github", 71, "merged")
        );
        assert_eq!(pull.head.len(), 40);
        assert!(
            lines(
                &gh(
                    &[
                        "api",
                        "repos/akki-team/lrpresse/issues/71/comments",
                        "--paginate",
                        "--jq",
                        ".[]"
                    ],
                    None
                )
                .unwrap()
            )
            .iter()
            .all(|c| c["body"].is_string())
        );
    }

    #[test]
    #[ignore = "fetches the Fileas repository"]
    fn real_bitbucket_pull_request_diff_matches_bitbucket() {
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
    #[ignore = "asks Bitbucket and GitHub with the real accounts"]
    fn real_linked_pull_requests_of_both_hosts() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        for dir in [
            "/home/jgrasp/Projects/Sylius/kiosquemag",
            "/home/jgrasp/Projects/Shopify/lr-presse-shopify",
        ] {
            let dir = Path::new(dir);
            let lists = rt.block_on(lists(dir)).unwrap();
            let url = lists
                .review
                .into_iter()
                .chain(lists.mine)
                .next()
                .map(|p| p.url);
            let url = url.unwrap_or_else(|| {
                let repo = repo_of(dir).unwrap();
                match repo.host {
                    Host::GitHub => format!("https://github.com/{}/pull/1", repo.slug),
                    _ => format!("https://bitbucket.org/{}/pull-requests/1", repo.slug),
                }
            });
            let found = rt.block_on(linked(&url, vec![dir.to_path_buf()])).unwrap();
            println!("{url}\n{found:#?}");
            assert!(found.pull.number > 0 && !found.pull.title.is_empty());
            assert_eq!(found.project.as_deref(), Some(dir));
        }
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
        assert_eq!(read_secret("cached-check").unwrap().as_deref(), Some("kept"));
        assert_eq!(read_secret("uncached-check").unwrap_err(), KEYRING_BUSY);
    }

    #[tokio::test]
    #[ignore = "calls Bitbucket"]
    async fn real_bitbucket_rejects_a_wrong_token() {
        let refused = sign_in("nobody@example.com".into(), "wrong".into()).await;
        assert_eq!(
            refused,
            Err("Bitbucket refused this email and token.".into())
        );
    }

    #[tokio::test]
    #[ignore = "reads the pull requests of Fileas and LR Presse"]
    async fn real_lists_of_pull_requests() {
        for dir in [
            "/home/jgrasp/Projects/Symfony/fileas",
            "/home/jgrasp/Projects/Shopify/lr-presse-shopify",
        ] {
            match lists(Path::new(dir)).await {
                Ok(found) => println!(
                    "{dir}: {} to review {:?}, {} mine {:?}",
                    found.review.len(),
                    found
                        .review
                        .iter()
                        .map(|p| (p.number, p.reviewed.clone()))
                        .collect::<Vec<_>>(),
                    found.mine.len(),
                    found
                        .mine
                        .iter()
                        .map(|p| (p.number, p.checks.clone(), p.approvals))
                        .collect::<Vec<_>>()
                ),
                Err(e) => println!("{dir}: error {e}"),
            }
        }
    }
    #[test]
    fn sums_up_runs_of_both_hosts() {
        let run = github_run(
            &json!({"id": 7, "run_number": 14, "name": "Release", "display_title": "Squadron 0.10.0", "event": "push", "head_branch": "main", "status": "completed", "conclusion": "timed_out", "html_url": "u", "created_at": "a", "run_started_at": "b", "updated_at": "c", "actor": {"login": "Jgrasp", "avatar_url": "https://avatars.githubusercontent.com/u/1"}}),
        );
        assert_eq!(
            (
                run.id.as_str(),
                run.status.as_str(),
                run.started.as_str(),
                run.finished.as_deref()
            ),
            ("7", "failure", "b", Some("c"))
        );
        assert_eq!(
            (run.author.as_str(), run.author_avatar.as_str()),
            ("Jgrasp", "https://avatars.githubusercontent.com/u/1")
        );
        let run =
            github_run(&json!({"status": "waiting", "created_at": "a", "run_started_at": null}));
        assert_eq!(
            (run.status.as_str(), run.started.as_str(), run.finished),
            ("queued", "a", None)
        );
        let run = bitbucket_run(
            &json!({"uuid": "{x}", "build_number": 42, "state": {"name": "COMPLETED", "result": {"name": "STOPPED"}}, "target": {"ref_name": "main", "selector": {"type": "custom", "pattern": "deploy-prod"}, "commit": {"message": "Ship it\n\nmore"}}, "trigger": {"name": "MANUAL"}, "creator": {"display_name": "Quentin Briais", "links": {"avatar": {"href": "https://avatar/q"}}}, "created_on": "a", "completed_on": "b"}),
            "akki-team/fileas",
        );
        assert_eq!(
            (
                run.name.as_str(),
                run.title.as_str(),
                run.status.as_str(),
                run.event.as_str(),
                run.url.as_str()
            ),
            (
                "custom: deploy-prod",
                "Ship it",
                "cancelled",
                "manual",
                "https://bitbucket.org/akki-team/fileas/pipelines/results/42"
            )
        );
        assert_eq!(
            (run.author.as_str(), run.author_avatar.as_str()),
            ("Quentin Briais", "https://avatar/q")
        );
        let run = bitbucket_run(
            &json!({"state": {"name": "IN_PROGRESS", "stage": {"name": "RUNNING"}}, "target": {"selector": {"type": "default"}}}),
            "a/b",
        );
        assert_eq!(
            (run.name.as_str(), run.status.as_str()),
            ("default", "running")
        );
    }

    #[test]
    fn reads_a_repository_given_by_hand() {
        let dir = Path::new("/nowhere");
        assert_eq!(
            repo_given(dir, "github.com/Jgrasp/squadron").unwrap().slug,
            "Jgrasp/squadron"
        );
        assert_eq!(
            repo_given(dir, "git@bitbucket.org:akki-team/fileas.git")
                .unwrap()
                .host,
            Host::Bitbucket
        );
        assert!(repo_given(dir, "Jgrasp/squadron").is_err());
        assert_eq!(
            repo_given(dir, "gitlab.com/grp/proj").unwrap().host,
            Host::GitLab
        );
    }

    #[test]
    #[ignore = "asks GitHub through gh"]
    fn real_github_runs() {
        let runs = tauri::async_runtime::block_on(runs(
            Path::new("/nowhere"),
            "github.com/Jgrasp/squadron",
        ))
        .unwrap();
        assert!(runs.iter().any(|r| {
            r.name == "Release"
                && !r.author_avatar.is_empty()
                && r.url
                    .starts_with("https://github.com/Jgrasp/squadron/actions/runs/")
        }));
    }

    #[test]
    #[ignore = "asks Bitbucket with the saved token"]
    fn real_bitbucket_runs() {
        let runs = tauri::async_runtime::block_on(runs(
            Path::new("/home/jgrasp/Projects/Symfony/fileas"),
            "",
        ))
        .unwrap();
        println!("{runs:#?}");
        assert!(!runs.is_empty());
    }
}
