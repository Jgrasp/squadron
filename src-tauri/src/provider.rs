use crate::forge::{Draft, Linked, Lists, Person, Posted, Pull, Remark, Run};
use crate::{git, projects, wasm};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct ForgeSpec {
    pub remotes: Vec<String>,
    pub links: Vec<String>,
    pub calls: Vec<String>,
    pub verdicts: Vec<String>,
}

pub struct Provider {
    pub extension: String,
    pub spec: ForgeSpec,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub extension: String,
    pub calls: Vec<String>,
    pub host: String,
    pub repo: String,
    pub groups: BTreeMap<String, String>,
}

pub fn normalize(remote: &str) -> Option<String> {
    let url = remote.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let (host, path) = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?,
        None => match url.split_once(':') {
            Some((host, path)) if !host.contains('/') => (host, path),
            _ => url.split_once('/')?,
        },
    };
    let host = host.rsplit('@').next()?.split(':').next()?.to_lowercase();
    let path = path.trim_matches('/');
    (!host.is_empty() && !path.is_empty()).then(|| format!("{host}/{path}"))
}

static COMPILED: Mutex<Option<HashMap<String, Option<Regex>>>> = Mutex::new(None);

fn compiled(pattern: &str) -> Option<Regex> {
    let mut cache = COMPILED.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get_or_insert_with(HashMap::new)
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).ok())
        .clone()
}

fn expanded(pattern: &str, hosts: &[String]) -> Option<String> {
    if !pattern.contains("{account}") {
        return Some(pattern.to_string());
    }
    if hosts.is_empty() {
        return None;
    }
    let hosts: Vec<String> = hosts.iter().map(|h| regex::escape(h)).collect();
    Some(pattern.replace("{account}", &format!("(?:{})", hosts.join("|"))))
}

fn found(provider: &Provider, re: &Regex, text: &str) -> Option<(Target, Option<u64>)> {
    let caps = re.captures(text)?;
    let mut groups = BTreeMap::new();
    let mut parts = Vec::new();
    let mut number = None;
    for name in re.capture_names().flatten() {
        let Some(value) = caps.name(name) else {
            continue;
        };
        if name == "number" {
            number = value.as_str().parse().ok();
        } else {
            parts.push(value.as_str().to_string());
            groups.insert(name.to_string(), value.as_str().to_string());
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some((
        Target {
            extension: provider.extension.clone(),
            calls: provider.spec.calls.clone(),
            host: String::new(),
            repo: parts.join("/"),
            groups,
        },
        number,
    ))
}

pub fn remote_target(providers: &[Provider], remote: &str) -> Option<Target> {
    let normal = normalize(remote)?;
    let host = normal.split('/').next().unwrap_or_default().to_string();
    providers.iter().find_map(|provider| {
        provider.spec.remotes.iter().find_map(|pattern| {
            let pattern = expanded(pattern, &provider.hosts)?;
            let re = compiled(&format!("^(?:{pattern})$"))?;
            let (target, _) = found(provider, &re, &normal)?;
            Some(Target {
                host: host.clone(),
                ..target
            })
        })
    })
}

pub fn link_target(providers: &[Provider], url: &str) -> Option<(Target, u64)> {
    let url = url.trim();
    let host = reqwest::Url::parse(url)
        .ok()?
        .host_str()?
        .trim_start_matches("www.")
        .to_lowercase();
    providers.iter().find_map(|provider| {
        provider.spec.links.iter().find_map(|pattern| {
            let pattern = expanded(pattern, &provider.hosts)?;
            let re = compiled(&format!("^(?:{pattern})(?:[/?#].*)?$"))?;
            let (target, number) = found(provider, &re, url)?;
            Some((
                Target {
                    host: host.clone(),
                    ..target
                },
                number?,
            ))
        })
    })
}

pub fn failure(raw: &str) -> String {
    let text = |value: &Value| -> Option<String> {
        match value {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Object(o) => o.get("en")?.as_str().map(str::to_string),
            _ => None,
        }
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(o)) => {
            let title = o.get("title").and_then(text);
            let detail = o.get("detail").and_then(text);
            match (title, detail) {
                (Some(t), Some(d)) => format!("{t}: {d}"),
                (Some(t), None) | (None, Some(t)) => t,
                (None, None) => raw.to_string(),
            }
        }
        _ => raw.to_string(),
    }
}

fn input(target: &Target, args: Value) -> String {
    json!({ "host": target.host, "repo": target.repo, "groups": target.groups, "args": args })
        .to_string()
}

pub fn ask<T: DeserializeOwned>(
    runtime: &wasm::Runtime,
    target: &Target,
    call: &str,
    args: Value,
) -> Result<T, String> {
    if !target.calls.iter().any(|c| c == call) {
        return Err(format!("{} does not answer {call}.", target.extension));
    }
    let answer = runtime
        .forge(&target.extension, call, &input(target, args))?
        .map_err(|raw| failure(&raw))?;
    serde_json::from_str(&answer).map_err(|e| {
        format!(
            "{} answered {call} with JSON Squadron cannot read: {e}",
            target.extension
        )
    })
}

async fn call<T: DeserializeOwned + Send + 'static>(
    app: &AppHandle,
    target: Target,
    call: &'static str,
    args: Value,
) -> Result<T, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ask(&app.state::<wasm::Runtime>(), &target, call, args)
    })
    .await
    .map_err(|e| e.to_string())?
}

