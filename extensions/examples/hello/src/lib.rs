use squadron_extension_api::{Agent, Extension, Request, host, squadron_extension};

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

impl Extension for Hello {
    fn on_event(name: String, agent: Option<Agent>, data: String) {
        match (name.as_str(), agent) {
            ("started", _) => host::set_badge("count", Some(&count().to_string())),
            ("panel.message", _) if data.contains(r#""type":"count""#) => add(),
            ("panel.message", _) => show(count()),
            ("settings.message", _) if data.contains(r#""type":"reset""#) => reset(),
            ("settings.message", _) => host::settings_post(&format!(r#"{{"count":{}}}"#, count())),
            ("agent.done", Some(agent)) => host::notify("Hello", &format!("{} finished its turn.", agent.name)),
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
}

squadron_extension!(Hello);
