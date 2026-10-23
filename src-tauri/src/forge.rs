use crate::{git, projects};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const BITBUCKET: &str = "https://api.bitbucket.org/2.0";
const NO_TOKEN: &str = "Add a Bitbucket API token in Settings › Code hosting.";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Host {
    Bitbucket,
    GitHub,
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
    let host = match host.rsplit('@').next()?.split(':').next()? {
        "bitbucket.org" => Host::Bitbucket,
        "github.com" => Host::GitHub,
        _ => return None,
    };
    let mut parts = path.trim_start_matches('/').split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (parts.next().is_none() && valid(owner) && valid(name)).then(|| Repo {
        host,
        slug: format!("{owner}/{name}"),
    })
}

fn repo_of(dir: &Path) -> Result<Repo, String> {
    git::origin(dir)
        .and_then(|url| repo(&url))
        .ok_or_else(|| "The project is not on Bitbucket or GitHub.".into())
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Pull {
    pub host: String,
    pub number: u64,
    pub title: String,
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
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Remark {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub removed: bool,
    pub reply_to: Option<u64>,
    pub at: String,
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
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Accounts {
    pub bitbucket: Option<String>,
    pub github: Option<String>,
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

fn checks<'a>(states: impl Iterator<Item = &'a str>) -> Option<String> {
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
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("squadron", "bitbucket").map_err(|e| e.to_string())
}

fn account() -> Result<Account, String> {
    let secret = entry()?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => NO_TOKEN.to_string(),
        e => e.to_string(),
    })?;
    serde_json::from_str(&secret).map_err(|e| e.to_string())
}

pub async fn sign_in(email: String, token: String) -> Result<(), String> {
    let response = reqwest::Client::new()
        .get(format!("{BITBUCKET}/user"))
        .basic_auth(&email, Some(&token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Bitbucket refused this email and token.".into());
    }
    let secret = serde_json::to_string(&Account { email, token }).map_err(|e| e.to_string())?;
    entry()?.set_password(&secret).map_err(|e| e.to_string())
}

pub fn sign_out() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn accounts() -> Accounts {
    Accounts {
        bitbucket: account().ok().map(|a| a.email),
        github: gh(&["api", "user", "--jq", ".login"], None)
            .ok()
            .map(|login| login.trim().to_string())
            .filter(|login| !login.is_empty()),
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
    let mut request = reqwest::Client::new()
        .request(method, url)
        .basic_auth(account.email, Some(account.token));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(
            "Bitbucket refused the API token: add a new one in Settings › Code hosting.".into(),
        );
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    if !status.is_success() {
        let reason = value["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| body.chars().take(200).collect());
        return Err(format!("Bitbucket answered {status}: {reason}"));
    }
    Ok(value)
}

fn gh(args: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut child = Command::new("gh")
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
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
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

pub async fn pull(dir: &Path) -> Result<Option<Pull>, String> {
    let repo = repo_of(dir)?;
    let Some(branch) = projects::branch(dir) else {
        return Ok(None);
    };
    match repo.host {
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
                    ("fields", "values.id,values.title,values.state,values.draft,values.comment_count,values.links.html.href,values.source.commit.hash,values.destination.branch.name,values.participants.approved,values.participants.state"),
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
            Ok(Some(bitbucket_pull(found, &statuses, &branch)))
        }
        Host::GitHub => blocking(move || {
            let out = gh(
                &[
                    "pr", "list", "--repo", &repo.slug, "--head", &branch, "--state", "all",
                    "--limit", "1", "--json",
                    "number,title,url,state,isDraft,reviewDecision,latestReviews,comments,statusCheckRollup,baseRefName,headRefOid",
                ],
                None,
            )?;
            let list: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
            Ok(list.get(0).map(|found| github_pull(found, &branch)))
        })
        .await,
    }
}

fn bitbucket_pull(found: &Value, statuses: &Value, branch: &str) -> Pull {
    let people = found["participants"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let states = statuses["values"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| match s["state"].as_str() {
            Some("SUCCESSFUL") => "pass",
            Some("INPROGRESS") => "pending",
            _ => "fail",
        });
    Pull {
        host: "bitbucket".into(),
        number: found["id"].as_u64().unwrap_or_default(),
        title: text(&found["title"]),
        url: text(&found["links"]["html"]["href"]),
        state: text(&found["state"]).to_lowercase(),
        draft: found["draft"].as_bool().unwrap_or(false),
        approvals: people.iter().filter(|p| p["approved"] == true).count() as u32,
        changes_requested: people.iter().any(|p| p["state"] == "changes_requested"),
        comments: found["comment_count"].as_u64().unwrap_or_default() as u32,
        checks: checks(states),
        branch: branch.into(),
        base: text(&found["destination"]["branch"]["name"]),
        head: text(&found["source"]["commit"]["hash"]),
    }
}

fn github_pull(found: &Value, branch: &str) -> Pull {
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
        url: text(&found["url"]),
        state: text(&found["state"]).to_lowercase(),
        draft: found["isDraft"].as_bool().unwrap_or(false),
        approvals: reviews.iter().filter(|r| r["state"] == "APPROVED").count() as u32,
        changes_requested: found["reviewDecision"] == "CHANGES_REQUESTED",
        comments: found["comments"].as_array().map_or(0, Vec::len) as u32,
        checks: checks(states),
        branch: branch.into(),
        base: text(&found["baseRefName"]),
        head: text(&found["headRefOid"]),
    }
}

pub async fn remarks(dir: &Path, number: u64) -> Result<Vec<Remark>, String> {
    let repo = repo_of(dir)?;
    match repo.host {
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
                        &[("pagelen", "100"), ("fields", "next,values.id,values.content.raw,values.user.display_name,values.inline,values.parent.id,values.created_on,values.deleted")]
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
                            .map(github_remark),
                    );
                }
                found.sort_by(|a, b| a.at.cmp(&b.at));
                Ok(found)
            })
            .await
        }
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
        body: text(&found["content"]["raw"]),
        path: inline["path"].as_str().map(str::to_string),
        line: to.or(from).filter(|_| current).map(|n| n as u32),
        removed: to.is_none() && from.is_some(),
        reply_to: found["parent"]["id"].as_u64(),
        at: text(&found["created_on"]),
    })
}