const NOT_HOSTED: &str = "No forge extension knows this project.";

fn providers(app: &AppHandle) -> Vec<Provider> {
    app.state::<wasm::Runtime>().providers()
}

fn of_dir(app: &AppHandle, dir: &Path) -> Option<Target> {
    remote_target(&providers(app), &git::origin(dir)?)
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Coverage {
    pub path: PathBuf,
    pub remote: Option<String>,
    pub by: Option<String>,
}

pub fn covered(providers: &[Provider], path: PathBuf, origin: Option<String>) -> Coverage {
    let by = origin
        .as_deref()
        .and_then(|remote| remote_target(providers, remote).map(|t| t.extension));
    Coverage {
        path,
        remote: origin.as_deref().and_then(normalize),
        by,
    }
}

pub fn coverage(app: &AppHandle, projects: Vec<PathBuf>) -> Vec<Coverage> {
    let list = providers(app);
    projects
        .into_iter()
        .map(|path| {
            let origin = git::origin(&path);
            covered(&list, path, origin)
        })
        .collect()
}

fn owned(target: &Target, mut pull: Pull) -> Pull {
    pull.host = target.extension.clone();
    pull.mergeable &= target.calls.iter().any(|c| c == "merge");
    pull
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Numbered {
    pull: Pull,
    additions: Option<u64>,
    deletions: Option<u64>,
    files: Option<u64>,
}

pub async fn pull(
    app: &AppHandle,
    dir: &Path,
    branch: Option<String>,
) -> Result<Option<Pull>, String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let Some(branch) = branch.or_else(|| projects::branch(dir)) else {
        return Ok(None);
    };
    if !target.calls.iter().any(|c| c == "pull") {
        return Ok(None);
    }
    let found: Option<Pull> =
        call(app, target.clone(), "pull", json!({ "branch": branch })).await?;
    Ok(found.map(|p| owned(&target, p)))
}

async fn numbered_of(app: &AppHandle, target: &Target, number: u64) -> Result<Numbered, String> {
    let mut found: Numbered =
        call(app, target.clone(), "numbered", json!({ "number": number })).await?;
    found.pull = owned(target, found.pull);
    Ok(found)
}

pub async fn numbered(app: &AppHandle, dir: &Path, number: u64) -> Result<Pull, String> {
    match of_dir(app, dir) {
        Some(target) => Ok(numbered_of(app, &target, number).await?.pull),
        None => Err(NOT_HOSTED.into()),
    }
}

pub async fn linked(app: &AppHandle, url: &str, projects: Vec<PathBuf>) -> Result<Linked, String> {
    let list = providers(app);
    let Some((target, number)) = link_target(&list, url) else {
        return Err("Not a pull request link.".into());
    };
    let project = projects.into_iter().find(|dir| {
        git::origin(dir)
            .and_then(|remote| remote_target(&list, &remote))
            .is_some_and(|t| {
                t.extension == target.extension && t.repo.eq_ignore_ascii_case(&target.repo)
            })
    });
    let found = numbered_of(app, &target, number).await?;
    Ok(Linked {
        repo: target.repo,
        pull: found.pull,
        additions: found.additions,
        deletions: found.deletions,
        files: found.files,
        project,
    })
}

pub async fn lists(app: &AppHandle, dir: &Path) -> Result<Lists, String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let found: Lists = call(app, target.clone(), "lists", json!({})).await?;
    let own = |list: Vec<Pull>| list.into_iter().map(|p| owned(&target, p)).collect();
    Ok(Lists {
        review: own(found.review),
        mine: own(found.mine),
        others: own(found.others),
    })
}

