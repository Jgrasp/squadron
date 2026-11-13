use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, UNIX_EPOCH};

pub static ROOT: OnceLock<PathBuf> = OnceLock::new();

const PICKER: &str = include_str!("artifact-pick.js");
const PICKER_PATH: &str = "/__squadron/pick.js";
const THEME_PATH: &str = "/__squadron/theme.css";
const FONTS_PATH: &str = "/__squadron/fonts/";
const APP_CSS: &str = include_str!("../../src/app.css");
const FONTS: [(&str, &str, u16, &[u8]); 7] = [
    (
        "dm-sans-latin-400-normal.woff2",
        "DM Sans",
        400,
        include_bytes!(
            "../../node_modules/@fontsource/dm-sans/files/dm-sans-latin-400-normal.woff2"
        ),
    ),
    (
        "dm-sans-latin-500-normal.woff2",
        "DM Sans",
        500,
        include_bytes!(
            "../../node_modules/@fontsource/dm-sans/files/dm-sans-latin-500-normal.woff2"
        ),
    ),
    (
        "dm-sans-latin-600-normal.woff2",
        "DM Sans",
        600,
        include_bytes!(
            "../../node_modules/@fontsource/dm-sans/files/dm-sans-latin-600-normal.woff2"
        ),
    ),
    (
        "dm-sans-latin-700-normal.woff2",
        "DM Sans",
        700,
        include_bytes!(
            "../../node_modules/@fontsource/dm-sans/files/dm-sans-latin-700-normal.woff2"
        ),
    ),
    (
        "dm-sans-latin-400-italic.woff2",
        "DM Sans",
        400,
        include_bytes!(
            "../../node_modules/@fontsource/dm-sans/files/dm-sans-latin-400-italic.woff2"
        ),
    ),
    (
        "dm-mono-latin-400-normal.woff2",
        "DM Mono",
        400,
        include_bytes!(
            "../../node_modules/@fontsource/dm-mono/files/dm-mono-latin-400-normal.woff2"
        ),
    ),
    (
        "dm-mono-latin-500-normal.woff2",
        "DM Mono",
        500,
        include_bytes!(
            "../../node_modules/@fontsource/dm-mono/files/dm-mono-latin-500-normal.woff2"
        ),
    ),
];
const META: &str = ".squadron.json";
const ENTRIES: [(&str, &str); 3] = [
    ("index.html", "html"),
    ("index.md", "markdown"),
    ("index.svg", "svg"),
];
const CSP: &str = "default-src 'self' artifact: http://artifact.localhost https: data: blob: 'unsafe-inline' 'unsafe-eval'; form-action 'none'";

#[derive(Serialize, Debug, PartialEq)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub entry: String,
    pub title: String,
    pub modified: u64,
    pub link: Option<String>,
    pub links: serde_json::Map<String, serde_json::Value>,
    pub keep: Option<u32>,
}

fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

pub fn agent_dir(root: &Path, agent: &str) -> Result<PathBuf, String> {
    if !name_ok(agent) {
        return Err("Invalid agent id.".into());
    }
    Ok(root.join(agent))
}

fn artifact_dir(root: &Path, agent: &str, id: &str) -> Result<PathBuf, String> {
    if !name_ok(id) {
        return Err("Invalid artifact id.".into());
    }
    Ok(agent_dir(root, agent)?.join(id))
}

fn entry_of(dir: &Path) -> Option<(&'static str, &'static str)> {
    ENTRIES
        .into_iter()
        .find(|(file, _)| dir.join(file).is_file())
}

