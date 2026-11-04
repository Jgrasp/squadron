use crate::forge::{
    self, Draft, Linked, Lists, Person, Posted, Pull, Remark, Reviewer, Run, Token, checks, text,
};
use reqwest::Method;
use serde_json::{Value, json};

const NO_TOKEN: &str = "Add a GitLab token in Settings › Code hosting.";
const REFUSED: &str = "GitLab refused the token: add a new one in Settings › Code hosting.";
const NO_REQUEST_CHANGES: &str =
    "The comments are on GitLab, but Squadron cannot request changes there: do it on GitLab.";

fn site_host(site: &str) -> Option<String> {
    reqwest::Url::parse(site)
        .ok()?
        .host_str()
        .map(str::to_lowercase)
}

pub fn host() -> Option<String> {
    forge::token("gitlab", NO_TOKEN)
        .ok()
        .and_then(|t| site_host(&t.site))
}

pub fn account() -> Option<String> {
    let token = forge::token("gitlab", NO_TOKEN).ok()?;
    let host = site_host(&token.site).unwrap_or(token.site);
    Some(format!("{}@{host}", token.name))
}

pub async fn sign_in(site: String, token: String) -> Result<(), String> {
    let site = site.trim().trim_end_matches('/');
    let site = match site {
        "" => "https://gitlab.com".to_string(),
        s if s.contains("://") => s.to_string(),
        s => format!("https://{s}"),
    };
    site_host(&site).ok_or("This GitLab address is not valid.")?;
    let user = forge::send(
        forge::client()
            .get(format!("{site}/api/v4/user"))
            .header("PRIVATE-TOKEN", &token),
        "GitLab",
        "GitLab refused this token.",
    )
    .await?;
    let name = text(&user["username"]);
    if name.is_empty() {
        return Err("GitLab refused this token.".into());
    }
    forge::keep_token("gitlab", &Token { site, token, name })
}

fn project(slug: &str) -> String {
    let path = slug.split_once('/').map_or("", |(_, path)| path);
    format!("/projects/{}", path.replace('/', "%2F"))
}

fn request(slug: &str, iid: u64) -> String {
    format!("{}/merge_requests/{iid}", project(slug))
}

async fn api(
    method: Method,
    slug: &str,
    path: &str,
    query: &[(&str, &str)],
    body: Option<Value>,
) -> Result<Value, String> {
    let token = forge::token("gitlab", NO_TOKEN)?;
    let host = slug.split('/').next().unwrap_or_default();
    if site_host(&token.site).as_deref() != Some(host) {
        return Err(NO_TOKEN.into());
    }
    let site = token.site.trim_end_matches('/');
    let url = reqwest::Url::parse_with_params(&format!("{site}/api/v4{path}"), query)
        .map_err(|e| e.to_string())?;
    let mut request = forge::client()
        .request(method, url)
        .header("PRIVATE-TOKEN", &token.token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    forge::send(request, "GitLab", REFUSED).await
}

fn username() -> String {
    forge::token("gitlab", NO_TOKEN)
        .map(|t| t.name)
        .unwrap_or_default()
}

fn verdict(state: &str) -> &'static str {
    match state {
        "approved" => "approved",
        "requested_changes" => "changes_requested",
        "reviewed" => "commented",
        _ => "waiting",
    }
}

