use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Url, WebviewBuilder, WebviewUrl};

const PAGE: &str = include_str!("browser-page.js");
const SIZE: (f64, f64) = (1280.0, 800.0);
const AWAY: f64 = -20000.0;
const CONSOLE: &str = "(() => { const key = Symbol.for('squadron.console'); if (window[key]) return; const kept = []; Object.defineProperty(window, key, { value: kept }); const plain = (a) => { const p = Object.getPrototypeOf(a); return p === Object.prototype || p === Array.prototype || p === null }; const text = (a) => { try { if (a instanceof Error) return a.stack || String(a); if (a === null || typeof a !== 'object') return String(a); return plain(a) ? JSON.stringify(a) : Object.prototype.toString.call(a) } catch { return '?' } }; const add = (level, args) => { try { if (kept.length >= 200) kept.shift(); kept.push([level, Array.from(args, text).join(' ').slice(0, 600)]) } catch {} }; for (const level of ['error', 'warn']) { const f = console[level]; console[level] = function (...args) { add(level, args); return f.apply(this, args) } } addEventListener('error', (e) => { if (e instanceof ErrorEvent) add('error', [e.error || e.message]) }, true); addEventListener('unhandledrejection', (e) => add('error', ['Unhandled rejection:', e.reason])) })()";
const LOGS: usize = 300;
const TALL: i32 = 16384;
const TALL_INLINE: f64 = 2000.0;
const NATIVE_DIALOGS: &str = "for (const name of ['alert', 'confirm']) window[name] = function () { const f = document.createElement('iframe'); f.style.display = 'none'; document.documentElement.appendChild(f); try { return f.contentWindow[name](...arguments) } finally { f.remove() } }";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub id: String,
    pub agent: String,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub dialog: Option<Value>,
    pub width: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub tabs: Vec<TabInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    agent: String,
    url: String,
    title: String,
    session: String,
}

struct Tab {
    info: TabInfo,
    view: tauri::Webview,
    session: String,
    refs: u64,
    last: Vec<String>,
    starts: u64,
    loads: u64,
    failed: Option<String>,
    logs: Vec<Log>,
    failures: u64,
    shown: f64,
    zoom: f64,
}

pub struct Log {
    pub failed: bool,
    pub line: String,
}

#[derive(Default)]
struct State {
    tabs: Vec<Tab>,
    next: u64,
}

pub(crate) struct Inner {
    app: Option<AppHandle>,
    dir: PathBuf,
    blocked: Vec<String>,
    state: Mutex<State>,
    saved: Mutex<Vec<Saved>>,
    notify: Box<dyn Fn(Status) + Send + Sync>,
}

#[derive(Clone)]
pub struct Browser(Arc<Inner>);

pub struct Image {
    pub bytes: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub cut: bool,
}

pub struct Handle {
    pub agent: String,
    pub url: String,
    pub loading: bool,
    pub dialog: Option<Value>,
}

fn log(t: &mut Tab, failed: bool, line: String) {
    if t.logs.len() >= LOGS {
        t.logs.remove(0);
    }
    t.failures += failed as u64;
    t.logs.push(Log { failed, line });
}

pub fn bare(url: &str) -> Option<String> {
    let mut url = Url::parse(url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    let _ = url.set_username("");
    let _ = url.set_password(None);
    Some(url.into())
}

fn blank(url: &str) -> bool {
    url.is_empty() || url == "about:blank"
}

pub fn address(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return "about:blank".into();
    }
    if input.contains("://") || input.starts_with("about:") || input.starts_with("data:") {
        return input.into();
    }
    let host = input.split(['/', '?', '#']).next().unwrap_or_default();
    let local = host.starts_with("localhost")
        || host.starts_with("127.")
        || host.starts_with("[::1]")
        || host.ends_with(".local")
        || host.ends_with(".test")
        || host.ends_with(".localhost");
    if !input.contains(' ') && (host.contains('.') || host.contains(':') || local) {
        return format!("{}://{input}", if local { "http" } else { "https" });
    }
    let query: String = input
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("https://www.google.com/search?q={query}")
}

pub fn tab(label: &str) -> bool {
    label.starts_with("tab-")
}

pub fn safe(url: &str, blocked: &[String]) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    match url.scheme() {
        "about" | "data" | "blob" => true,
        "http" | "https" => !blocked.contains(&url.origin().ascii_serialization()),
        _ => false,
    }
}

