use crate::forge::{
    self, Draft, Linked, Lists, Person, Posted, Pull, Remark, Reviewer, Run, Token, checks, text,
};
use reqwest::Method;
use serde_json::{Value, json};

const NO_TOKEN: &str = "Add an Azure DevOps token in Settings › Code hosting.";
const REFUSED: &str = "Azure DevOps refused the token: add a new one in Settings › Code hosting.";
const SITE: &str = "https://dev.azure.com";

pub fn account() -> Option<String> {
    forge::token("azure", NO_TOKEN).ok().map(|t| t.name)
}

async fn connection(org: &str, token: &str, refused: &str) -> Result<Value, String> {
    let request = forge::client()
        .get(format!(
            "{SITE}/{org}/_apis/connectionData?api-version=7.1-preview"
        ))
        .basic_auth("", Some(token));
    let mut found = forge::send(request, "Azure DevOps", refused).await?;
    let user = found["authenticatedUser"].take();
    if user["id"].is_string() {
        Ok(user)
    } else {
        Err(refused.into())
    }
}

pub async fn sign_in(organization: String, token: String) -> Result<(), String> {
    let site = organization.trim().trim_end_matches('/');
    let org = site.rsplit('/').next().unwrap_or_default().to_string();
    if org.is_empty() {
        return Err("Enter the Azure DevOps organization.".into());
    }
    let user = connection(&org, &token, "Azure DevOps refused this token.").await?;
    let name = text(&user["providerDisplayName"]);
    forge::keep_token(
        "azure",
        &Token {
            site: org,
            token,
            name,
        },
    )
}

fn parts(slug: &str) -> (&str, &str, &str) {
    let mut parts = slug.splitn(3, '/');
    (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
    )
}

fn repository(slug: &str) -> String {
    let (org, project, name) = parts(slug);
    format!("{SITE}/{org}/{project}/_apis/git/repositories/{name}")
}

fn pull_api(slug: &str, number: u64) -> String {
    format!("{}/pullRequests/{number}", repository(slug))
}

async fn api(
    method: Method,
    url: &str,
    query: &[(&str, &str)],
    body: Option<Value>,
) -> Result<Value, String> {
    let token = forge::token("azure", NO_TOKEN)?;
    let mut query = query.to_vec();
    query.push(("api-version", "7.1"));
    let url = reqwest::Url::parse_with_params(url, &query).map_err(|e| e.to_string())?;
    let mut request = forge::client()
        .request(method, url)
        .basic_auth("", Some(&token.token));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let found = forge::send(request, "Azure DevOps", REFUSED).await?;
    if found.is_null() {
        return Err(REFUSED.into());
    }
    Ok(found)
}

async fn me_id(slug: &str) -> Result<String, String> {
    let token = forge::token("azure", NO_TOKEN)?;
    let user = connection(parts(slug).0, &token.token, REFUSED).await?;
    Ok(text(&user["id"]))
}

fn branch(value: &Value) -> String {
    text(value).trim_start_matches("refs/heads/").to_string()
}

fn verdict(vote: i64) -> &'static str {
    match vote {
        v if v > 0 => "approved",
        v if v < 0 => "changes_requested",
        _ => "waiting",
    }
}