pub(crate) fn gitlab_pull(found: &Value, reviewers: &Value, approvals: &Value, me: &str) -> Pull {
    let approved: Vec<&Value> = approvals["approved_by"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| &a["user"])
        .collect();
    let mut people: Vec<(&Value, &str)> = match reviewers.as_array() {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|r| (&r["user"], verdict(r["state"].as_str().unwrap_or_default())))
            .collect(),
        _ => found["reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|user| (user, "waiting"))
            .collect(),
    };
    for user in &approved {
        match people.iter_mut().find(|(u, _)| u["id"] == user["id"]) {
            Some(person) => person.1 = "approved",
            None => people.push((user, "approved")),
        }
    }
    let state = match found["state"].as_str().unwrap_or_default() {
        "opened" => "open",
        "locked" => "closed",
        other => other,
    };
    let draft = found["draft"] == true;
    let checks = checks(
        found["head_pipeline"]["status"]
            .as_str()
            .map(|s| match s {
                "success" | "skipped" | "manual" => "pass",
                "failed" | "canceled" => "fail",
                _ => "pending",
            })
            .into_iter(),
    );
    let mine = |u: &Value| !me.is_empty() && u["username"] == me;
    Pull {
        host: "gitlab".into(),
        number: found["iid"].as_u64().unwrap_or_default(),
        title: text(&found["title"]),
        description: text(&found["description"]),
        url: text(&found["web_url"]),
        state: state.into(),
        draft,
        approvals: approved.len() as u32,
        changes_requested: people.iter().any(|(_, v)| *v == "changes_requested"),
        comments: found["user_notes_count"].as_u64().unwrap_or_default() as u32,
        mergeable: state == "open" && !draft && found["detailed_merge_status"] == "mergeable",
        checks,
        branch: text(&found["source_branch"]),
        base: text(&found["target_branch"]),
        head: text(&found["sha"]),
        author: text(&found["author"]["name"]),
        author_avatar: text(&found["author"]["avatar_url"]),
        updated: text(&found["updated_at"]),
        reviewed: people
            .iter()
            .find(|(u, _)| mine(u))
            .map(|(_, v)| *v)
            .filter(|v| *v != "waiting")
            .map(str::to_string),
        reviewed_head: None,
        reviewed_at: approvals["approved_by"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|a| mine(&a["user"]))
            .and_then(|a| a["approved_at"].as_str())
            .map(str::to_string),
        head_at: None,
        reviewers: people
            .iter()
            .map(|(u, v)| Reviewer {
                name: text(&u["name"]),
                avatar: text(&u["avatar_url"]),
                verdict: v.to_string(),
            })
            .collect(),
        conflicts: found["has_conflicts"] == true,
    }
}

async fn full(slug: &str, iid: u64, me: &str) -> Result<(Pull, Value), String> {
    let path = request(slug, iid);
    let found = api(Method::GET, slug, &path, &[], None).await?;
    let reviewers = api(Method::GET, slug, &format!("{path}/reviewers"), &[], None)
        .await
        .unwrap_or_default();
    let approvals = api(Method::GET, slug, &format!("{path}/approvals"), &[], None)
        .await
        .unwrap_or_default();
    Ok((gitlab_pull(&found, &reviewers, &approvals, me), found))
}

async fn list(slug: &str, query: &[(&str, &str)]) -> Result<Vec<u64>, String> {
    let path = format!("{}/merge_requests", project(slug));
    let found = api(Method::GET, slug, &path, query, None).await?;
    Ok(found
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["iid"].as_u64())
        .collect())
}

pub async fn pull(slug: &str, branch: &str) -> Result<Option<Pull>, String> {
    let found = list(
        slug,
        &[
            ("source_branch", branch),
            ("order_by", "updated_at"),
            ("per_page", "1"),
        ],
    )
    .await?;
    match found.first() {
        Some(iid) => Ok(Some(full(slug, *iid, &username()).await?.0)),
        None => Ok(None),
    }
}

pub async fn linked(slug: &str, iid: u64) -> Result<Linked, String> {
    let (pull, found) = full(slug, iid, &username()).await?;
    Ok(Linked {
        pull,
        files: text(&found["changes_count"])
            .trim_end_matches('+')
            .parse()
            .ok(),
        ..Linked::default()
    })
}