fn millis(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

fn scan(dir: &Path, newest: &mut u64, count: &mut u64) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for item in read.flatten() {
        if item.file_name() == META {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        *count += 1;
        *newest = (*newest).max(millis(&meta));
        if meta.is_dir() && !meta.is_symlink() {
            scan(&item.path(), newest, count);
        }
    }
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let lower = text.to_ascii_lowercase();
    let start = lower.find(open)? + open.len();
    let end = start + lower[start..].find(close)?;
    Some(text[start..end].trim()).filter(|t| !t.is_empty())
}

fn title_of(path: &Path, kind: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let found = match kind {
        "markdown" => text
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .map(str::trim),
        _ => between(&text, "<title>", "</title>"),
    };
    found.map(String::from)
}

fn meta_of(dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(dir.join(META))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn link_of(dir: &Path) -> Option<String> {
    meta_of(dir).get("link")?.as_str().map(String::from)
}

fn links_of(dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    match meta_of(dir).remove("links") {
        Some(serde_json::Value::Object(links)) => links,
        _ => Default::default(),
    }
}

pub fn list(root: &Path, agent: &str) -> Result<Vec<Artifact>, String> {
    let Ok(read) = std::fs::read_dir(agent_dir(root, agent)?) else {
        return Ok(vec![]);
    };
    let mut found: Vec<Artifact> = read
        .flatten()
        .filter_map(|item| {
            let id = item.file_name().to_str()?.to_string();
            let dir = item.path();
            if !name_ok(&id) || !dir.is_dir() {
                return None;
            }
            let (entry, kind) = entry_of(&dir)?;
            let (mut modified, mut count) = (0, 0);
            scan(&dir, &mut modified, &mut count);
            Some(Artifact {
                title: title_of(&dir.join(entry), kind).unwrap_or_else(|| id.clone()),
                id,
                kind: kind.into(),
                entry: entry.into(),
                modified,
                link: link_of(&dir),
                links: links_of(&dir),
                keep: meta_of(&dir)
                    .get("keep")
                    .and_then(|k| k.as_u64())
                    .map(|k| k as u32),
            })
        })
        .collect();
    found.sort_by_key(|a| a.modified);
    Ok(found)
}

pub fn read(root: &Path, agent: &str, id: &str) -> Result<String, String> {
    let dir = artifact_dir(root, agent, id)?;
    let (entry, _) = entry_of(&dir).ok_or("The artifact is gone.")?;
    std::fs::read_to_string(dir.join(entry)).map_err(|e| e.to_string())
}

pub fn write(
    root: &Path,
    agent: &str,
    id: &str,
    kind: &str,
    content: &str,
) -> Result<PathBuf, String> {
    let (file, _) = ENTRIES
        .into_iter()
        .find(|(_, k)| *k == kind)
        .ok_or("Unknown kind.")?;
    let dir = artifact_dir(root, agent, id)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (other, _) in ENTRIES.into_iter().filter(|(f, _)| *f != file) {
        let _ = std::fs::remove_file(dir.join(other));
    }
    let path = dir.join(file);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

fn change_meta(
    root: &Path,
    agent: &str,
    id: &str,
    change: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    let dir = artifact_dir(root, agent, id)?;
    if entry_of(&dir).is_none() {
        return Err("The artifact is gone.".into());
    }
    let mut meta = meta_of(&dir);
    change(&mut meta);
    std::fs::write(dir.join(META), serde_json::Value::Object(meta).to_string())
        .map_err(|e| e.to_string())
}

pub fn set_link(root: &Path, agent: &str, id: &str, link: &str) -> Result<(), String> {
    change_meta(root, agent, id, |meta| {
        meta.insert("link".into(), link.into());
    })
}

pub fn set_shared(
    root: &Path,
    agent: &str,
    id: &str,
    destination: &str,
    link: Option<&str>,
    at: u64,
) -> Result<(), String> {
    change_meta(root, agent, id, |meta| {
        let mut links = match meta.remove("links") {
            Some(serde_json::Value::Object(links)) => links,
            _ => Default::default(),
        };
        match link {
            Some(link) => links.insert(
                destination.into(),
                serde_json::json!({ "link": link, "at": at }),
            ),
            None => links.remove(destination),
        };
        if !links.is_empty() {
            meta.insert("links".into(), links.into());
        }
    })
}

pub fn set_keep(root: &Path, agent: &str, id: &str, keep: Option<u32>) -> Result<(), String> {
    change_meta(root, agent, id, |meta| match keep {
        Some(days) => {
            meta.insert("keep".into(), days.into());
        }
        None => {
            meta.remove("keep");
        }
    })
}

pub fn owners(root: &Path) -> Vec<String> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|item| item.path().is_dir())
        .filter_map(|item| item.file_name().to_str().map(String::from))
        .filter(|name| name_ok(name))
        .collect()
}

fn shared(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|item| !links_of(&item.path()).is_empty())
}