pub async fn remarks(app: &AppHandle, dir: &Path, number: u64) -> Result<Vec<Remark>, String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let mut found: Vec<Remark> = call(app, target, "remarks", json!({ "number": number })).await?;
    found.sort_by(|a, b| a.at.cmp(&b.at));
    Ok(found)
}

pub async fn me(app: &AppHandle, dir: &Path) -> Result<Person, String> {
    match of_dir(app, dir) {
        Some(target) => call(app, target, "me", json!({})).await,
        None => Err(NOT_HOSTED.into()),
    }
}

pub async fn reply(
    app: &AppHandle,
    dir: &Path,
    number: u64,
    to: String,
    kind: String,
    body: String,
) -> Result<(), String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let body = body.trim();
    if body.is_empty() {
        return Err("The reply is empty.".into());
    }
    let args = json!({ "number": number, "to": to, "kind": kind, "body": body });
    call::<Value>(app, target, "reply", args).await.map(|_| ())
}

pub async fn resolve(
    app: &AppHandle,
    dir: &Path,
    number: u64,
    to: String,
    kind: String,
    resolved: bool,
) -> Result<(), String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let args = json!({ "number": number, "to": to, "kind": kind, "resolved": resolved });
    call::<Value>(app, target, "resolve", args)
        .await
        .map(|_| ())
}

pub async fn post(
    app: &AppHandle,
    dir: &Path,
    number: u64,
    head: String,
    drafts: Vec<Draft>,
    summary: String,
    verdict: String,
) -> Result<Posted, String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let verdict = if verdict.is_empty() {
        "comment"
    } else {
        &verdict
    };
    let args = json!({ "number": number, "head": head, "drafts": drafts, "summary": summary.trim(), "verdict": verdict });
    call(app, target, "post", args).await
}

pub async fn merge(app: &AppHandle, dir: &Path, number: u64) -> Result<(), String> {
    match of_dir(app, dir) {
        Some(target) => call::<Value>(app, target, "merge", json!({ "number": number }))
            .await
            .map(|_| ()),
        None => Err(NOT_HOSTED.into()),
    }
}

pub async fn update(
    app: &AppHandle,
    dir: &Path,
    number: u64,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    let Some(target) = of_dir(app, dir) else {
        return Err(NOT_HOSTED.into());
    };
    let title = title.map(|t| t.trim().to_string());
    if title.as_deref() == Some("") {
        return Err("The title is empty.".into());
    }
    if title.is_none() && description.is_none() {
        return Err("Give a new title, a new description, or both.".into());
    }
    let mut args = Map::new();
    args.insert("number".into(), json!(number));
    if let Some(title) = title {
        args.insert("title".into(), json!(title));
    }
    if let Some(description) = description {
        args.insert("description".into(), json!(description));
    }
    call::<Value>(app, target, "update", Value::Object(args))
        .await
        .map(|_| ())
}