pub async fn lists(slug: &str) -> Result<Lists, String> {
    let me = username();
    let mut found = Vec::new();
    for who in ["reviewer_username", "author_username"] {
        let mut pulls = Vec::new();
        let query = [
            (who, me.as_str()),
            ("state", "opened"),
            ("order_by", "updated_at"),
            ("per_page", "50"),
        ];
        for iid in list(slug, &query).await? {
            pulls.push(full(slug, iid, &me).await?.0);
        }
        found.push(pulls);
    }
    let mine = found.pop().unwrap_or_default();
    let review = found.pop().unwrap_or_default();
    Ok(Lists { review, mine })
}

async fn discussions(slug: &str, iid: u64) -> Result<Vec<Value>, String> {
    let path = format!("{}/discussions", request(slug, iid));
    let mut found = Vec::new();
    for page in 1..=20 {
        let page = page.to_string();
        let list = api(
            Method::GET,
            slug,
            &path,
            &[("per_page", "100"), ("page", &page)],
            None,
        )
        .await?;
        let list = list.as_array().cloned().unwrap_or_default();
        let last = list.len() < 100;
        found.extend(list);
        if last {
            break;
        }
    }
    Ok(found)
}

pub(crate) fn gitlab_remarks(discussion: &Value) -> Vec<Remark> {
    let notes: Vec<&Value> = discussion["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["system"] != true)
        .collect();
    let root = notes.first().and_then(|n| n["id"].as_u64());
    notes
        .iter()
        .filter_map(|note| {
            let id = note["id"].as_u64()?;
            let position = &note["position"];
            let (new, old) = (position["new_line"].as_u64(), position["old_line"].as_u64());
            let path = position["new_path"]
                .as_str()
                .or(position["old_path"].as_str());
            Some(Remark {
                id,
                author: text(&note["author"]["name"]),
                avatar: text(&note["author"]["avatar_url"]),
                body: text(&note["body"]),
                path: path.map(str::to_string),
                line: new.or(old).map(|n| n as u32),
                removed: new.is_none() && old.is_some(),
                reply_to: root.filter(|r| *r != id),
                at: text(&note["created_at"]),
                kind: if path.is_some() { "line" } else { "general" }.into(),
            })
        })
        .collect()
}

pub async fn remarks(slug: &str, iid: u64) -> Result<Vec<Remark>, String> {
    let mut found: Vec<Remark> = discussions(slug, iid)
        .await?
        .iter()
        .flat_map(gitlab_remarks)
        .collect();
    found.sort_by(|a, b| a.at.cmp(&b.at));
    Ok(found)
}

pub async fn me(slug: &str) -> Result<Person, String> {
    let user = api(Method::GET, slug, "/user", &[], None).await?;
    Ok(Person {
        name: text(&user["name"]),
        avatar: text(&user["avatar_url"]),
    })
}

pub async fn reply(slug: &str, iid: u64, to: u64, body: String) -> Result<(), String> {
    let path = request(slug, iid);
    let thread = discussions(slug, iid).await?.into_iter().find(|d| {
        d["notes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|n| n["id"].as_u64() == Some(to))
    });
    let url = match thread.as_ref().and_then(|d| d["id"].as_str()) {
        Some(id) => format!("{path}/discussions/{id}/notes"),
        None => format!("{path}/notes"),
    };
    api(Method::POST, slug, &url, &[], Some(json!({ "body": body })))
        .await
        .map(|_| ())
}

pub async fn post(
    slug: &str,
    iid: u64,
    drafts: Vec<Draft>,
    summary: String,
    verdict: String,
) -> Result<Posted, String> {
    let path = request(slug, iid);
    let refs = api(Method::GET, slug, &path, &[], None).await?["diff_refs"].take();
    let total = drafts.len();
    for (done, draft) in drafts.into_iter().enumerate() {
        let sent = if draft.path.is_empty() {
            let note = json!({ "body": draft.body });
            api(
                Method::POST,
                slug,
                &format!("{path}/notes"),
                &[],
                Some(note),
            )
            .await
        } else {
            let mut position = json!({
                "position_type": "text",
                "base_sha": refs["base_sha"],
                "start_sha": refs["start_sha"],
                "head_sha": refs["head_sha"],
                "new_path": draft.path,
                "old_path": draft.path,
            });
            position[if draft.removed {
                "old_line"
            } else {
                "new_line"
            }] = json!(draft.line);
            let thread = json!({ "body": draft.body, "position": position });
            api(
                Method::POST,
                slug,
                &format!("{path}/discussions"),
                &[],
                Some(thread),
            )
            .await
        };
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
        let note = json!({ "body": summary });
        api(
            Method::POST,
            slug,
            &format!("{path}/notes"),
            &[],
            Some(note),
        )
        .await
    };
    let said = sent.is_ok();
    match verdict.as_str() {
        "approve" if sent.is_ok() => {
            sent = api(Method::POST, slug, &format!("{path}/approve"), &[], None).await;
        }
        "request_changes" if sent.is_ok() => sent = Err(NO_REQUEST_CHANGES.into()),
        _ => {}
    }
    Ok(Posted {
        done: total,
        said,
        error: sent.err(),
    })
}

pub async fn merge(slug: &str, iid: u64) -> Result<(), String> {
    let path = format!("{}/merge", request(slug, iid));
    api(Method::PUT, slug, &path, &[], None).await.map(|_| ())
}

pub(crate) fn gitlab_run(value: &Value) -> Run {
    let status = match value["status"].as_str().unwrap_or_default() {
        "success" => "success",
        "failed" => "failure",
        "canceled" | "canceling" => "cancelled",
        "skipped" => "skipped",
        "running" => "running",
        _ => "queued",
    };
    let source = text(&value["source"]);
    let branch = text(&value["ref"]);
    Run {
        host: "gitlab".into(),
        id: value["id"].as_u64().unwrap_or_default().to_string(),
        number: value["iid"].as_u64().unwrap_or_default(),
        name: value["name"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| source.clone()),
        title: branch.clone(),
        event: source,
        branch,
        url: text(&value["web_url"]),
        started: text(&value["created_at"]),
        finished: matches!(status, "success" | "failure" | "cancelled" | "skipped")
            .then(|| text(&value["updated_at"])),
        status: status.into(),
        author: String::new(),
        author_avatar: String::new(),
    }
}

pub async fn runs(slug: &str) -> Result<Vec<Run>, String> {
    let path = format!("{}/pipelines", project(slug));
    let list = api(Method::GET, slug, &path, &[("per_page", "20")], None).await?;
    Ok(list
        .as_array()
        .into_iter()
        .flatten()
        .map(gitlab_run)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_up_a_merge_request() {
        let user = |id: u64, username: &str, name: &str| json!({ "id": id, "username": username, "name": name, "avatar_url": format!("https://gitlab.com/{username}.png") });
        let found = json!({
            "iid": 7767, "title": "go: Update module k8s.io/api to v0.37.2", "description": "Bump", "state": "opened", "draft": false,
            "detailed_merge_status": "mergeable", "has_conflicts": false, "user_notes_count": 2,
            "source_branch": "renovate/k8s.io-api-0.x", "target_branch": "main", "sha": "2c7a3330",
            "web_url": "https://gitlab.com/gitlab-org/gitlab-runner/-/merge_requests/7767", "updated_at": "2026-11-04T09:32:42.995Z",
            "head_pipeline": { "status": "running" },
            "reviewers": [user(8111826, "ratchade", "Romuald Atchadé"), user(10764887, "vtak", "Vishal Tak")],
            "author": user(8420142, "gitlab-dependency-update-bot", "GitLab Dependency Bot")
        });
        let approvals = json!({ "approved_by": [{ "user": user(10764887, "vtak", "Vishal Tak"), "approved_at": "2026-11-04T09:32:31.669Z" }] });
        let pull = gitlab_pull(&found, &Value::Null, &approvals, "vtak");
        assert_eq!(
            (
                pull.number,
                pull.state.as_str(),
                pull.approvals,
                pull.comments,
                pull.checks.as_deref()
            ),
            (7767, "open", 1, 2, Some("pending"))
        );
        assert_eq!(
            (
                pull.branch.as_str(),
                pull.base.as_str(),
                pull.author.as_str(),
                pull.mergeable
            ),
            (
                "renovate/k8s.io-api-0.x",
                "main",
                "GitLab Dependency Bot",
                true
            )
        );
        assert_eq!(
            (pull.reviewed.as_deref(), pull.reviewed_at.as_deref()),
            (Some("approved"), Some("2026-11-04T09:32:31.669Z"))
        );
        let seen: Vec<_> = pull
            .reviewers
            .iter()
            .map(|r| (r.name.as_str(), r.verdict.as_str()))
            .collect();
        assert_eq!(
            seen,
            [("Romuald Atchadé", "waiting"), ("Vishal Tak", "approved")]
        );
        let states = json!([{ "user": user(8111826, "ratchade", "Romuald Atchadé"), "state": "requested_changes" }]);
        let asked = gitlab_pull(&found, &states, &Value::Null, "ratchade");
        assert!(asked.changes_requested);
        assert_eq!(asked.reviewed.as_deref(), Some("changes_requested"));
        let mut merged = found.clone();
        merged["state"] = json!("merged");
        assert!(!gitlab_pull(&merged, &Value::Null, &Value::Null, "").mergeable);
    }

    #[test]
    fn reads_remarks_of_a_discussion() {
        let discussion = json!({ "id": "6a9c", "notes": [
            { "id": 1, "system": true, "body": "added 1 commit", "author": { "name": "A" }, "created_at": "t0" },
            { "id": 2, "type": "DiffNote", "body": "Rename", "author": { "name": "Ann", "avatar_url": "https://a/ann.png" }, "created_at": "t1", "position": { "new_path": "src/a.go", "old_path": "src/a.go", "new_line": null, "old_line": 4 } },
            { "id": 3, "type": "DiffNote", "body": "Done", "author": { "name": "Bob" }, "created_at": "t2", "position": { "new_path": "src/a.go", "new_line": 4 } }
        ] });
        let found = gitlab_remarks(&discussion);
        let seen: Vec<_> = found
            .iter()
            .map(|r| (r.id, r.line, r.removed, r.reply_to, r.kind.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                (2, Some(4), true, None, "line"),
                (3, Some(4), false, Some(2), "line")
            ]
        );
        let general = json!({ "notes": [{ "id": 9, "body": "LGTM", "author": { "name": "A" }, "created_at": "t" }] });
        assert_eq!(gitlab_remarks(&general)[0].kind, "general");
    }

    #[test]
    fn sums_up_a_pipeline() {
        let run = gitlab_run(
            &json!({ "id": 3026531863u64, "iid": 54657, "ref": "refs/merge-requests/7673/merge", "status": "failed", "source": "merge_request_event", "created_at": "a", "updated_at": "b", "web_url": "https://gitlab.com/p/-/pipelines/3026531863", "name": null }),
        );
        assert_eq!(
            (
                run.id.as_str(),
                run.number,
                run.name.as_str(),
                run.status.as_str(),
                run.finished.as_deref()
            ),
            (
                "3026531863",
                54657,
                "merge_request_event",
                "failure",
                Some("b")
            )
        );
        assert_eq!(gitlab_run(&json!({ "status": "pending" })).finished, None);
    }

    #[test]
    fn keeps_the_project_path_encoded() {
        let url = reqwest::Url::parse(&format!(
            "https://gitlab.com/api/v4{}",
            project("gitlab.com/grp/sub/proj")
        ))
        .unwrap();
        assert_eq!(url.path(), "/api/v4/projects/grp%2Fsub%2Fproj");
    }
}