pub fn remove(root: &Path, agent: &str, id: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(artifact_dir(root, agent, id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

pub fn forget(root: &Path, agent: &str) {
    let Ok(dir) = agent_dir(root, agent) else {
        return;
    };
    for item in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        if links_of(&item.path()).is_empty() {
            let _ = std::fs::remove_dir_all(item.path());
        }
    }
    let _ = std::fs::remove_dir(dir);
}

pub fn sweep(root: &Path, keep: &HashSet<String>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for item in read.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if !keep.contains(&name) && !shared(&item.path()) {
            let _ = std::fs::remove_dir_all(item.path());
        }
    }
}

pub(crate) fn decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn resolve(root: &Path, path: &str) -> Option<PathBuf> {
    let path = decode(path)?;
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 3
        || parts
            .iter()
            .any(|p| p.starts_with('.') || p.contains('\\') || p.contains('\0'))
    {
        return None;
    }
    let file = parts.iter().fold(root.to_path_buf(), |dir, p| dir.join(p));
    let inside = file
        .canonicalize()
        .ok()?
        .starts_with(root.canonicalize().ok()?);
    (inside && file.is_file()).then_some(file)
}

pub(crate) fn mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "md" | "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn inject(html: &str) -> String {
    let tag = format!("<script src=\"{PICKER_PATH}\"></script>");
    match html.to_ascii_lowercase().rfind("</body>") {
        Some(at) => format!("{}{tag}{}", &html[..at], &html[at..]),
        None => format!("{html}{tag}"),
    }
}