pub async fn runs(app: &AppHandle, dir: &Path, repository: &str) -> Result<Vec<Run>, String> {
    let given = repository.trim();
    let remote = if given.is_empty() {
        git::origin(dir)
    } else {
        Some(given.to_string())
    };
    let Some(target) = remote.and_then(|r| remote_target(&providers(app), &r)) else {
        return Err(NOT_HOSTED.into());
    };
    if !target.calls.iter().any(|c| c == "runs") {
        return Ok(Vec::new());
    }
    let found: Vec<Run> = call(app, target.clone(), "runs", json!({})).await?;
    Ok(found
        .into_iter()
        .map(|run| Run {
            host: target.extension.clone(),
            ..run
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github() -> Provider {
        Provider {
            extension: "github".into(),
            spec: ForgeSpec {
                remotes: vec![
                    r"^github\.com/(?<owner>[A-Za-z0-9_.-]+)/(?<name>[A-Za-z0-9_.-]+)$".into(),
                ],
                links: vec![r"https?://(?:www\.)?github\.com/(?<owner>[A-Za-z0-9_.-]+)/(?<name>[A-Za-z0-9_.-]+)/pull/(?<number>[0-9]+)".into()],
                ..ForgeSpec::default()
            },
            hosts: vec![],
        }
    }

    fn gitlab(hosts: &[&str]) -> Provider {
        Provider {
            extension: "gitlab".into(),
            spec: ForgeSpec {
                remotes: vec![
                    r"(?<site>gitlab\.com)/(?<path>[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+)".into(),
                    r"(?<site>{account})/(?<path>[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+)".into(),
                ],
                links: vec![r"https://(?<site>gitlab\.com|{account})/(?<path>[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+?)/-/merge_requests/(?<number>[0-9]+)".into()],
                ..ForgeSpec::default()
            },
            hosts: hosts.iter().map(|h| h.to_string()).collect(),
        }
    }

    #[test]
    fn normalizes_remotes_of_every_form() {
        for remote in [
            "git@github.com:Acme/Shop.git",
            "https://github.com/Acme/Shop/",
            "ssh://git@github.com:22/Acme/Shop.git",
            "https://user:ghp_secret@github.com/Acme/Shop.git",
            "github.com/Acme/Shop",
        ] {
            assert_eq!(
                normalize(remote).as_deref(),
                Some("github.com/Acme/Shop"),
                "{remote}"
            );
        }
        assert_eq!(normalize("not a remote"), None);
    }

    #[test]
    fn finds_the_provider_of_a_remote_and_its_repo() {
        let list = [github(), gitlab(&["git.acme.io"])];
        let found = remote_target(&list, "git@github.com:acme/shop.git").unwrap();
        assert_eq!(
            (
                found.extension.as_str(),
                found.host.as_str(),
                found.repo.as_str()
            ),
            ("github", "github.com", "acme/shop")
        );
        assert_eq!(found.groups["owner"], "acme");
        let found = remote_target(&list, "https://git.acme.io/team/web/shop.git").unwrap();
        assert_eq!(
            (found.extension.as_str(), found.repo.as_str()),
            ("gitlab", "git.acme.io/team/web/shop")
        );
        assert!(remote_target(&[github(), gitlab(&[])], "https://git.acme.io/team/shop").is_none());
        assert!(remote_target(&list, "git@bitbucket.org:acme/shop.git").is_none());
        assert!(remote_target(&list, "https://github.com/acme/shop/extra").is_none());
    }

    #[test]
    fn reads_pull_request_links_with_their_tail() {
        let list = [github(), gitlab(&["git.acme.io"])];
        let (found, number) =
            link_target(&list, "https://github.com/acme/shop/pull/2108/files?w=1").unwrap();
        assert_eq!((found.repo.as_str(), number), ("acme/shop", 2108));
        let (found, number) = link_target(
            &list,
            "https://git.acme.io/team/web/shop/-/merge_requests/7",
        )
        .unwrap();
        assert_eq!(
            (found.extension.as_str(), found.repo.as_str(), number),
            ("gitlab", "git.acme.io/team/web/shop", 7)
        );
        assert!(link_target(&list, "https://github.com/acme/shop/pulls").is_none());
        assert!(link_target(&list, "https://github.com/acme/shop/pull/12x").is_none());
    }

    #[test]
    fn says_which_forge_covers_a_project() {
        let list = [github()];
        let at = |remote: &str| covered(&list, PathBuf::from("/p"), Some(remote.into()));
        assert_eq!(
            at("git@github.com:acme/shop.git").by.as_deref(),
            Some("github")
        );
        let bitbucket = at("git@bitbucket.org:acme/shop.git");
        assert_eq!(
            (bitbucket.by, bitbucket.remote.as_deref()),
            (None, Some("bitbucket.org/acme/shop"))
        );
        let gitlab = at("git@gitlab.com:acme/shop.git");
        assert_eq!(
            (gitlab.by, gitlab.remote.as_deref()),
            (None, Some("gitlab.com/acme/shop"))
        );
        let azure = at("https://org@dev.azure.com/Org/My%20Proj/_git/Shop");
        assert_eq!(
            (azure.by, azure.remote.as_deref()),
            (None, Some("dev.azure.com/Org/My%20Proj/_git/Shop"))
        );
        let unknown = at("https://git.acme.io/team/wiki.git");
        assert_eq!(
            (unknown.by, unknown.remote.as_deref()),
            (None, Some("git.acme.io/team/wiki"))
        );
        assert_eq!(
            covered(
                &[],
                PathBuf::from("/p"),
                Some("git@github.com:acme/shop.git".into())
            )
            .by,
            None
        );
        assert_eq!(
            covered(&list, PathBuf::from("/p"), None),
            Coverage {
                path: PathBuf::from("/p"),
                remote: None,
                by: None
            }
        );
    }

    #[test]
    fn turns_an_extension_error_into_a_message() {
        assert_eq!(failure("GitHub answered 404"), "GitHub answered 404");
        assert_eq!(
            failure(
                r#"{"code":"account","title":{"en":"Add a GitHub account","fr":"…"},"detail":"Settings › Pull requests"}"#
            ),
            "Add a GitHub account: Settings › Pull requests"
        );
        assert_eq!(
            failure(r#"{"code":"unsupported","title":"No runs"}"#),
            "No runs"
        );
    }
}
