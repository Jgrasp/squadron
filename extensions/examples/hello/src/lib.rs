use squadron_extension_api::{Account, Agent, Extension, Request, host, squadron_extension};

struct Hello;

fn count() -> u32 {
    host::state_get("count").and_then(|c| c.parse().ok()).unwrap_or(0)
}

fn show(count: u32) {
    host::set_badge("count", Some(&count.to_string()));
    host::panel_post("main", &format!(r#"{{"count":{count}}}"#));
}

fn option(name: &str) -> Option<String> {
    let options = host::options();
    let rest = &options[options.find(&format!(r#""{name}":""#))? + name.len() + 4..];
    Some(rest[..rest.find('"')?].to_string())
}

fn reset() {
    host::state_set("count", None);
    show(0);
    host::settings_post(r#"{"count":0}"#);
}

fn add() {
    let next = count() + 1;
    host::state_set("count", Some(&next.to_string()));
    show(next);
}

fn draft_of(data: &str) -> String {
    let rest = &data[data.find(r#""draft":""#).map_or(data.len(), |at| at + 9)..];
    rest[..rest.find('"').unwrap_or(0)].to_string()
}

fn brief(draft: &str, short: bool) {
    let text = if short { "Say hello." } else { "Say hello, then list the files you would read first." };
    host::update_new_agent(draft, &format!(r#"{{"text":"{text}","goal":"The agent said hello","status":{{"state":"done","text":"Written by Hello"}}}}"#));
}

fn field(account: &Account, name: &str) -> String {
    account.fields.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()).unwrap_or_default()
}

fn quoted(text: &str) -> String {
    let mut out = String::from('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn value<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let rest = body[body.find(&format!(r#""{key}":"#))? + key.len() + 3..].trim_start_matches('"');
    Some(&rest[..rest.find(['"', ',', '}'])?])
}

impl Extension for Hello {
    fn on_event(name: String, agent: Option<Agent>, data: String) {
        match (name.as_str(), agent) {
            ("started", _) => host::set_badge("count", Some(&count().to_string())),
            ("panel.message", _) if data.contains(r#""type":"count""#) => add(),
            ("panel.message", _) => show(count()),
            ("settings.message", _) if data.contains(r#""type":"reset""#) => reset(),
            ("settings.message", _) => host::settings_post(&format!(r#"{{"count":{}}}"#, count())),
            ("agent.done", Some(agent)) => host::notify("Hello", &format!("{} finished its turn.", agent.name)),
            ("new-agent.regenerate", _) => brief(&draft_of(&data), data.contains(r#""short""#)),
            ("new-agent.started", Some(agent)) => host::notify("Hello", &format!("{} started from the Hello draft.", agent.name)),
            _ => {}
        }
    }

    fn run(action: String, agent: Option<Agent>) -> Result<(), String> {
        match action.as_str() {
            "count" => {
                add();
                Ok(())
            }
            "greet" => {
                let agent = agent.ok_or("No agent to greet.")?;
                let greeting = option("greeting").unwrap_or_else(|| "Hello".into());
                let end = if option("tone").as_deref() == Some("warm") { " 🙂" } else { "" };
                host::insert_prompt(&agent.id, &format!("{greeting} {}, what are you working on?{end}", agent.name));
                Ok(())
            }
            "draft" => {
                let agent = agent.ok_or("No agent to start from.")?;
                let draft = host::open_new_agent(
                    &agent.id,
                    r#"{"name":"Hello task","link":{"key":"HELLO-1","title":"Say hello"},"choices":{"label":"Context","items":[{"id":"short","label":"Short","on":false}]},"status":{"state":"running","text":"Hello is writing the first message…"}}"#,
                );
                brief(&draft, false);
                Ok(())
            }
            _ => Err(format!("Unknown action {action}.")),
        }
    }

    fn call_tool(name: String, _input: String, agent: Option<Agent>) -> Result<String, String> {
        match name.as_str() {
            "changed_files" => {
                let dir = agent.map(|a| a.dir);
                let out = host::exec("git", &["status".into(), "--short".into()], dir.as_deref())?;
                if out.code != 0 {
                    return Err(out.stderr);
                }
                Ok(if out.stdout.is_empty() { "No changed file.".into() } else { out.stdout })
            }
            "clicks" => Ok(format!("The Count button was clicked {} times.", count())),
            "options" => Ok(host::options()),
            "github_user" => {
                let answer = host::fetch(&Request {
                    method: "GET".into(),
                    url: "https://api.github.com/user".into(),
                    headers: vec![
                        ("Authorization".into(), "Bearer {{token}}".into()),
                        ("User-Agent".into(), "squadron-hello".into()),
                    ],
                    body: None,
                })?;
                if answer.status != 200 {
                    return Err(format!("GitHub answered {}: {}", answer.status, answer.body));
                }
                Ok(answer.body)
            }
            _ => Err(format!("Unknown tool {name}.")),
        }
    }

    fn check_account(account: Account) -> Result<String, String> {
        let api = field(&account, "api");
        let path = if api == "api.github.com" { "/user" } else { "/api/v3/user" };
        let answer = host::fetch(&Request {
            method: "GET".into(),
            url: format!("https://{api}{path}"),
            headers: vec![
                ("Authorization".into(), format!("Bearer {{{{{}.token}}}}", account.id)),
                ("User-Agent".into(), "squadron-hello".into()),
            ],
            body: None,
        })
        .map_err(|e| format!(r#"{{"title":{{"en":"Could not reach {api}","fr":"{api} ne répond pas"}},"detail":{},"field":"api"}}"#, quoted(&e)))?;
        match answer.status {
            200 => {
                let login = value(&answer.body, "login").unwrap_or("?");
                let repos = value(&answer.body, "public_repos").unwrap_or("0");
                Ok(format!(r#"{{"text":{{"en":"Connected as {login}","fr":"Connecté en tant que {login}"}},"detail":{{"en":"{repos} public repositories","fr":"{repos} dépôts publics"}}}}"#))
            }
            401 => Err(r#"{"title":{"en":"GitHub refused this token","fr":"GitHub refuse ce jeton"},"detail":{"en":"It may be wrong, expired or revoked: create a new one and paste it here.","fr":"Il est peut-être faux, expiré ou révoqué : créez-en un nouveau et collez-le ici."},"field":"token"}"#.into()),
            status => Err(format!(r#"{{"title":{{"en":"GitHub answered {status}","fr":"GitHub a répondu {status}"}}}}"#)),
        }
    }
}

squadron_extension!(Hello);