fn github_remark(found: &Value) -> Remark {
    Remark {
        id: found["id"].as_u64().unwrap_or_default(),
        author: text(&found["user"]["login"]),
        body: text(&found["body"]),
        path: found["path"].as_str().map(str::to_string),
        line: found["line"].as_u64().map(|n| n as u32),
        removed: found["side"] == "LEFT",
        reply_to: found["in_reply_to_id"].as_u64(),
        at: text(&found["created_at"]),
    }
}

pub async fn post(
    dir: &Path,
    number: u64,
    head: String,
    drafts: Vec<Draft>,
) -> Result<Posted, String> {
    let repo = repo_of(dir)?;
    match repo.host {
        Host::Bitbucket => {
            let total = drafts.len();
            for (done, draft) in drafts.into_iter().enumerate() {
                let mut inline = json!({ "path": draft.path });
                inline[if draft.removed { "from" } else { "to" }] = json!(draft.line);
                let sent = bitbucket(
                    Method::POST,
                    &format!("/repositories/{}/pullrequests/{number}/comments", repo.slug),
                    &[],
                    Some(json!({ "content": { "raw": draft.body }, "inline": inline })),
                )
                .await;
                if let Err(error) = sent {
                    return Ok(Posted {
                        done,
                        error: Some(error),
                    });
                }
            }
            Ok(Posted {
                done: total,
                error: None,
            })
        }
        Host::GitHub => blocking(move || {
            let comments: Vec<Value> = drafts
                .iter()
                .map(|d| json!({ "path": d.path, "line": d.line, "side": if d.removed { "LEFT" } else { "RIGHT" }, "body": d.body }))
                .collect();
            let review = json!({ "commit_id": head, "event": "COMMENT", "comments": comments });
            let sent = gh(
                &["api", "-X", "POST", &format!("repos/{}/pulls/{number}/reviews", repo.slug), "--input", "-"],
                Some(&review.to_string()),
            );
            Ok(Posted {
                done: if sent.is_ok() { drafts.len() } else { 0 },
                error: sent.err(),
            })
        })
        .await,
    }
}