impl Browser {
    pub fn new(
        app: AppHandle,
        dir: PathBuf,
        notify: impl Fn(Status) + Send + Sync + 'static,
    ) -> Self {
        let mut blocked: Vec<String> = [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
        ]
        .map(String::from)
        .to_vec();
        if tauri::is_dev()
            && let Some(dev) = &app.config().build.dev_url
        {
            blocked.push(dev.origin().ascii_serialization());
        }
        let saved = std::fs::read(dir.join("tabs.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self(Arc::new(Inner {
            app: Some(app),
            dir,
            blocked,
            state: Default::default(),
            saved: Mutex::new(saved),
            notify: Box::new(notify),
        }))
    }

    #[cfg(test)]
    pub fn detached() -> Self {
        Self(Arc::new(Inner {
            app: None,
            dir: PathBuf::new(),
            blocked: Vec::new(),
            state: Default::default(),
            saved: Default::default(),
            notify: Box::new(|_| {}),
        }))
    }

    pub fn status(&self) -> Status {
        let st = self.0.state.lock().unwrap();
        Status {
            tabs: st.tabs.iter().map(|t| t.info.clone()).collect(),
        }
    }

    pub fn tabs(&self, agent: &str) -> Vec<TabInfo> {
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .filter(|t| t.info.agent == agent)
            .map(|t| t.info.clone())
            .collect()
    }

    fn changed(&self) {
        (self.0.notify)(self.status());
    }

    fn save(&self) {
        if self.0.app.is_none() {
            return;
        }
        let mut all: Vec<Saved> = {
            let st = self.0.state.lock().unwrap();
            st.tabs
                .iter()
                .filter(|t| !blank(&t.info.url))
                .map(|t| Saved {
                    agent: t.info.agent.clone(),
                    url: t.info.url.clone(),
                    title: t.info.title.clone(),
                    session: t.session.clone(),
                })
                .collect()
        };
        all.extend(self.0.saved.lock().unwrap().iter().cloned());
        let path = self.0.dir.join("tabs.json");
        let tmp = path.with_extension("json.tmp");
        let written = serde_json::to_vec(&all)
            .map_err(|e| e.to_string())
            .and_then(|b| std::fs::write(&tmp, b).map_err(|e| e.to_string()))
            .and_then(|_| std::fs::rename(&tmp, &path).map_err(|e| e.to_string()));
        if let Err(e) = written {
            log::warn!("browser: saving tabs: {e}");
        }
    }

    pub(crate) fn kept(&self, tab: &str, session: &[u8]) {
        let session = base64::engine::general_purpose::STANDARD.encode(session);
        self.update(tab, |t| {
            t.session = session;
            false
        });
        self.save();
    }

    pub async fn restore(&self, agent: &str) {
        let mine: Vec<Saved> = {
            let mut saved = self.0.saved.lock().unwrap();
            let (mine, rest) = saved.drain(..).partition(|s| s.agent == agent);
            *saved = rest;
            mine
        };
        for s in mine {
            if let Err(e) = self.reopen(s).await {
                log::warn!("browser: restoring a tab: {e}");
            }
        }
    }

    async fn reopen(&self, s: Saved) -> Result<(), String> {
        self.check(&s.url)?;
        let id = self.next();
        let view = self.create(&id).await?;
        let session = base64::engine::general_purpose::STANDARD
            .decode(&s.session)
            .unwrap_or_default();
        self.add(
            &id,
            &s.agent,
            view.clone(),
            s.url.clone(),
            s.title,
            s.session,
        );
        if session.is_empty() || !native::restore(&view, session).await? {
            view.navigate(Url::parse(&s.url).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn next(&self) -> String {
        let mut st = self.0.state.lock().unwrap();
        st.next += 1;
        format!("t{}", st.next)
    }

    fn add(
        &self,
        id: &str,
        agent: &str,
        view: tauri::Webview,
        url: String,
        title: String,
        session: String,
    ) {
        self.0.state.lock().unwrap().tabs.push(Tab {
            info: TabInfo {
                id: id.into(),
                agent: agent.into(),
                url,
                title,
                loading: false,
                dialog: None,
                width: None,
            },
            view,
            session,
            refs: 0,
            last: Vec::new(),
            starts: 0,
            loads: 0,
            failed: None,
            logs: Vec::new(),
            failures: 0,
            shown: SIZE.0,
            zoom: 1.0,
        });
        self.changed();
    }

    fn update(&self, tab: &str, f: impl FnOnce(&mut Tab) -> bool) {
        let changed = {
            let mut st = self.0.state.lock().unwrap();
            st.tabs.iter_mut().find(|t| t.info.id == tab).is_some_and(f)
        };
        if changed {
            self.changed();
        }
    }

    pub(crate) fn loading(&self, tab: &str, loading: bool) {
        self.update(tab, |t| {
            if loading {
                t.starts += 1;
                t.failed = None;
            } else {
                t.loads += 1;
            }
            t.info.loading = loading;
            true
        });
    }

    pub(crate) fn located(&self, tab: &str, url: String) {
        self.update(tab, |t| {
            t.info.url != url && {
                t.info.url = url;
                true
            }
        });
    }

    pub(crate) fn titled(&self, tab: &str, title: String) {
        self.update(tab, |t| {
            t.info.title != title && {
                t.info.title = title;
                true
            }
        });
    }

    pub(crate) fn failed(&self, tab: &str, error: String) {
        self.update(tab, |t| {
            t.failed = Some(error);
            false
        });
    }

    pub(crate) fn logged(&self, tab: &str, failed: bool, line: String) {
        self.update(tab, |t| {
            log(t, failed, line);
            false
        });
    }

    async fn pull(&self, tab: &str) {
        let Ok(Value::Array(found)) = self
            .eval_main(
                tab,
                "(() => { const k = window[Symbol.for('squadron.console')]; return k ? k.splice(0) : [] })()",
            )
            .await
        else {
            return;
        };
        self.update(tab, |t| {
            for entry in found {
                let level = entry[0].as_str().unwrap_or("error");
                let text = entry[1].as_str().unwrap_or_default();
                // Tauri also injects its plugins' scripts into tabs, where their calls are refused.
                if text.contains(" not allowed on window ") && text.contains(" webview \"tab-") {
                    continue;
                }
                log(t, level == "error", format!("console {level}: {text}"));
            }
            false
        });
    }

    pub async fn failures(&self, tab: &str) -> u64 {
        self.pull(tab).await;
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .find(|t| t.info.id == tab)
            .map_or(0, |t| t.failures)
    }

    pub async fn logs(&self, tab: &str) -> Vec<Log> {
        self.pull(tab).await;
        let mut st = self.0.state.lock().unwrap();
        st.tabs
            .iter_mut()
            .find(|t| t.info.id == tab)
            .map(|t| std::mem::take(&mut t.logs))
            .unwrap_or_default()
    }

    pub(crate) fn asked(&self, tab: &str, dialog: Option<Value>) {
        self.update(tab, |t| {
            t.info.dialog = dialog;
            true
        });
    }

    fn view(&self, tab: &str) -> Result<tauri::Webview, String> {
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .find(|t| t.info.id == tab)
            .map(|t| t.view.clone())
            .ok_or_else(|| format!("No tab {tab}."))
    }

    fn counters(&self, tab: &str) -> Option<(u64, u64, bool, bool)> {
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .find(|t| t.info.id == tab)
            .map(|t| (t.starts, t.loads, t.info.loading, t.info.dialog.is_some()))
    }

    pub fn handle(&self, tab: &str) -> Result<Handle, String> {
        let st = self.0.state.lock().unwrap();
        let t = st
            .tabs
            .iter()
            .find(|t| t.info.id == tab)
            .ok_or_else(|| format!("No tab {tab}."))?;
        Ok(Handle {
            agent: t.info.agent.clone(),
            url: t.info.url.clone(),
            loading: t.info.loading,
            dialog: t.info.dialog.clone(),
        })
    }

    fn check(&self, url: &str) -> Result<(), String> {
        if safe(url, &self.0.blocked) {
            Ok(())
        } else {
            Err(format!("The browser cannot open {url}."))
        }
    }

    pub fn open<'a>(
        &'a self,
        agent: &'a str,
        url: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let url = address(url);
            self.check(&url)?;
            let id = self.next();
            let view = self.create(&id).await?;
            self.add(
                &id,
                agent,
                view,
                "about:blank".into(),
                String::new(),
                String::new(),
            );
            if !blank(&url) {
                self.navigate(&id, &url)
                    .await
                    .map_err(|e| format!("{e} (tab {id} is open)"))?;
            }
            Ok(id)
        })
    }

    async fn create(&self, id: &str) -> Result<tauri::Webview, String> {
        let window = self
            .0
            .app
            .as_ref()
            .and_then(|app| app.get_window("main"))
            .ok_or("No main window.")?;
        let blocked = self.0.blocked.clone();
        let weak = Arc::downgrade(&self.0);
        let opener = id.to_string();
        let builder = WebviewBuilder::new(
            format!("tab-{id}"),
            WebviewUrl::External("about:blank".parse().map_err(|e| format!("{e}"))?),
        )
        .data_directory(self.0.dir.clone())
        .initialization_script(NATIVE_DIALOGS)
        .initialization_script(CONSOLE)
        .on_navigation(move |url| {
            let ok = safe(url.as_str(), &blocked);
            if !ok {
                log::warn!("browser: refused {url}");
            }
            ok
        })
        .on_new_window(move |url, _| {
            if let Some(inner) = weak.upgrade() {
                let (browser, opener) = (Browser(inner), opener.clone());
                tauri::async_runtime::spawn(
                    async move { browser.popup(&opener, url.as_str()).await },
                );
            }
            tauri::webview::NewWindowResponse::Deny
        });
        let view = tauri::async_runtime::spawn_blocking(move || {
            window.add_child(
                builder,
                LogicalPosition::new(AWAY, 0.0),
                LogicalSize::new(SIZE.0, SIZE.1),
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        native::setup(&view, Arc::downgrade(&self.0), id.to_string())?;
        Ok(view)
    }

    async fn popup(&self, opener: &str, url: &str) {
        let Ok(handle) = self.handle(opener) else {
            return;
        };
        if let Err(e) = self.open(&handle.agent, url).await {
            log::warn!("browser: popup {url}: {e}");
        }
    }

    pub async fn close(&self, tab: &str) -> Result<(), String> {
        let view = self.view(tab)?;
        self.0
            .state
            .lock()
            .unwrap()
            .tabs
            .retain(|t| t.info.id != tab);
        self.changed();
        self.save();
        native::forget(&view, tab);
        view.close().map_err(|e| e.to_string())
    }

    pub async fn close_agent(&self, agent: &str) {
        self.0.saved.lock().unwrap().retain(|s| s.agent != agent);
        for tab in self.tabs(agent) {
            let _ = self.close(&tab.id).await;
        }
        self.save();
    }

    pub async fn navigate(&self, tab: &str, url: &str) -> Result<(), String> {
        let url = address(url);
        self.check(&url)?;
        let view = self.view(tab)?;
        let since = self.counters(tab).map(|c| (c.0, c.1));
        view.navigate(Url::parse(&url).map_err(|e| format!("{url}: {e}"))?)
            .map_err(|e| e.to_string())?;
        self.wait_load(tab, since, Duration::from_secs(10)).await;
        let failed = {
            let st = self.0.state.lock().unwrap();
            st.tabs
                .iter()
                .find(|t| t.info.id == tab)
                .and_then(|t| t.failed.clone())
        };
        failed.map_or(Ok(()), Err)
    }

    pub async fn wait_load(&self, tab: &str, since: Option<(u64, u64)>, timeout: Duration) -> bool {
        let start = Instant::now();
        let since = match since {
            Some(since) => since,
            None => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                match self.counters(tab) {
                    Some((starts, loads, true, _)) => (starts - 1, loads),
                    Some(_) => return true,
                    None => return false,
                }
            }
        };
        loop {
            let Some((starts, loads, loading, dialog)) = self.counters(tab) else {
                return false;
            };
            if loads > since.1 && !loading {
                tokio::time::sleep(Duration::from_millis(300)).await;
                return true;
            }
            if dialog
                || start.elapsed() > timeout
                || (starts == since.0 && start.elapsed() > Duration::from_secs(1))
            {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn history(&self, tab: &str, delta: i64) -> Result<(), String> {
        native::history(&self.view(tab)?, delta).await?;
        self.wait_load(tab, None, Duration::from_secs(10)).await;
        Ok(())
    }

    pub async fn reload(&self, tab: &str) -> Result<(), String> {
        self.view(tab)?.reload().map_err(|e| e.to_string())?;
        self.wait_load(tab, None, Duration::from_secs(10)).await;
        Ok(())
    }

    pub fn place(&self, tab: &str, rect: Option<[f64; 4]>) -> Result<(), String> {
        let view = self.view(tab)?;
        native::place(&view, tab, rect)?;
        if let Some([.., w, _]) = rect {
            self.update(tab, |t| {
                t.shown = w.max(1.0);
                false
            });
        }
        self.zoom_to_width(tab, &view)
    }

    pub fn resize(&self, tab: &str, width: Option<u32>) -> Result<(), String> {
        let view = self.view(tab)?;
        self.update(tab, |t| {
            t.info.width = width.filter(|w| *w > 0);
            true
        });
        self.zoom_to_width(tab, &view)
    }

    fn zoom_to_width(&self, tab: &str, view: &tauri::Webview) -> Result<(), String> {
        let mut zoom = None;
        self.update(tab, |t| {
            let want = t.info.width.map_or(1.0, |w| t.shown / w as f64);
            if (want - t.zoom).abs() > 0.001 {
                t.zoom = want;
                zoom = Some(want);
            }
            false
        });
        zoom.map_or(Ok(()), |z| native::zoom(view, z))
    }

    pub fn zoom(&self, tab: &str) -> f64 {
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .find(|t| t.info.id == tab)
            .map_or(1.0, |t| t.zoom)
    }

    async fn guarded(
        &self,
        tab: &str,
        work: impl Future<Output = Result<Value, String>>,
    ) -> Result<Value, String> {
        let dialog = async {
            loop {
                tokio::time::sleep(Duration::from_millis(40)).await;
                if self.counters(tab).is_none_or(|c| c.3) {
                    break;
                }
            }
        };
        tokio::select! {
            result = tokio::time::timeout(Duration::from_secs(30), work) => {
                result.unwrap_or_else(|_| Err("The page took more than 30 s to answer.".into()))
            }
            _ = dialog => Ok(Value::Null),
        }
    }

    pub async fn page(&self, tab: &str, function: &str, args: Value) -> Result<Value, String> {
        let view = self.view(tab)?;
        let refs = {
            let st = self.0.state.lock().unwrap();
            st.tabs
                .iter()
                .find(|t| t.info.id == tab)
                .map_or(0, |t| t.refs)
        };
        let body = format!(
            "{PAGE}\n__sq.S.n = Math.max(__sq.S.n, {refs})\nreturn __sq.{function}({args})"
        );
        let value = self.guarded(tab, native::eval(&view, body, true)).await?;
        if let Some(title) = value["title"].as_str() {
            self.titled(tab, title.to_string());
        }
        if let Some(n) = value["n"].as_u64() {
            self.update(tab, |t| {
                t.refs = t.refs.max(n);
                false
            });
        }
        Ok(value)
    }

    pub async fn eval_main(&self, tab: &str, expression: &str) -> Result<Value, String> {
        let view = self.view(tab)?;
        match self
            .guarded(
                tab,
                native::eval(&view, format!("return await ({expression}\n)"), false),
            )
            .await
        {
            Err(e) if e.contains("SyntaxError") => {
                self.guarded(tab, native::run(&view, expression.to_string()))
                    .await
            }
            other => other,
        }
    }

    pub fn last(&self, tab: &str) -> Vec<String> {
        let st = self.0.state.lock().unwrap();
        st.tabs
            .iter()
            .find(|t| t.info.id == tab)
            .map(|t| t.last.clone())
            .unwrap_or_default()
    }

    pub fn remember(&self, tab: &str, lines: Vec<String>) {
        self.update(tab, |t| {
            t.last = lines;
            false
        });
    }

    pub async fn shot(
        &self,
        tab: &str,
        full: bool,
        clip: Option<[f64; 4]>,
        width: Option<f64>,
        quality: u32,
    ) -> Result<Image, String> {
        native::snapshot(&self.view(tab)?, full, clip, width, quality).await
    }

    pub async fn picture(&self, tab: &str) -> Result<Vec<u8>, String> {
        Ok(native::snapshot(&self.view(tab)?, false, None, None, 90)
            .await?
            .bytes)
    }

    pub fn keep(&self, tab: &str, png: &[u8]) -> Result<PathBuf, String> {
        let dir = self
            .0
            .app
            .as_ref()
            .ok_or("No app.")?
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;
        crate::attach::save(
            &dir.join("attachments"),
            &format!("page-{tab}.png"),
            &base64::engine::general_purpose::STANDARD.encode(png),
        )
    }

    pub async fn dialog(
        &self,
        tab: &str,
        accept: bool,
        text: Option<String>,
    ) -> Result<(), String> {
        native::dialog(&self.view(tab)?, tab, accept, text).await?;
        self.asked(tab, None);
        Ok(())
    }

    pub async fn files(
        &self,
        tab: &str,
        reference: &str,
        files: Vec<String>,
    ) -> Result<(), String> {
        native::files(&self.view(tab)?, tab, Some(files)).await?;
        let clicked = self
            .page(tab, "click", json!({"ref": reference, "clicks": 1}))
            .await;
        let left = native::files(&self.view(tab)?, tab, None).await?;
        if let Some(cover) = clicked?["covered"].as_str() {
            return Err(format!("{reference} is covered by {cover}."));
        }
        if left {
            return Err(format!("{reference} did not open a file chooser."));
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod native {
    use super::{AWAY, Browser, Image, Inner, PAGE, SIZE, TALL, TALL_INLINE};
    use gtk::prelude::*;
    use javascriptcore::ValueExt;
    use serde_json::{Value, json};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Weak;
    use std::time::Instant;
    use tokio::sync::oneshot;
    use webkit2gtk::{
        BackForwardListExt, FileChooserRequestExt, LoadEvent, ScriptDialog, ScriptDialogType,
        URIRequestExt, URIResponseExt, UserContentInjectedFrames, UserContentManagerExt,
        UserScript, UserScriptInjectionTime, WebResourceExt, WebView, WebViewExt,
        WebViewSessionState,
    };

    thread_local! {
        static RECTS: RefCell<HashMap<String, (i32, i32, i32, i32)>> = Default::default();
        static DIALOGS: RefCell<HashMap<String, ScriptDialog>> = Default::default();
        static FILES: RefCell<HashMap<String, Vec<String>>> = Default::default();
    }

    fn on_main<T: Send + 'static>(
        view: &tauri::Webview,
        f: impl FnOnce(WebView, oneshot::Sender<T>) + Send + 'static,
    ) -> Result<oneshot::Receiver<T>, String> {
        let (tx, rx) = oneshot::channel();
        view.with_webview(move |p| f(p.inner(), tx))
            .map_err(|e| e.to_string())?;
        Ok(rx)
    }

    async fn answer<T>(rx: Result<oneshot::Receiver<T>, String>) -> Result<T, String> {
        rx?.await.map_err(|_| "The tab closed.".to_string())
    }

    fn browser(weak: &Weak<Inner>) -> Option<Browser> {
        weak.upgrade().map(Browser)
    }

    fn adopt(w: &gtk::Widget) {
        let Some(vbox) = w.parent().and_then(|p| p.downcast::<gtk::Box>().ok()) else {
            return;
        };
        vbox.remove(w);
        let overlay = match vbox
            .children()
            .into_iter()
            .find_map(|c| c.downcast::<gtk::Overlay>().ok())
        {
            Some(overlay) => overlay,
            None => {
                let overlay = gtk::Overlay::new();
                if let Some(main) = vbox.children().into_iter().next() {
                    vbox.remove(&main);
                    overlay.add(&main);
                }
                overlay.connect_get_child_position(|_, child| {
                    let name = child.widget_name();
                    RECTS.with(|r| {
                        r.borrow()
                            .get(name.as_str())
                            .map(|&(x, y, w, h)| gtk::gdk::Rectangle::new(x, y, w, h))
                    })
                });
                vbox.pack_start(&overlay, true, true, 0);
                overlay.show_all();
                overlay
            }
        };
        overlay.add_overlay(w);
        w.show();
    }

    pub fn setup(view: &tauri::Webview, weak: Weak<Inner>, tab: String) -> Result<(), String> {
        view.with_webview(move |p| {
            let v = p.inner();
            let w: gtk::Widget = v.clone().upcast();
            w.set_widget_name(&tab);
            RECTS.with(|r| {
                r.borrow_mut()
                    .insert(tab.clone(), (AWAY as i32, 0, SIZE.0 as i32, SIZE.1 as i32))
            });
            adopt(&w);
            if let Some(content) = v.user_content_manager() {
                content.add_script(&UserScript::for_world(
                    PAGE,
                    UserContentInjectedFrames::TopFrame,
                    UserScriptInjectionTime::Start,
                    "squadron",
                    &[],
                    &[],
                ));
            }
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_load_changed(move |v, event| {
                let Some(b) = browser(&b) else { return };
                match event {
                    LoadEvent::Started => b.loading(&t, true),
                    LoadEvent::Committed => b.located(&t, v.uri().unwrap_or_default().into()),
                    LoadEvent::Finished => {
                        b.loading(&t, false);
                        if let Some(state) = v.session_state().and_then(|s| s.serialize()) {
                            b.kept(&t, &state);
                        }
                    }
                    _ => {}
                }
            });
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_uri_notify(move |v| {
                if let Some(b) = browser(&b) {
                    b.located(&t, v.uri().unwrap_or_default().into());
                }
            });
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_title_notify(move |v| {
                if let Some(b) = browser(&b) {
                    b.titled(&t, v.title().unwrap_or_default().into());
                }
            });
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_resource_load_started(move |_, resource, request| {
                let Some(url) = resource.uri().and_then(|u| super::bare(&u)) else {
                    return;
                };
                let method = request.http_method().unwrap_or_else(|| "GET".into());
                let start = Instant::now();
                let error: Rc<RefCell<Option<String>>> = Default::default();
                let e = error.clone();
                resource.connect_failed(move |_, err| {
                    if !err.matches(webkit2gtk::NetworkError::Cancelled) {
                        *e.borrow_mut() = Some(err.message().to_string());
                    }
                });
                let (b, t) = (b.clone(), t.clone());
                resource.connect_finished(move |r| {
                    let Some(b) = browser(&b) else { return };
                    let ms = start.elapsed().as_millis();
                    let status = r.response().map_or(0, |r| r.status_code());
                    match error.borrow_mut().take() {
                        Some(err) => {
                            b.logged(&t, true, format!("{method} failed {url}: {err} ({ms} ms)"))
                        }
                        None => b.logged(
                            &t,
                            status >= 400,
                            format!("{method} {status} {url} ({ms} ms)"),
                        ),
                    }
                });
            });
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_load_failed(move |_, _, uri, error| {
                if let Some(b) = browser(&b)
                    && !error.matches(webkit2gtk::NetworkError::Cancelled)
                {
                    b.failed(&t, format!("{uri}: {error}"));
                }
                false
            });
            let (b, t) = (weak.clone(), tab.clone());
            v.connect_script_dialog(move |_, dialog| {
                let Some(b) = browser(&b) else { return false };
                let kind = match dialog.dialog_type() {
                    ScriptDialogType::Alert => "alert",
                    ScriptDialogType::Confirm => "confirm",
                    ScriptDialogType::Prompt => "prompt",
                    _ => "beforeunload",
                };
                let mut info =
                    json!({"type": kind, "message": dialog.message().unwrap_or_default().as_str()});
                if kind == "prompt" {
                    info["defaultPrompt"] = dialog
                        .prompt_get_default_text()
                        .unwrap_or_default()
                        .as_str()
                        .into();
                }
                DIALOGS.with(|d| d.borrow_mut().insert(t.clone(), dialog.clone()));
                b.asked(&t, Some(info));
                true
            });
            let t = tab.clone();
            v.connect_run_file_chooser(move |_, request| {
                let Some(files) = FILES.with(|f| f.borrow_mut().remove(&t)) else {
                    return false;
                };
                request.select_files(&files.iter().map(String::as_str).collect::<Vec<_>>());
                true
            });
        })
        .map_err(|e| e.to_string())
    }

    pub fn place(view: &tauri::Webview, tab: &str, rect: Option<[f64; 4]>) -> Result<(), String> {
        let tab = tab.to_string();
        view.with_webview(move |p| {
            let w: gtk::Widget = p.inner().upcast();
            RECTS.with(|r| {
                let mut r = r.borrow_mut();
                let old = r
                    .get(&tab)
                    .copied()
                    .unwrap_or((0, 0, SIZE.0 as i32, SIZE.1 as i32));
                let new = match rect {
                    Some([x, y, w, h]) => (
                        x.round() as i32,
                        y.round() as i32,
                        w.round().max(1.0) as i32,
                        h.round().max(1.0) as i32,
                    ),
                    None => (AWAY as i32, 0, old.2, old.3),
                };
                r.insert(tab, new);
            });
            if let Some(parent) = w.parent() {
                parent.queue_resize();
            }
        })
        .map_err(|e| e.to_string())
    }

    pub fn zoom(view: &tauri::Webview, zoom: f64) -> Result<(), String> {
        view.with_webview(move |p| p.inner().set_zoom_level(zoom))
            .map_err(|e| e.to_string())
    }

    pub fn forget(view: &tauri::Webview, tab: &str) {
        let tab = tab.to_string();
        let _ = view.with_webview(move |_| {
            RECTS.with(|r| r.borrow_mut().remove(&tab));
            DIALOGS.with(|d| d.borrow_mut().remove(&tab));
            FILES.with(|f| f.borrow_mut().remove(&tab));
        });
    }

    fn value(r: Result<javascriptcore::Value, gtk::glib::Error>) -> Result<Value, String> {
        let v = r.map_err(|e| e.message().to_string())?;
        Ok(v.to_json(0)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null))
    }

    pub async fn eval(
        view: &tauri::Webview,
        body: String,
        isolated: bool,
    ) -> Result<Value, String> {
        let world = isolated.then_some("squadron");
        answer(on_main(view, move |v, tx| {
            v.call_async_javascript_function(
                &body,
                None,
                world,
                None,
                None::<&gtk::gio::Cancellable>,
                move |r| {
                    let _ = tx.send(value(r));
                },
            );
        }))
        .await?
    }

    pub async fn run(view: &tauri::Webview, script: String) -> Result<Value, String> {
        answer(on_main(view, move |v, tx| {
            v.evaluate_javascript(
                &script,
                None,
                None,
                None::<&gtk::gio::Cancellable>,
                move |r| {
                    let _ = tx.send(value(r));
                },
            );
        }))
        .await?
    }

    pub async fn history(view: &tauri::Webview, delta: i64) -> Result<(), String> {
        answer(on_main(view, move |v, tx| {
            let _ = tx.send(match delta {
                ..=-1 if v.can_go_back() => {
                    v.go_back();
                    Ok(())
                }
                1.. if v.can_go_forward() => {
                    v.go_forward();
                    Ok(())
                }
                _ => Err("No page there in the history.".to_string()),
            });
        }))
        .await?
    }

    pub async fn restore(view: &tauri::Webview, session: Vec<u8>) -> Result<bool, String> {
        answer(on_main(view, move |v, tx| {
            let state = WebViewSessionState::new(&gtk::glib::Bytes::from_owned(session));
            v.restore_session_state(&state);
            let item = v.back_forward_list().and_then(|l| l.current_item());
            if let Some(item) = &item {
                v.go_to_back_forward_list_item(item);
            }
            let _ = tx.send(item.is_some());
        }))
        .await
    }

    fn encode(
        r: Result<gtk::cairo::Surface, gtk::glib::Error>,
        scale: f64,
        clip: Option<[f64; 4]>,
        width: Option<f64>,
        quality: u32,
    ) -> Result<Image, String> {
        let surface = gtk::cairo::ImageSurface::try_from(r.map_err(|e| e.message().to_string())?)
            .map_err(|_| "No image.".to_string())?;
        let (sw, sh) = (surface.width(), surface.height());
        let [x, y, w, h] = clip.unwrap_or([0.0, 0.0, sw as f64 / scale, sh as f64 / scale]);
        let x0 = ((x * scale).round() as i32).clamp(0, sw - 1);
        let y0 = ((y * scale).round() as i32).clamp(0, sh - 1);
        let w0 = ((w * scale).round() as i32).clamp(1, sw - x0);
        let tallest = match width {
            Some(max) => (TALL_INLINE * w0 as f64 / max.min(w0 as f64)) as i32,
            None => TALL,
        };
        let h1 = ((h * scale).round() as i32).clamp(1, sh - y0);
        let h0 = h1.min(tallest);
        let mut rgb = Vec::with_capacity((w0 * h0 * 3) as usize);
        let stride = surface.stride() as usize;
        surface
            .with_data(|d| {
                for y in y0 as usize..(y0 + h0) as usize {
                    for x in x0 as usize..(x0 + w0) as usize {
                        let i = y * stride + x * 4;
                        rgb.extend([d[i + 2], d[i + 1], d[i]]);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let mut pixbuf = gtk::gdk_pixbuf::Pixbuf::from_bytes(
            &gtk::glib::Bytes::from_owned(rgb),
            gtk::gdk_pixbuf::Colorspace::Rgb,
            false,
            8,
            w0,
            h0,
            w0 * 3,
        );
        if let Some(max) = width
            && w0 as f64 > max
        {
            let h1 = (h0 as f64 * max / w0 as f64).round().max(1.0) as i32;
            pixbuf = pixbuf
                .scale_simple(max as i32, h1, gtk::gdk_pixbuf::InterpType::Bilinear)
                .ok_or("No image.")?;
        }
        let bytes = if quality == 0 {
            pixbuf.save_to_bufferv("png", &[])
        } else {
            pixbuf.save_to_bufferv("jpeg", &[("quality", &quality.to_string())])
        }
        .map_err(|e| e.message().to_string())?;
        Ok(Image {
            bytes,
            width: pixbuf.width(),
            height: pixbuf.height(),
            cut: h0 < h1,
        })
    }

    pub async fn snapshot(
        view: &tauri::Webview,
        full: bool,
        clip: Option<[f64; 4]>,
        width: Option<f64>,
        quality: u32,
    ) -> Result<Image, String> {
        answer(on_main(view, move |v, tx| {
            let scale = v.scale_factor().max(1) as f64;
            v.snapshot(
                if full {
                    webkit2gtk::SnapshotRegion::FullDocument
                } else {
                    webkit2gtk::SnapshotRegion::Visible
                },
                webkit2gtk::SnapshotOptions::NONE,
                None::<&gtk::gio::Cancellable>,
                move |r| {
                    let _ = tx.send(encode(r, scale, clip, width, quality));
                },
            );
        }))
        .await?
    }

    pub async fn dialog(
        view: &tauri::Webview,
        tab: &str,
        accept: bool,
        text: Option<String>,
    ) -> Result<(), String> {
        let tab = tab.to_string();
        answer(on_main(view, move |_, tx| {
            let Some(dialog) = DIALOGS.with(|d| d.borrow_mut().remove(&tab)) else {
                let _ = tx.send(Err("No dialog is open.".to_string()));
                return;
            };
            match dialog.dialog_type() {
                ScriptDialogType::Confirm | ScriptDialogType::BeforeUnloadConfirm => {
                    dialog.confirm_set_confirmed(accept)
                }
                ScriptDialogType::Prompt if accept => {
                    dialog.prompt_set_text(&text.unwrap_or_default())
                }
                _ => {}
            }
            dialog.close();
            let _ = tx.send(Ok(()));
        }))
        .await?
    }

    pub async fn files(
        view: &tauri::Webview,
        tab: &str,
        files: Option<Vec<String>>,
    ) -> Result<bool, String> {
        let tab = tab.to_string();
        answer(on_main(view, move |_, tx| {
            let left = FILES.with(|f| match files {
                Some(files) => f.borrow_mut().insert(tab, files).is_some(),
                None => f.borrow_mut().remove(&tab).is_some(),
            });
            let _ = tx.send(left);
        }))
        .await
    }
}

#[cfg(not(target_os = "linux"))]
mod native {
    use super::{Image, Inner};
    use serde_json::Value;
    use std::sync::Weak;

    const ONLY: &str = "The built-in browser works on Linux only for now.";

    pub fn setup(_: &tauri::Webview, _: Weak<Inner>, _: String) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn place(_: &tauri::Webview, _: &str, _: Option<[f64; 4]>) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn zoom(_: &tauri::Webview, _: f64) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn forget(_: &tauri::Webview, _: &str) {}
    pub async fn eval(_: &tauri::Webview, _: String, _: bool) -> Result<Value, String> {
        Err(ONLY.into())
    }
    pub async fn run(_: &tauri::Webview, _: String) -> Result<Value, String> {
        Err(ONLY.into())
    }
    pub async fn history(_: &tauri::Webview, _: i64) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub async fn restore(_: &tauri::Webview, _: Vec<u8>) -> Result<bool, String> {
        Err(ONLY.into())
    }
    pub async fn snapshot(
        _: &tauri::Webview,
        _: bool,
        _: Option<[f64; 4]>,
        _: Option<f64>,
        _: u32,
    ) -> Result<Image, String> {
        Err(ONLY.into())
    }
    pub async fn dialog(
        _: &tauri::Webview,
        _: &str,
        _: bool,
        _: Option<String>,
    ) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub async fn files(
        _: &tauri::Webview,
        _: &str,
        _: Option<Vec<String>>,
    ) -> Result<bool, String> {
        Err(ONLY.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(
            address("example.com/a b"),
            "https://www.google.com/search?q=example.com%2Fa+b"
        );
        assert_eq!(address("example.com/a"), "https://example.com/a");
        assert_eq!(address("localhost:3000"), "http://localhost:3000");
        assert_eq!(address("shop.test"), "http://shop.test");
        assert_eq!(address("https://x.fr"), "https://x.fr");
        assert_eq!(
            address("rust lang"),
            "https://www.google.com/search?q=rust+lang"
        );
    }

    #[test]
    fn keeps_tabs_off_the_app() {
        let blocked = [
            "tauri://localhost".to_string(),
            "http://localhost:5173".to_string(),
        ];
        for url in [
            "https://example.com/a",
            "http://localhost:3000",
            "about:blank",
            "data:text/html,x",
        ] {
            assert!(safe(url, &blocked), "{url}");
        }
        for url in [
            "tauri://localhost/",
            "http://localhost:5173/x",
            "artifact://localhost/a/index.html",
            "extension://localhost/x/panel.html",
            "ipc://localhost/browser_status",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "nonsense",
        ] {
            assert!(!safe(url, &blocked), "{url}");
        }
    }
}