fn spoken(threads: &Value) -> impl Iterator<Item = (&Value, &Value)> {
    threads["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| t["isDeleted"] != true)
        .flat_map(|t| {
            t["comments"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| c["commentType"] == "text" && c["isDeleted"] != true)
                .map(move |c| (t, c))
        })
}

pub(crate) fn azure_pull(
    found: &Value,
    statuses: &Value,
    threads: &Value,
    me: &str,
    slug: &str,
) -> Pull {
    let people = found["reviewers"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let vote = |p: &Value| verdict(p["vote"].as_i64().unwrap_or_default());
    let approvals = people.iter().filter(|p| vote(p) == "approved").count() as u32;
    let changes_requested = people.iter().any(|p| vote(p) == "changes_requested");
    let checks = checks(
        statuses["value"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| match s["state"].as_str() {
                Some("succeeded") => Some("pass"),
                Some("pending") => Some("pending"),
                Some("failed" | "error") => Some("fail"),
                _ => None,
            }),
    );
    let state = match found["status"].as_str().unwrap_or_default() {
        "active" => "open",
        "completed" => "merged",
        "abandoned" => "closed",
        other => other,
    };
    let draft = found["isDraft"] == true;
    let number = found["pullRequestId"].as_u64().unwrap_or_default();
    let (org, project, name) = parts(slug);
    Pull {
        host: "azure".into(),
        number,
        title: text(&found["title"]),
        description: text(&found["description"]),
        url: format!("{SITE}/{org}/{project}/_git/{name}/pullrequest/{number}"),
        state: state.into(),
        draft,
        approvals,
        changes_requested,
        comments: spoken(threads).count() as u32,
        mergeable: state == "open"
            && !draft
            && found["mergeStatus"] == "succeeded"
            && approvals > 0
            && !changes_requested
            && checks.as_deref().is_none_or(|c| c == "passing"),
        checks,
        branch: branch(&found["sourceRefName"]),
        base: branch(&found["targetRefName"]),
        head: text(&found["lastMergeSourceCommit"]["commitId"]),
        author: text(&found["createdBy"]["displayName"]),
        author_avatar: String::new(),
        updated: found["closedDate"]
            .as_str()
            .or(found["creationDate"].as_str())
            .unwrap_or_default()
            .into(),
        reviewed: people
            .iter()
            .find(|p| !me.is_empty() && p["id"] == me)
            .map(vote)
            .filter(|v| *v != "waiting")
            .map(str::to_string),
        reviewed_head: None,
        reviewed_at: None,
        head_at: None,
        reviewers: people
            .iter()
            .map(|p| Reviewer {
                name: text(&p["displayName"]),
                avatar: String::new(),
                verdict: vote(p).into(),
            })
            .collect(),
        conflicts: found["mergeStatus"] == "conflicts",
    }
}

async fn full(slug: &str, found: &Value, me: &str) -> Pull {
    let path = pull_api(slug, found["pullRequestId"].as_u64().unwrap_or_default());
    let statuses = api(Method::GET, &format!("{path}/statuses"), &[], None)
        .await
        .unwrap_or_default();
    let threads = api(Method::GET, &format!("{path}/threads"), &[], None)
        .await
        .unwrap_or_default();
    azure_pull(found, &statuses, &threads, me, slug)
}

async fn search(slug: &str, query: &[(&str, &str)], me: &str) -> Result<Vec<Pull>, String> {
    let url = format!("{}/pullrequests", repository(slug));
    let list = api(Method::GET, &url, query, None).await?;
    let mut found = Vec::new();
    for pull in list["value"].as_array().into_iter().flatten() {
        found.push(full(slug, pull, me).await);
    }
    Ok(found)
}

pub async fn pull(slug: &str, name: &str) -> Result<Option<Pull>, String> {
    let source = format!("refs/heads/{name}");
    let me = me_id(slug).await.unwrap_or_default();
    let query = [
        ("searchCriteria.sourceRefName", source.as_str()),
        ("searchCriteria.status", "all"),
        ("$top", "1"),
    ];
    Ok(search(slug, &query, &me).await?.into_iter().next())
}

pub async fn linked(slug: &str, number: u64) -> Result<Linked, String> {
    let found = api(Method::GET, &pull_api(slug, number), &[], None).await?;
    let me = me_id(slug).await.unwrap_or_default();
    Ok(Linked {
        pull: full(slug, &found, &me).await,
        ..Linked::default()
    })
}

pub async fn lists(slug: &str) -> Result<Lists, String> {
    let me = me_id(slug).await?;
    let list = |who: &'static str| {
        let me = me.clone();
        async move {
            let query = [
                (who, me.as_str()),
                ("searchCriteria.status", "active"),
                ("$top", "50"),
            ];
            search(slug, &query, &me).await
        }
    };
    Ok(Lists {
        review: list("searchCriteria.reviewerId").await?,
        mine: list("searchCriteria.creatorId").await?,
    })
}

pub(crate) fn azure_remarks(threads: &Value) -> Vec<Remark> {
    spoken(threads)
        .filter_map(|(thread, comment)| {
            let tid = thread["id"].as_u64()?;
            let id = comment["id"].as_u64()?;
            let context = &thread["threadContext"];
            let path = context["filePath"]
                .as_str()
                .map(|p| p.trim_start_matches('/').to_string());
            let right = context["rightFileStart"]["line"].as_u64();
            let left = context["leftFileStart"]["line"].as_u64();
            let parent = comment["parentCommentId"].as_u64().unwrap_or_default();
            Some(Remark {
                id: tid << 16 | id,
                author: text(&comment["author"]["displayName"]),
                avatar: String::new(),
                body: text(&comment["content"]),
                kind: if path.is_some() { "line" } else { "general" }.into(),
                path,
                line: right.or(left).map(|n| n as u32),
                removed: right.is_none() && left.is_some(),
                reply_to: (parent > 0).then_some(tid << 16 | parent),
                at: text(&comment["publishedDate"]),
            })
        })
        .collect()
}

pub async fn remarks(slug: &str, number: u64) -> Result<Vec<Remark>, String> {
    let threads = api(
        Method::GET,
        &format!("{}/threads", pull_api(slug, number)),
        &[],
        None,
    )
    .await?;
    let mut found = azure_remarks(&threads);
    found.sort_by(|a, b| a.at.cmp(&b.at));
    Ok(found)
}

pub async fn me(slug: &str) -> Result<Person, String> {
    let token = forge::token("azure", NO_TOKEN)?;
    let user = connection(parts(slug).0, &token.token, REFUSED).await?;
    Ok(Person {
        name: text(&user["providerDisplayName"]),
        avatar: String::new(),
    })
}

pub async fn reply(slug: &str, number: u64, to: u64, body: String) -> Result<(), String> {
    let url = format!("{}/threads/{}/comments", pull_api(slug, number), to >> 16);
    let comment = json!({ "content": body, "parentCommentId": to & 0xffff, "commentType": 1 });
    api(Method::POST, &url, &[], Some(comment))
        .await
        .map(|_| ())
}

fn thread(body: &str, draft: Option<&Draft>) -> Value {
    let mut thread = json!({
        "comments": [{ "parentCommentId": 0, "content": body, "commentType": 1 }],
        "status": "active",
    });
    if let Some(draft) = draft.filter(|d| !d.path.is_empty()) {
        let line = json!({ "line": draft.line, "offset": 1 });
        let side = if draft.removed { "left" } else { "right" };
        let context = &mut thread["threadContext"];
        context["filePath"] = json!(format!("/{}", draft.path.trim_start_matches('/')));
        context[format!("{side}FileStart")] = line.clone();
        context[format!("{side}FileEnd")] = line;
    }
    thread
}

pub async fn post(
    slug: &str,
    number: u64,
    drafts: Vec<Draft>,
    summary: String,
    verdict: String,
) -> Result<Posted, String> {
    let path = pull_api(slug, number);
    let threads = format!("{path}/threads");
    let total = drafts.len();
    for (done, draft) in drafts.iter().enumerate() {
        let sent = api(
            Method::POST,
            &threads,
            &[],
            Some(thread(&draft.body, Some(draft))),
        )
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
        api(Method::POST, &threads, &[], Some(thread(&summary, None))).await
    };
    let said = sent.is_ok();
    let vote = match verdict.as_str() {
        "approve" => Some(10),
        "request_changes" => Some(-5),
        _ => None,
    };
    if let (Ok(_), Some(vote)) = (&sent, vote) {
        sent = match me_id(slug).await {
            Ok(me) => {
                let url = format!("{path}/reviewers/{me}");
                api(Method::PUT, &url, &[], Some(json!({ "vote": vote }))).await
            }
            Err(e) => Err(e),
        };
    }
    Ok(Posted {
        done: total,
        said,
        error: sent.err(),
    })
}

pub async fn merge(slug: &str, number: u64) -> Result<(), String> {
    let path = pull_api(slug, number);
    let found = api(Method::GET, &path, &[], None).await?;
    let done =
        json!({ "status": "completed", "lastMergeSourceCommit": found["lastMergeSourceCommit"] });
    api(Method::PATCH, &path, &[], Some(done)).await.map(|_| ())
}

pub async fn update(slug: &str, number: u64, changes: Value) -> Result<(), String> {
    api(Method::PATCH, &pull_api(slug, number), &[], Some(changes))
        .await
        .map(|_| ())
}

pub(crate) fn azure_run(value: &Value) -> Run {
    let status = match (
        value["status"].as_str().unwrap_or_default(),
        value["result"].as_str().unwrap_or_default(),
    ) {
        ("completed", "succeeded") => "success",
        ("completed", "canceled") => "cancelled",
        ("completed", _) => "failure",
        ("inProgress" | "cancelling", _) => "running",
        _ => "queued",
    };
    let id = value["id"].as_u64().unwrap_or_default();
    Run {
        host: "azure".into(),
        id: id.to_string(),
        number: id,
        name: text(&value["definition"]["name"]),
        title: text(&value["triggerInfo"]["ci.message"])
            .lines()
            .next()
            .unwrap_or_default()
            .to_string(),
        event: text(&value["reason"]),
        branch: branch(&value["sourceBranch"]),
        status: status.into(),
        url: text(&value["_links"]["web"]["href"]),
        started: value["startTime"]
            .as_str()
            .or(value["queueTime"].as_str())
            .unwrap_or_default()
            .into(),
        finished: (value["status"] == "completed").then(|| text(&value["finishTime"])),
        author: text(&value["requestedFor"]["displayName"]),
        author_avatar: String::new(),
    }
}

pub async fn runs(slug: &str) -> Result<Vec<Run>, String> {
    let (org, project, _) = parts(slug);
    let found = api(Method::GET, &repository(slug), &[], None).await?;
    let id = text(&found["id"]);
    let list = api(
        Method::GET,
        &format!("{SITE}/{org}/{project}/_apis/build/builds"),
        &[
            ("repositoryId", id.as_str()),
            ("repositoryType", "TfsGit"),
            ("$top", "20"),
            ("queryOrder", "queueTimeDescending"),
        ],
        None,
    )
    .await?;
    Ok(list["value"]
        .as_array()
        .into_iter()
        .flatten()
        .map(azure_run)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_up_a_pull_request() {
        let found = json!({
            "pullRequestId": 5, "status": "active", "title": "Inventory bootstrapping", "description": "Adds metadata",
            "createdBy": { "displayName": "MerlinBot", "id": "56b0" }, "creationDate": "2026-02-18T04:35:01Z",
            "sourceRefName": "refs/heads/invBootstrap", "targetRefName": "refs/heads/main", "mergeStatus": "succeeded", "isDraft": false,
            "lastMergeSourceCommit": { "commitId": "3aae318f" },
            "reviewers": [{ "vote": 10, "displayName": "Mark Wilkie", "id": "08c3" }, { "vote": 0, "displayName": "[public]\\Reviewers", "id": "ab54", "isContainer": true }]
        });
        let threads = json!({ "value": [
            { "id": 8, "comments": [{ "id": 1, "parentCommentId": 0, "content": "MerlinBot added 3 reviewers", "commentType": "system" }] },
            { "id": 11, "threadContext": { "filePath": "/src/a.cs", "rightFileStart": { "line": 12, "offset": 1 } }, "comments": [
                { "id": 1, "parentCommentId": 0, "content": "Rename", "commentType": "text", "author": { "displayName": "Ann" }, "publishedDate": "t1" },
                { "id": 2, "parentCommentId": 1, "content": "Done", "commentType": "text", "author": { "displayName": "Bob" }, "publishedDate": "t2" },
                { "id": 3, "parentCommentId": 1, "commentType": "text", "isDeleted": true }
            ] },
            { "id": 12, "isDeleted": true, "comments": [{ "id": 1, "content": "x", "commentType": "text" }] }
        ] });
        let statuses = json!({ "value": [{ "state": "succeeded" }, { "state": "notApplicable" }] });
        let pull = azure_pull(&found, &statuses, &threads, "08c3", "org/my%20proj/shop");
        assert_eq!(
            (
                pull.url.as_str(),
                pull.state.as_str(),
                pull.branch.as_str(),
                pull.base.as_str(),
                pull.head.as_str()
            ),
            (
                "https://dev.azure.com/org/my%20proj/_git/shop/pullrequest/5",
                "open",
                "invBootstrap",
                "main",
                "3aae318f"
            )
        );
        assert_eq!(
            (
                pull.approvals,
                pull.comments,
                pull.checks.as_deref(),
                pull.mergeable,
                pull.reviewed.as_deref()
            ),
            (1, 2, Some("passing"), true, Some("approved"))
        );
        let seen: Vec<_> = pull.reviewers.iter().map(|r| r.verdict.as_str()).collect();
        assert_eq!(seen, ["approved", "waiting"]);
        let mut rejected = found.clone();
        rejected["reviewers"][1]["vote"] = json!(-5);
        rejected["status"] = json!("completed");
        let pull = azure_pull(&rejected, &Value::Null, &Value::Null, "", "o/p/r");
        assert_eq!(
            (pull.state.as_str(), pull.changes_requested, pull.mergeable),
            ("merged", true, false)
        );

        let remarks = azure_remarks(&threads);
        let seen: Vec<_> = remarks
            .iter()
            .map(|r| (r.id, r.path.as_deref(), r.line, r.reply_to))
            .collect();
        assert_eq!(
            seen,
            [
                (11 << 16 | 1, Some("src/a.cs"), Some(12), None),
                (11 << 16 | 2, Some("src/a.cs"), Some(12), Some(11 << 16 | 1))
            ]
        );
    }

    #[test]
    fn places_a_draft_on_its_side_of_the_diff() {
        let draft = Draft {
            path: "src/a.cs".into(),
            line: 4,
            removed: true,
            body: "Why?".into(),
        };
        let posted = thread("Why?", Some(&draft));
        assert_eq!(posted["threadContext"]["filePath"], "/src/a.cs");
        assert_eq!(posted["threadContext"]["leftFileStart"]["line"], 4);
        assert!(thread("LGTM", None)["threadContext"].is_null());
    }

    #[test]
    fn sums_up_a_build() {
        let run = azure_run(
            &json!({ "id": 1663614, "buildNumber": "20261104.17", "status": "completed", "result": "partiallySucceeded", "queueTime": "q", "startTime": "s", "finishTime": "f", "sourceBranch": "refs/heads/main", "reason": "individualCI", "definition": { "name": "maui-pr" }, "requestedFor": { "displayName": "Microsoft.VisualStudio.Services.TFS" }, "triggerInfo": { "ci.message": "[iOS] Fix badge\n\nmore" }, "_links": { "web": { "href": "https://dev.azure.com/o/p/_build/results?buildId=1663614" } } }),
        );
        assert_eq!(
            (
                run.name.as_str(),
                run.title.as_str(),
                run.branch.as_str(),
                run.status.as_str(),
                run.started.as_str(),
                run.finished.as_deref()
            ),
            (
                "maui-pr",
                "[iOS] Fix badge",
                "main",
                "failure",
                "s",
                Some("f")
            )
        );
        let queued = azure_run(&json!({ "status": "notStarted", "queueTime": "q" }));
        assert_eq!(
            (
                queued.status.as_str(),
                queued.started.as_str(),
                queued.finished
            ),
            ("queued", "q", None)
        );
    }
}
