use squadron_extension_api::forge::{self, Input, NoArgs, Person};
use squadron_extension_api::{Agent, Extension, host, squadron_extension};

struct Forge;

fn number(key: &str) -> u64 {
    host::state_get(key)
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn bump(key: &str) -> u64 {
    let next = number(key) + 1;
    host::state_set(key, Some(&next.to_string()));
    next
}

impl Extension for Forge {
    fn call_tool(name: String, _input: String, _agent: Option<Agent>) -> Result<String, String> {
        Ok(number(&name).to_string())
    }

    fn forge(call: String, input: String) -> Result<String, String> {
        match call.as_str() {
            "wait" => {
                let ms: u64 = input.trim().parse().unwrap_or(0);
                if ms == 400 {
                    bump("waits");
                }
                host::state_set(&format!("wait-{ms}"), Some(&ms.to_string()));
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(ms.to_string())
            }
            "bump" => Ok(bump("bumps").to_string()),
            "git" => host::exec("git", &["--version".into()], None).map(|out| out.stdout),
            "trap" => panic!("trapped"),
            "me" => {
                let asked: Input<NoArgs> = forge::input(&input)?;
                forge::answer(&Person {
                    name: format!("{} on {}", asked.repo, asked.host),
                    avatar: String::new(),
                })
            }
            _ => {
                bump("unsupported");
                Err(forge::unsupported(&call))
            }
        }
    }
}

squadron_extension!(Forge);
