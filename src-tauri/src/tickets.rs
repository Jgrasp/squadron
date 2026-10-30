use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

#[derive(Clone, Serialize, Deserialize)]
struct Secret {
    id: String,
    name: String,
    site: String,
    email: String,
    token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub site: String,
    pub email: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub connection: String,
    pub key: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Serialize)]
pub struct JiraProject {
    pub key: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct Person {
    pub name: String,
    pub avatar: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub connection: String,
    pub key: String,
    pub title: String,
    pub status: String,
    pub category: String,
    pub kind: String,
    pub priority: String,
    pub urgency: u8,
    pub epic: String,
    pub assignee: Option<Person>,
    pub mine: bool,
}

#[derive(Serialize)]
pub struct Sprint {
    pub tickets: Vec<Summary>,
    pub sprints: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct Comment {
    pub id: String,
    pub parent: Option<String>,
    pub author: Person,
    pub created: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct Link {
    pub relation: String,
    pub key: String,
    pub title: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct Field {
    pub name: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct Ticket {
    pub connection: String,
    pub key: String,
    pub url: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub category: String,
    pub priority: String,
    pub urgency: u8,
    pub assignee: Option<Person>,
    pub reporter: Option<Person>,
    pub labels: Vec<String>,
    pub components: Vec<String>,
    pub versions: Vec<String>,
    pub epic: Option<Link>,
    pub sprint: String,
    pub updated: String,
    pub description: String,
    pub subtasks: Vec<Link>,
    pub links: Vec<Link>,
    pub attachments: Vec<String>,
    pub comments: Vec<Comment>,
    pub custom: Vec<Field>,
}

pub const FIELDS: [&str; 13] = [
    "description",
    "comments",
    "subtasks",
    "links",
    "attachments",
    "epic",
    "type",
    "status",
    "labels",
    "components",
    "versions",
    "people",
    "custom",
];

fn entry() -> Result<keyring::Entry, String> {
    let service = if cfg!(debug_assertions) {
        "squadron-dev"
    } else {
        "squadron"
    };
    keyring::Entry::new(service, "tickets").map_err(|e| e.to_string())
}

fn secrets() -> Result<Vec<Secret>, String> {
    match entry()?.get_password() {
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string()),
        Err(keyring::Error::NoEntry) => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn keep(list: &[Secret]) -> Result<(), String> {
    let text = serde_json::to_string(list).map_err(|e| e.to_string())?;
    entry()?.set_password(&text).map_err(|e| e.to_string())
}

fn secret(id: &str) -> Result<Secret, String> {
    secrets()?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "This Jira connection no longer exists.".into())
}

pub fn connections() -> Result<Vec<Connection>, String> {
    Ok(secrets()?
        .into_iter()
        .map(|s| Connection {
            id: s.id,
            name: s.name,
            site: s.site,
            email: s.email,
        })
        .collect())
}

pub fn site(input: &str) -> String {
    let site = input.trim().trim_end_matches('/');
    let site = site
        .strip_prefix("https://")
        .or_else(|| site.strip_prefix("http://"))
        .unwrap_or(site);
    site.split('/').next().unwrap_or_default().to_lowercase()
}

pub async fn connect(
    id: Option<String>,
    name: String,
    site_input: String,
    email: String,
    token: String,
) -> Result<Vec<Connection>, String> {
    let mut list = secrets()?;
    let old = id
        .as_ref()
        .and_then(|id| list.iter().find(|s| &s.id == id))
        .cloned();
    let token = match (token.trim(), &old) {
        ("", Some(old)) => old.token.clone(),
        ("", None) => return Err("Enter a Jira API token.".into()),
        (token, _) => token.to_string(),
    };
    let found = Secret {
        id: id.unwrap_or_else(|| format!("{:x}", rand_id())),
        name: name.trim().to_string(),
        site: site(&site_input),
        email: email.trim().to_string(),
        token,
    };
    if found.site.is_empty() || !found.site.contains('.') {
        return Err("Enter the Jira site, such as company.atlassian.net.".into());
    }
    let me = request(&found, Method::GET, "/rest/api/3/myself", &[], None).await?;
    let mut found = found;
    if found.name.is_empty() {
        found.name = found.site.split('.').next().unwrap_or_default().to_string();
    }
    if found.email.is_empty() {
        found.email = text(&me["emailAddress"]);
    }
    match list.iter_mut().find(|s| s.id == found.id) {
        Some(slot) => *slot = found,
        None => list.push(found),
    }
    keep(&list)?;
    connections()
}

fn rand_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

pub fn disconnect(id: &str) -> Result<Vec<Connection>, String> {
    let mut list = secrets()?;
    list.retain(|s| s.id != id);
    keep(&list)?;
    connections()
}

fn client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}

async fn request(
    secret: &Secret,
    method: Method,
    path: &str,
    query: &[(&str, &str)],
    body: Option<Value>,
) -> Result<Value, String> {
    let url = reqwest::Url::parse_with_params(&format!("https://{}{path}", secret.site), query)
        .map_err(|e| e.to_string())?;
    let mut call = client()
        .request(method, url)
        .basic_auth(&secret.email, Some(&secret.token))
        .header("Accept", "application/json");
    if let Some(body) = body {
        call = call.json(&body);
    }
    let response = call.send().await.map_err(|e| {
        if e.is_connect() {
            format!("Jira could not be reached at {}.", secret.site)
        } else {
            e.to_string()
        }
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Jira refused this email and token.".into());
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err("Jira did not find this ticket or project.".into());
    }
    if !status.is_success() {
        let body: Value = response.json().await.unwrap_or_default();
        let message = body["errorMessages"]
            .as_array()
            .and_then(|m| m.first())
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| status.to_string());
        return Err(format!("Jira: {message}"));
    }
    response.json().await.map_err(|e| e.to_string())
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

fn person(value: &Value) -> Option<Person> {
    value.is_object().then(|| Person {
        name: text(&value["displayName"]),
        avatar: text(&value["avatarUrls"]["48x48"]),
    })
}

pub async fn projects(connection: &str) -> Result<Vec<JiraProject>, String> {
    let secret = secret(connection)?;
    let mut found = Vec::new();
    let mut start = 0;
    loop {
        let at = start.to_string();
        let page = request(
            &secret,
            Method::GET,
            "/rest/api/3/project/search",
            &[("startAt", &at), ("maxResults", "100"), ("orderBy", "key")],
            None,
        )
        .await?;
        let values = page["values"].as_array().cloned().unwrap_or_default();
        found.extend(values.iter().map(|p| JiraProject {
            key: text(&p["key"]),
            name: text(&p["name"]),
        }));
        start += values.len();
        if values.is_empty() || page["isLast"].as_bool().unwrap_or(true) || start >= 2000 {
            return Ok(found);
        }
    }
}

async fn sprint_field(secret: &Secret) -> Option<String> {
    let fields = request(secret, Method::GET, "/rest/api/3/field", &[], None)
        .await
        .ok()?;
    fields.as_array()?.iter().find_map(|f| {
        (f["schema"]["custom"].as_str() == Some("com.pyxis.greenhopper.jira:gh-sprint"))
            .then(|| text(&f["id"]))
    })
}

fn urgency(priority: &Value) -> u8 {
    match priority["id"].as_str() {
        Some("1" | "2") => 3,
        Some("3") => 2,
        Some("4" | "5") => 1,
        _ => 0,
    }
}

fn jql_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub async fn sprint(sources: Vec<Source>) -> Sprint {
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for source in sources.into_iter().filter(|s| jql_key(&s.key)) {
        by.entry(source.connection).or_default().push(source.key);
    }
    let mut result = Sprint {
        tickets: Vec::new(),
        sprints: Vec::new(),
        errors: Vec::new(),
    };
    for (connection, keys) in by {
        match sprint_of(&connection, &keys).await {
            Ok((tickets, sprints)) => {
                result.tickets.extend(tickets);
                for name in sprints {
                    if !result.sprints.contains(&name) {
                        result.sprints.push(name);
                    }
                }
            }
            Err(e) => result.errors.push(e),
        }
    }
    result
}

async fn sprint_of(
    connection: &str,
    keys: &[String],
) -> Result<(Vec<Summary>, Vec<String>), String> {
    let secret = secret(connection)?;
    let field = sprint_field(&secret).await;
    let me = request(&secret, Method::GET, "/rest/api/3/myself", &[], None)
        .await
        .map(|m| text(&m["accountId"]))
        .unwrap_or_default();
    let mut fields = vec![
        "summary",
        "status",
        "assignee",
        "issuetype",
        "priority",
        "parent",
    ];
    if let Some(field) = &field {
        fields.push(field);
    }
    let jql = format!(
        "project in ({}) AND sprint in openSprints() ORDER BY project ASC, rank ASC",
        keys.join(", ")
    );
    let mut tickets = Vec::new();
    let mut sprints = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let mut body = json!({ "jql": jql, "fields": fields, "maxResults": 100 });
        if let Some(token) = &token {
            body["nextPageToken"] = json!(token);
        }
        let page = request(
            &secret,
            Method::POST,
            "/rest/api/3/search/jql",
            &[],
            Some(body),
        )
        .await?;
        for issue in page["issues"].as_array().into_iter().flatten() {
            let f = &issue["fields"];
            if let Some(field) = &field {
                for s in f[field].as_array().into_iter().flatten() {
                    let name = text(&s["name"]);
                    if s["state"] == "active" && !name.is_empty() && !sprints.contains(&name) {
                        sprints.push(name);
                    }
                }
            }
            tickets.push(Summary {
                connection: connection.to_string(),
                key: text(&issue["key"]),
                title: text(&f["summary"]),
                status: text(&f["status"]["name"]),
                category: text(&f["status"]["statusCategory"]["key"]),
                kind: text(&f["issuetype"]["name"]),
                priority: text(&f["priority"]["name"]),
                urgency: urgency(&f["priority"]),
                epic: text(&f["parent"]["fields"]["summary"]),
                assignee: person(&f["assignee"]),
                mine: !me.is_empty() && text(&f["assignee"]["accountId"]) == me,
            });
        }
        token = page["nextPageToken"].as_str().map(str::to_string);
        if token.is_none() || tickets.len() >= 500 {
            return Ok((tickets, sprints));
        }
    }
}

fn link(issue: &Value, relation: String) -> Link {
    Link {
        relation,
        key: text(&issue["key"]),
        title: text(&issue["fields"]["summary"]),
        status: text(&issue["fields"]["status"]["name"]),
    }
}

fn names(list: &Value, field: &str) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .map(|v| text(&v[field]))
        .filter(|s| !s.is_empty())
        .collect()
}

fn custom_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(custom_value)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(o) if o.get("type").and_then(Value::as_str) == Some("doc") => {
            adf(value).trim().to_string()
        }
        Value::Object(o) => ["value", "name", "displayName", "key"]
            .iter()
            .find_map(|k| o.get(*k).and_then(Value::as_str))
            .unwrap_or_default()
            .to_string(),
    }
}

pub async fn ticket(connection: &str, key: &str) -> Result<Ticket, String> {
    let secret = secret(connection)?;
    let issue = request(
        &secret,
        Method::GET,
        &format!("/rest/api/3/issue/{key}"),
        &[("expand", "names")],
        None,
    )
    .await?;
    let comments = all_comments(&secret, key).await?;
    let f = &issue["fields"];
    let names_map = issue["names"].as_object().cloned().unwrap_or_default();
    let mut sprint = String::new();
    let mut custom = Vec::new();
    for (id, value) in f.as_object().into_iter().flatten() {
        if !id.starts_with("customfield_") {
            continue;
        }
        let name = names_map.get(id).map(text).unwrap_or_else(|| id.clone());
        if name == "Sprint" {
            sprint = value
                .as_array()
                .into_iter()
                .flatten()
                .rev()
                .map(|s| text(&s["name"]))
                .next()
                .unwrap_or_default();
            continue;
        }
        if ["Rank", "Development", "Team", "Parent Link", "Flagged"].contains(&name.as_str()) {
            continue;
        }
        let value = custom_value(value);
        if !value.is_empty() && !value.starts_with('{') && value.len() < 4000 {
            custom.push(Field { name, value });
        }
    }
    custom.sort_by(|a, b| a.name.cmp(&b.name));
    let mut links = Vec::new();
    for l in f["issuelinks"].as_array().into_iter().flatten() {
        if l["outwardIssue"].is_object() {
            links.push(link(&l["outwardIssue"], text(&l["type"]["outward"])));
        }
        if l["inwardIssue"].is_object() {
            links.push(link(&l["inwardIssue"], text(&l["type"]["inward"])));
        }
    }
    Ok(Ticket {
        connection: connection.to_string(),
        key: text(&issue["key"]),
        url: format!("https://{}/browse/{}", secret.site, text(&issue["key"])),
        title: text(&f["summary"]),
        kind: text(&f["issuetype"]["name"]),
        status: text(&f["status"]["name"]),
        category: text(&f["status"]["statusCategory"]["key"]),
        priority: text(&f["priority"]["name"]),
        urgency: urgency(&f["priority"]),
        assignee: person(&f["assignee"]),
        reporter: person(&f["reporter"]),
        labels: f["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .map(text)
            .collect(),
        components: names(&f["components"], "name"),
        versions: names(&f["fixVersions"], "name"),
        epic: f["parent"].is_object().then(|| {
            link(
                &f["parent"],
                text(&f["parent"]["fields"]["issuetype"]["name"]),
            )
        }),
        sprint,
        updated: text(&f["updated"]),
        description: adf(&f["description"]).trim().to_string(),
        subtasks: f["subtasks"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| link(s, String::new()))
            .collect(),
        links,
        attachments: names(&f["attachment"], "filename"),
        comments,
        custom,
    })
}

async fn all_comments(secret: &Secret, key: &str) -> Result<Vec<Comment>, String> {
    let mut found = Vec::new();
    loop {
        let at = found.len().to_string();
        let page = request(
            secret,
            Method::GET,
            &format!("/rest/api/3/issue/{key}/comment"),
            &[
                ("startAt", &at),
                ("maxResults", "100"),
                ("orderBy", "created"),
            ],
            None,
        )
        .await?;
        let list = page["comments"].as_array().cloned().unwrap_or_default();
        found.extend(list.iter().map(|c| Comment {
            id: custom_value(&c["id"]),
            parent: Some(custom_value(&c["parentId"])).filter(|p| !p.is_empty()),
            author: person(&c["author"]).unwrap_or(Person {
                name: String::new(),
                avatar: String::new(),
            }),
            created: text(&c["created"]),
            body: adf(&c["body"]).trim().to_string(),
        }));
        let total = page["total"].as_u64().unwrap_or(0) as usize;
        if list.is_empty() || found.len() >= total || found.len() >= 1000 {
            return Ok(found);
        }
    }
}

pub fn adf(node: &Value) -> String {
    let mut out = String::new();
    block(node, &mut out, "");
    out
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node["content"].as_array().into_iter().flatten()
}

fn inline(node: &Value) -> String {
    children(node).map(span).collect()
}

fn span(node: &Value) -> String {
    let attrs = &node["attrs"];
    match node["type"].as_str().unwrap_or_default() {
        "text" => {
            let mut s = text(&node["text"]);
            for mark in node["marks"].as_array().into_iter().flatten() {
                s = match mark["type"].as_str().unwrap_or_default() {
                    "strong" => format!("**{s}**"),
                    "em" => format!("*{s}*"),
                    "strike" => format!("~~{s}~~"),
                    "code" => format!("`{s}`"),
                    "link" => format!("[{s}]({})", text(&mark["attrs"]["href"])),
                    _ => s,
                };
            }
            s
        }
        "hardBreak" => "  \n".into(),
        "mention" => {
            let name = text(&attrs["text"]);
            if name.starts_with('@') {
                name
            } else {
                format!("@{name}")
            }
        }
        "emoji" => {
            let t = text(&attrs["text"]);
            if t.is_empty() {
                text(&attrs["shortName"])
            } else {
                t
            }
        }
        "inlineCard" | "blockCard" | "embedCard" => text(&attrs["url"]),
        "status" => format!("[{}]", text(&attrs["text"])),
        "date" => attrs["timestamp"]
            .as_str()
            .and_then(|t| t.parse::<i64>().ok())
            .map(|ms| day(ms.div_euclid(86_400_000)))
            .unwrap_or_default(),
        "media" | "mediaInline" => {
            let alt = text(&attrs["alt"]);
            format!(
                "[{}]",
                if alt.is_empty() {
                    "attachment".into()
                } else {
                    alt
                }
            )
        }
        _ => inline(node),
    }
}

fn day(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn block(node: &Value, out: &mut String, indent: &str) {
    let push = |out: &mut String, s: &str| {
        for (i, line) in s.lines().enumerate() {
            if i > 0 || out.ends_with('\n') || out.is_empty() {
                out.push_str(indent);
            }
            out.push_str(line);
            out.push('\n');
        }
    };
    match node["type"].as_str().unwrap_or_default() {
        "doc" | "panel" | "layoutSection" | "layoutColumn" | "expand" | "nestedExpand" => {
            if node["type"] == "expand" || node["type"] == "nestedExpand" {
                let title = text(&node["attrs"]["title"]);
                if !title.is_empty() {
                    push(out, &format!("**{title}**"));
                    out.push('\n');
                }
            }
            for child in children(node) {
                block(child, out, indent);
            }
        }
        "paragraph" => {
            push(out, &inline(node));
            out.push('\n');
        }
        "heading" => {
            let level = node["attrs"]["level"].as_u64().unwrap_or(2).clamp(1, 6) as usize;
            push(out, &format!("{} {}", "#".repeat(level), inline(node)));
            out.push('\n');
        }
        "bulletList" | "orderedList" | "taskList" => {
            let ordered = node["type"] == "orderedList";
            for (i, item) in children(node).enumerate() {
                let marker = if ordered {
                    format!("{}. ", i + 1)
                } else if item["type"] == "taskItem" {
                    if item["attrs"]["state"] == "DONE" {
                        "- [x] ".into()
                    } else {
                        "- [ ] ".into()
                    }
                } else {
                    "- ".into()
                };
                let mut inner = String::new();
                if item["type"] == "taskItem" {
                    inner.push_str(&inline(item));
                } else {
                    for child in children(item) {
                        block(child, &mut inner, "");
                    }
                }
                let inner = inner.trim_end();
                let pad = " ".repeat(marker.len());
                for (j, line) in inner.lines().filter(|l| !l.is_empty()).enumerate() {
                    out.push_str(indent);
                    out.push_str(if j == 0 { &marker } else { &pad });
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out.push('\n');
        }
        "codeBlock" => {
            let language = text(&node["attrs"]["language"]);
            push(out, &format!("```{language}"));
            let code: String = children(node).map(|c| text(&c["text"])).collect();
            push(out, &code);
            push(out, "```");
            out.push('\n');
        }
        "blockquote" => {
            let mut inner = String::new();
            for child in children(node) {
                block(child, &mut inner, "");
            }
            for line in inner.trim_end().lines() {
                push(out, &format!("> {line}"));
            }
            out.push('\n');
        }
        "rule" => {
            push(out, "---");
            out.push('\n');
        }
        "table" => {
            let rows: Vec<Vec<String>> = children(node)
                .map(|row| {
                    children(row)
                        .map(|cell| {
                            let mut inner = String::new();
                            for child in children(cell) {
                                block(child, &mut inner, "");
                            }
                            inner
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .replace('|', "\\|")
                        })
                        .collect()
                })
                .collect();
            for (i, row) in rows.iter().enumerate() {
                push(out, &format!("| {} |", row.join(" | ")));
                if i == 0 {
                    push(out, &format!("|{}", " --- |".repeat(row.len())));
                }
            }
            out.push('\n');
        }
        "mediaSingle" | "mediaGroup" => {
            push(out, &inline(node));
            out.push('\n');
        }
        _ => {
            let s = span(node);
            if !s.is_empty() {
                push(out, &s);
                out.push('\n');
            }
        }
    }
}

pub fn context(ticket: &Ticket, fields: &[String]) -> String {
    let on = |f: &str| fields.iter().any(|x| x == f);
    let mut out = format!(
        "<ticket source=\"jira\" key=\"{}\" url=\"{}\">\n# {} {}\n",
        ticket.key, ticket.url, ticket.key, ticket.title
    );
    let mut facts = Vec::new();
    if on("type") {
        facts.push(format!("Type: {}", ticket.kind));
        if !ticket.priority.is_empty() {
            facts.push(format!("Priority: {}", ticket.priority));
        }
    }
    if on("status") {
        facts.push(format!("Status: {}", ticket.status));
    }
    if on("people") {
        if let Some(p) = &ticket.assignee {
            facts.push(format!("Assignee: {}", p.name));
        }
        if let Some(p) = &ticket.reporter {
            facts.push(format!("Reporter: {}", p.name));
        }
    }
    if on("labels") && !ticket.labels.is_empty() {
        facts.push(format!("Labels: {}", ticket.labels.join(", ")));
    }
    if on("components") && !ticket.components.is_empty() {
        facts.push(format!("Components: {}", ticket.components.join(", ")));
    }
    if on("versions") && !ticket.versions.is_empty() {
        facts.push(format!("Fix versions: {}", ticket.versions.join(", ")));
    }
    if let Some(e) = ticket.epic.as_ref().filter(|_| on("epic")) {
        facts.push(format!("Parent: {} {}", e.key, e.title));
    }
    if on("custom") {
        for f in &ticket.custom {
            facts.push(format!("{}: {}", f.name, f.value));
        }
    }
    for fact in facts {
        out.push_str(&format!("- {fact}\n"));
    }
    if on("description") && !ticket.description.is_empty() {
        out.push_str(&format!("\n## Description\n{}\n", ticket.description));
    }
    if on("subtasks") && !ticket.subtasks.is_empty() {
        out.push_str("\n## Subtasks\n");
        for s in &ticket.subtasks {
            out.push_str(&format!("- {} {} ({})\n", s.key, s.title, s.status));
        }
    }
    if on("links") && !ticket.links.is_empty() {
        out.push_str("\n## Linked tickets\n");
        for l in &ticket.links {
            out.push_str(&format!(
                "- {} {} {} ({})\n",
                l.relation, l.key, l.title, l.status
            ));
        }
    }
    if on("attachments") && !ticket.attachments.is_empty() {
        out.push_str(&format!(
            "\n## Attachments\n{}\n",
            ticket.attachments.join(", ")
        ));
    }
    if on("comments") && !ticket.comments.is_empty() {
        out.push_str("\n## Comments\n");
        for c in ticket.comments.iter().filter(|c| c.parent.is_none()) {
            comment(&mut out, ticket, c, "");
        }
    }
    out.push_str("</ticket>");
    out
}

fn comment(out: &mut String, ticket: &Ticket, c: &Comment, indent: &str) {
    let date = c.created.get(..10).unwrap_or_default();
    let body = c
        .body
        .lines()
        .collect::<Vec<_>>()
        .join(&format!("\n{indent}  "));
    out.push_str(&format!("{indent}- {} ({date}): {body}\n", c.author.name));
    for reply in ticket
        .comments
        .iter()
        .filter(|r| r.parent.as_deref() == Some(c.id.as_str()))
    {
        comment(out, ticket, reply, &format!("{indent}  "));
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Brief {
    pub prompt: String,
    pub goal: String,
}

const BRIEF: &str = "You turn a Jira ticket into the instruction a coding agent receives in this repository. Write in the language the ticket is written in. Reply with JSON only, no code fence: {\"prompt\": \"...\", \"goal\": \"...\"}. prompt: what to do, in a short paragraph then a dash list of the concrete requirements, constraints and decisions taken in the comments (latest decision wins). Mention the ticket key once. No preamble, no restating the title. goal: one sentence saying how to check the work is finished.";

pub fn brief(program: &str, dir: &Path, context: &str) -> Result<Brief, String> {
    let dir = if dir.is_dir() { dir } else { Path::new("/") };
    let mut child = crate::process::command(program)
        .current_dir(dir)
        .args([
            "-p",
            "--model",
            "haiku",
            "--output-format",
            "json",
            "--tools",
            "",
            "--strict-mcp-config",
            "--no-session-persistence",
            "--append-system-prompt",
            BRIEF,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Claude Code could not start: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("Claude Code could not start.")?
        .write_all(context.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let reply: Value = serde_json::from_slice(&output.stdout).map_err(|_| {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.is_empty() {
            "Haiku did not answer.".to_string()
        } else {
            err
        }
    })?;
    parse_brief(&text(&reply["result"])).ok_or_else(|| "Haiku did not answer.".into())
}

fn parse_brief(result: &str) -> Option<Brief> {
    let start = result.find('{')?;
    let end = result.rfind('}')?;
    let brief: Brief = serde_json::from_str(result.get(start..=end)?).ok()?;
    (!brief.prompt.trim().is_empty()).then_some(Brief {
        prompt: brief.prompt.trim().to_string(),
        goal: brief.goal.trim().to_string(),
    })
}

pub fn mentions(dir: &Path) -> BTreeMap<String, (u32, u32)> {
    let run = |args: &[&str]| {
        crate::process::command("git")
            .current_dir(dir)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    };
    let mut found: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for line in run(&["log", "-200", "--format=%s %b%x00"]).split('\0') {
        for key in projects_in(line) {
            found.entry(key).or_default().0 += 1;
        }
    }
    for line in run(&["branch", "-a", "--format=%(refname:short)"]).lines() {
        for key in projects_in(&line.to_uppercase()) {
            found.entry(key).or_default().1 += 1;
        }
    }
    found
}

fn projects_in(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i].is_ascii_uppercase() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            while i < bytes.len()
                && (bytes[i].is_ascii_uppercase() || bytes[i].is_ascii_digit() || bytes[i] == b'_')
            {
                i += 1;
            }
            if i + 1 < bytes.len()
                && bytes[i] == b'-'
                && bytes[i + 1].is_ascii_digit()
                && i - start >= 2
            {
                let key = text[start..i].to_string();
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        i += 1;
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_keeps_the_host_only() {
        assert_eq!(
            site("https://Akki.atlassian.net/jira/software/"),
            "akki.atlassian.net"
        );
        assert_eq!(site(" akki.atlassian.net "), "akki.atlassian.net");
    }

    #[test]
    fn finds_project_keys() {
        assert_eq!(
            projects_in("RING-482 fix, see TMA-12 and RING-3"),
            ["RING", "TMA"]
        );
        assert!(
            projects_in("UTF-8 is not a key, nor A-1 nor X-Y")
                .iter()
                .all(|k| k == "UTF")
        );
        assert_eq!(projects_in("FEAT/RING-12-EXPORT"), ["RING"]);
    }

    #[test]
    fn converts_adf_to_markdown() {
        let doc = json!({"type": "doc", "content": [
            {"type": "heading", "attrs": {"level": 2}, "content": [{"type": "text", "text": "Goal"}]},
            {"type": "paragraph", "content": [
                {"type": "text", "text": "Use "},
                {"type": "text", "text": "Account::timezone", "marks": [{"type": "code"}]},
                {"type": "text", "text": ", ask "},
                {"type": "mention", "attrs": {"text": "@Marie"}}
            ]},
            {"type": "bulletList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "one"}]}]},
                {"type": "listItem", "content": [
                    {"type": "paragraph", "content": [{"type": "text", "text": "two"}]},
                    {"type": "bulletList", "content": [{"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "deep"}]}]}]}
                ]}
            ]},
            {"type": "codeBlock", "attrs": {"language": "php"}, "content": [{"type": "text", "text": "echo 1;"}]}
        ]});
        assert_eq!(
            adf(&doc).trim(),
            "## Goal\n\nUse `Account::timezone`, ask @Marie\n\n- one\n- two\n  - deep\n\n```php\necho 1;\n```"
        );
    }

    #[test]
    fn dates_read_as_days() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(20_385), "2025-10-24");
    }

    #[test]
    fn reads_the_brief_even_inside_a_fence() {
        let brief =
            parse_brief("```json\n{\"prompt\": \"Do it\", \"goal\": \"Tests pass\"}\n```").unwrap();
        assert_eq!(brief.prompt, "Do it");
        assert_eq!(brief.goal, "Tests pass");
        assert!(parse_brief("no json").is_none());
    }
}
