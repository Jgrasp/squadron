use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Gate,
    Pilot,
    Extensions,
    Askpass,
}

#[derive(Default)]
pub struct Keys {
    issued: Mutex<HashMap<(String, Route), String>>,
    owners: Mutex<HashMap<String, (String, Route)>>,
    pilot: AtomicBool,
}

pub static KEYS: LazyLock<Keys> = LazyLock::new(Keys::default);

impl Keys {
    pub fn issue(&self, agent: &str, route: Route) -> String {
        let mut issued = self.issued.lock().unwrap();
        issued
            .entry((agent.to_string(), route))
            .or_insert_with(|| {
                let token = uuid::Uuid::new_v4().simple().to_string();
                self.owners
                    .lock()
                    .unwrap()
                    .insert(token.clone(), (agent.to_string(), route));
                token
            })
            .clone()
    }

    pub fn owner(&self, token: &str, route: Route) -> Option<String> {
        let owners = self.owners.lock().unwrap();
        owners
            .get(token)
            .filter(|(_, r)| *r == route)
            .map(|(agent, _)| agent.clone())
    }

    pub fn revoke(&self, agent: &str) {
        self.issued.lock().unwrap().retain(|(a, _), _| a != agent);
        self.owners.lock().unwrap().retain(|_, (a, _)| a != agent);
    }

    pub fn set_pilot(&self, on: bool) {
        self.pilot.store(on, Ordering::Relaxed);
    }
}

pub fn guarded(router: axum::Router, route: Route) -> axum::Router {
    router.layer(axum::middleware::from_fn_with_state(route, guard))
}

async fn guard(State(route): State<Route>, mut request: Request, next: Next) -> Response {
    let token = request
        .headers()
        .get("x-squadron-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let Some(agent) = KEYS.owner(token, route) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if route == Route::Pilot && !KEYS.pilot.load(Ordering::Relaxed) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(agent) = HeaderValue::from_str(&agent) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    request.headers_mut().insert("x-squadron-agent", agent);
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn get(port: u16, path: &str, token: &str, agent: &str) -> String {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Squadron-Token: {token}\r\nX-Squadron-Agent: {agent}\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response.split(' ').nth(1).unwrap_or_default().to_string();
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, b)| b)
            .unwrap_or_default();
        format!("{status} {body}")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_guard_names_the_agent_and_closes_the_pilot_when_it_is_off() {
        let who = || {
            axum::Router::new().route(
                "/",
                axum::routing::get(|headers: axum::http::HeaderMap| async move {
                    headers["x-squadron-agent"].to_str().unwrap().to_string()
                }),
            )
        };
        let router = axum::Router::new()
            .merge(guarded(
                axum::Router::new().nest("/mcp", who()),
                Route::Gate,
            ))
            .merge(guarded(
                axum::Router::new().nest("/pilot", who()),
                Route::Pilot,
            ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, router).await });
        let gate = KEYS.issue("guard-1", Route::Gate);
        let pilot = KEYS.issue("guard-1", Route::Pilot);
        let answers = tokio::task::spawn_blocking(move || {
            KEYS.set_pilot(false);
            let closed = get(port, "/pilot", &pilot, "guard-1");
            KEYS.set_pilot(true);
            (
                get(port, "/mcp", &gate, "guard-2"),
                get(port, "/mcp", "", "guard-1"),
                get(port, "/mcp", &pilot, "guard-1"),
                closed,
                get(port, "/pilot", &pilot, "guard-1"),
            )
        })
        .await
        .unwrap();
        assert_eq!(answers.0, "200 guard-1");
        assert_eq!(answers.1, "401 ");
        assert_eq!(answers.2, "401 ");
        assert_eq!(answers.3, "403 ");
        assert_eq!(answers.4, "200 guard-1");
    }

    #[test]
    fn a_key_opens_one_route_for_one_agent() {
        let keys = Keys::default();
        let gate = keys.issue("agent-1", Route::Gate);
        assert_eq!(keys.issue("agent-1", Route::Gate), gate);
        assert_ne!(keys.issue("agent-1", Route::Pilot), gate);
        assert_ne!(keys.issue("agent-2", Route::Gate), gate);
        assert_eq!(keys.owner(&gate, Route::Gate).as_deref(), Some("agent-1"));
        assert_eq!(keys.owner(&gate, Route::Pilot), None);
        assert_eq!(keys.owner("", Route::Gate), None);
        keys.revoke("agent-1");
        assert_eq!(keys.owner(&gate, Route::Gate), None);
        assert!(
            keys.owner(&keys.issue("agent-2", Route::Gate), Route::Gate)
                .is_some()
        );
    }
}