fn theme() -> String {
    let faces: String = FONTS
        .iter()
        .map(|(file, family, weight, _)| {
            let style = if file.ends_with("italic.woff2") { "italic" } else { "normal" };
            format!("@font-face{{font-family:'{family}';font-style:{style};font-weight:{weight};font-display:swap;src:url({FONTS_PATH}{file}) format('woff2')}}\n")
        })
        .collect();
    let app: String = APP_CSS
        .lines()
        .filter(|line| !line.starts_with("@import"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{faces}{app}\nhtml,body{{height:auto;overflow:auto}}\n")
}

fn respond(status: u16, mime: &str, body: Vec<u8>) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", "no-store")
        .header("Content-Security-Policy", CSP)
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap()
}

pub fn serve(root: &Path, path: &str) -> http::Response<Vec<u8>> {
    if path == PICKER_PATH {
        return respond(200, mime(Path::new("pick.js")), PICKER.into());
    }
    if path == THEME_PATH {
        return respond(200, mime(Path::new("theme.css")), theme().into());
    }
    if let Some((_, _, _, bytes)) = path
        .strip_prefix(FONTS_PATH)
        .and_then(|name| FONTS.iter().find(|(file, ..)| *file == name))
    {
        return respond(200, mime(Path::new("font.woff2")), bytes.to_vec());
    }
    let Some(file) = resolve(root, path) else {
        return respond(404, "text/plain", b"Not found".to_vec());
    };
    let Ok(bytes) = std::fs::read(&file) else {
        return respond(404, "text/plain", b"Not found".to_vec());
    };
    let kind = mime(&file);
    if kind.starts_with("text/html") {
        return respond(200, kind, inject(&String::from_utf8_lossy(&bytes)).into());
    }
    respond(200, kind, bytes)
}

pub fn watch(root: PathBuf, changed: impl Fn(&str) + Send + 'static) {
    std::thread::spawn(move || {
        let mut seen: HashMap<String, (u64, u64)> = HashMap::new();
        loop {
            let mut now = HashMap::new();
            for item in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                let (mut newest, mut count) = (0, 0);
                scan(&item.path(), &mut newest, &mut count);
                now.insert(
                    item.file_name().to_string_lossy().to_string(),
                    (newest, count),
                );
            }
            for agent in now.keys().chain(seen.keys()).collect::<HashSet<_>>() {
                if now.get(agent) != seen.get(agent) {
                    changed(agent);
                }
            }
            seen = now;
            std::thread::sleep(Duration::from_millis(600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("squadron-artifacts-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_each_folder_with_an_index_as_an_artifact() {
        let root = root();
        write(
            &root,
            "a1",
            "plan",
            "markdown",
            "# Plan de migration\n\n1. Geler",
        )
        .unwrap();
        let page = write(
            &root,
            "a1",
            "page",
            "html",
            "<title>Accueil</title><p>Hi</p>",
        )
        .unwrap();
        std::fs::write(page.with_file_name("style.css"), "p{}").unwrap();
        std::fs::create_dir_all(root.join("a1/empty")).unwrap();
        set_link(&root, "a1", "plan", "https://claude.ai/artifact/x").unwrap();
        let mut found = list(&root, "a1").unwrap();
        found.sort_by(|a, b| a.id.cmp(&b.id));
        let summary: Vec<_> = found
            .iter()
            .map(|a| {
                (
                    a.id.as_str(),
                    a.kind.as_str(),
                    a.title.as_str(),
                    a.link.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("page", "html", "Accueil", None),
                (
                    "plan",
                    "markdown",
                    "Plan de migration",
                    Some("https://claude.ai/artifact/x")
                ),
            ]
        );
        assert_eq!(
            read(&root, "a1", "plan").unwrap(),
            "# Plan de migration\n\n1. Geler"
        );
        write(&root, "a1", "plan", "svg", "<svg/>").unwrap();
        assert_eq!(
            list(&root, "a1")
                .unwrap()
                .iter()
                .find(|a| a.id == "plan")
                .unwrap()
                .kind,
            "svg"
        );
        remove(&root, "a1", "page").unwrap();
        assert_eq!(list(&root, "a1").unwrap().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_a_link_per_destination_without_touching_the_edit_time() {
        let root = root();
        write(&root, "a1", "plan", "html", "<title>Plan</title>").unwrap();
        let before = list(&root, "a1").unwrap()[0].modified;
        std::thread::sleep(std::time::Duration::from_millis(20));
        set_shared(
            &root,
            "a1",
            "plan",
            "team.share",
            Some("https://script.google.com/x/exec"),
            42,
        )
        .unwrap();
        set_link(&root, "a1", "plan", "https://claude.ai/artifact/x").unwrap();
        let found = &list(&root, "a1").unwrap()[0];
        assert_eq!(found.modified, before);
        assert_eq!(found.link.as_deref(), Some("https://claude.ai/artifact/x"));
        assert_eq!(
            found.links["team.share"],
            serde_json::json!({ "link": "https://script.google.com/x/exec", "at": 42 })
        );
        set_shared(&root, "a1", "plan", "team.share", None, 43).unwrap();
        let found = &list(&root, "a1").unwrap()[0];
        assert!(found.links.is_empty());
        assert_eq!(found.link.as_deref(), Some("https://claude.ai/artifact/x"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn serves_only_files_inside_the_artifacts_folder() {
        let root = root();
        write(
            &root,
            "a1",
            "page",
            "html",
            "<html><body><p>Hi</p></body></html>",
        )
        .unwrap();
        std::fs::write(root.join("a1/page/.squadron.json"), "{}").unwrap();
        let page = serve(&root, "/a1/page/index.html");
        assert_eq!(page.status(), 200);
        let body = String::from_utf8(page.body().clone()).unwrap();
        assert!(
            body.ends_with("<p>Hi</p><script src=\"/__squadron/pick.js\"></script></body></html>")
        );
        assert_eq!(serve(&root, "/__squadron/pick.js").status(), 200);
        let theme =
            String::from_utf8(serve(&root, "/__squadron/theme.css").body().clone()).unwrap();
        assert!(theme.contains("--accent: #e08a5b") && !theme.contains("@import"));
        for (file, ..) in FONTS {
            assert!(theme.contains(file));
            assert_eq!(
                serve(&root, &format!("/__squadron/fonts/{file}")).status(),
                200
            );
        }
        assert_eq!(serve(&root, "/__squadron/fonts/../pick.js").status(), 404);
        for bad in [
            "/a1/page/../../secret.txt",
            "/a1/page/%2e%2e/%2e%2e/x",
            "/a1/page/.squadron.json",
            "/a1/page",
            "/a1/page/missing.css",
        ] {
            assert_eq!(serve(&root, bad).status(), 404, "{bad}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_its_own_cleanup_time_and_lists_the_agent_folders() {
        let root = root();
        write(&root, "agent", "a", "markdown", "x").unwrap();
        assert_eq!(list(&root, "agent").unwrap()[0].keep, None);
        set_keep(&root, "agent", "a", Some(0)).unwrap();
        assert_eq!(list(&root, "agent").unwrap()[0].keep, Some(0));
        set_keep(&root, "agent", "a", None).unwrap();
        assert_eq!(list(&root, "agent").unwrap()[0].keep, None);
        assert_eq!(owners(&root), vec!["agent".to_string()]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sweeps_the_folders_of_agents_that_no_longer_exist() {
        let root = root();
        write(&root, "kept", "a", "markdown", "x").unwrap();
        write(&root, "gone", "a", "markdown", "x").unwrap();
        write(&root, "shared", "a", "markdown", "x").unwrap();
        set_shared(&root, "shared", "a", "ext.team", Some("https://x.test"), 1).unwrap();
        sweep(&root, &HashSet::from(["kept".to_string()]));
        assert!(root.join("kept").exists() && !root.join("gone").exists());
        assert!(root.join("shared").exists());
        forget(&root, "kept");
        assert!(!root.join("kept").exists());
        write(&root, "shared", "b", "markdown", "x").unwrap();
        forget(&root, "shared");
        assert!(root.join("shared").join("a").exists() && !root.join("shared").join("b").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
