use squadron_mod_api::{Agent, Mod, host, squadron_mod};

struct Hello;

fn count() -> u32 {
    host::state_get("count").and_then(|c| c.parse().ok()).unwrap_or(0)
}

impl Mod for Hello {
    fn on_event(name: String, agent: Option<Agent>, _data: String) {
        match (name.as_str(), agent) {
            ("started", _) => host::set_badge("count", Some(&count().to_string())),
            ("agent.done", Some(agent)) => host::notify("Hello", &format!("{} finished its turn.", agent.name)),
            _ => {}
        }
    }

    fn run(action: String, agent: Option<Agent>) -> Result<(), String> {
        match action.as_str() {
            "count" => {
                let next = count() + 1;
                host::state_set("count", Some(&next.to_string()));
                host::set_badge("count", Some(&next.to_string()));
                Ok(())
            }
            "greet" => {
                let agent = agent.ok_or("No agent to greet.")?;
                host::insert_prompt(&agent.id, &format!("Hello {}, what are you working on?", agent.name));
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
            _ => Err(format!("Unknown tool {name}.")),
        }
    }
}

squadron_mod!(Hello);