#[cfg(test)]
mod tests {
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
            repo("git@github.com:Jgrasp/squadron.git"),
            Some(Repo {
                host: Host::GitHub,
                slug: "Jgrasp/squadron".into()
            })
        );
        assert_eq!(repo("git@gitlab.com:a/b.git"), None);
        assert_eq!(repo("git@github.com:a/b/c.git"), None);
        assert_eq!(repo("git@github.com:a/b?x=1"), None);
    }

    #[test]
    fn sums_up_a_bitbucket_pull_request() {
        let found = json!({
            "id": 2108, "title": "feat(prevision): [FIL-671] prévision", "state": "OPEN", "draft": false, "comment_count": 3,
            "links": { "html": { "href": "https://bitbucket.org/akki-team/fileas/pull-requests/2108" } },
            "source": { "commit": { "hash": "1094c23e6813" } },
            "destination": { "branch": { "name": "feature/FIL-671" } },
            "participants": [{ "approved": true, "state": "approved" }, { "approved": false, "state": "changes_requested" }, { "approved": false, "state": null }]
        });
        let statuses = json!({ "values": [{ "state": "SUCCESSFUL" }, { "state": "INPROGRESS" }] });
        assert_eq!(
            bitbucket_pull(&found, &statuses, "dev/FIL-671"),
            Pull {
                host: "bitbucket".into(),
                number: 2108,
                title: "feat(prevision): [FIL-671] prévision".into(),
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
            }
        );
        assert_eq!(bitbucket_pull(&found, &Value::Null, "b").checks, None);
    }

    #[test]
    fn sums_up_a_github_pull_request() {
        let found = json!({
            "number": 12, "title": "Export", "url": "https://github.com/a/b/pull/12", "state": "MERGED", "isDraft": true,
            "reviewDecision": "APPROVED", "latestReviews": [{ "state": "APPROVED" }, { "state": "COMMENTED" }], "comments": [{}, {}],
            "statusCheckRollup": [{ "__typename": "CheckRun", "status": "COMPLETED", "conclusion": "SUCCESS" }, { "__typename": "StatusContext", "state": "FAILURE" }],
            "baseRefName": "main", "headRefOid": "abc123"
        });
        let pull = github_pull(&found, "feat/export");
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
    fn reads_remarks_on_lines_and_on_the_whole_pull_request() {
        let on_line = json!({ "id": 7, "content": { "raw": "Rename this" }, "user": { "display_name": "Amandine T." }, "inline": { "path": "src/A.php", "from": null, "to": 12 }, "created_on": "2026-10-23T09:00:00Z" });
        let removed = json!({ "id": 8, "content": { "raw": "Why?" }, "user": { "display_name": "A" }, "inline": { "path": "src/A.php", "from": 4, "to": null }, "parent": { "id": 7 }, "created_on": "t" });
        let outdated = json!({ "id": 9, "content": { "raw": "Old" }, "user": { "display_name": "A" }, "inline": { "path": "src/A.php", "to": 3, "outdated": true }, "created_on": "t" });
        let general = json!({ "id": 10, "content": { "raw": "LGTM" }, "user": { "display_name": "A" }, "created_on": "t" });
        let gone = json!({ "id": 11, "deleted": true });
        let a = bitbucket_remark(&on_line).unwrap();
        assert_eq!(
            (a.path.as_deref(), a.line, a.removed, a.author.as_str()),
            (Some("src/A.php"), Some(12), false, "Amandine T.")
        );
        let b = bitbucket_remark(&removed).unwrap();
        assert_eq!((b.line, b.removed, b.reply_to), (Some(4), true, Some(7)));
        assert_eq!(bitbucket_remark(&outdated).unwrap().line, None);
        assert_eq!(bitbucket_remark(&general).unwrap().path, None);
        assert_eq!(bitbucket_remark(&gone), None);

        let github = json!({ "id": 1, "body": "Nit", "user": { "login": "octo" }, "path": "a.ts", "line": 5, "side": "LEFT", "in_reply_to_id": null, "created_at": "t" });
        let c = github_remark(&github);
        assert_eq!(
            (
                c.path.as_deref(),
                c.line,
                c.removed,
                c.author.as_str(),
                c.reply_to
            ),
            (Some("a.ts"), Some(5), true, "octo", None)
        );
    }

    #[test]
    #[ignore = "asks GitHub through gh"]
    fn real_github_pull_request_and_remarks() {
        let out = gh(&["pr", "list", "--repo", "akki-team/lrpresse", "--head", "dev/theme-quentin", "--state", "all", "--limit", "1", "--json", "number,title,url,state,isDraft,reviewDecision,latestReviews,comments,statusCheckRollup,baseRefName,headRefOid"], None).unwrap();
        let list: Value = serde_json::from_str(&out).unwrap();
        let pull = github_pull(&list[0], "dev/theme-quentin");
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
            git::pull_changes(dir, "feature/FIL-671", "dev/FIL-671", "1094c23e6813").unwrap();
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
}
