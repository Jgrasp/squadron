use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, UNIX_EPOCH};

pub static ROOT: OnceLock<PathBuf> = OnceLock::new();

const PICKER: &str = include_str!("artifact-pick.js");
const PICKER_PATH: &str = "/__squadron/pick.js";
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

fn link_of(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(META)).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&text).ok()?;
    meta["link"].as_str().map(String::from)
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

pub fn set_link(root: &Path, agent: &str, id: &str, link: &str) -> Result<(), String> {
    let dir = artifact_dir(root, agent, id)?;
    if entry_of(&dir).is_none() {
        return Err("The artifact is gone.".into());
    }
    std::fs::write(
        dir.join(META),
        serde_json::json!({ "link": link }).to_string(),
    )
    .map_err(|e| e.to_string())
}

pub fn remove(root: &Path, agent: &str, id: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(artifact_dir(root, agent, id)?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

pub fn forget(root: &Path, agent: &str) {
    if let Ok(dir) = agent_dir(root, agent) {
        let _ = std::fs::remove_dir_all(dir);
    }
}

pub fn sweep(root: &Path, keep: &HashSet<String>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for item in read.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        if !keep.contains(&name) {
            let _ = std::fs::remove_dir_all(item.path());
        }
    }
}

pub fn expire(root: &Path, archived: &Path, days: u32) {
    if days == 0 {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let limit = now.saturating_sub(u64::from(days) * 86_400_000);
    for item in std::fs::read_dir(archived).into_iter().flatten().flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let since = std::fs::read_to_string(item.path())
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|data| data["archived"].as_u64())
            .or_else(|| item.metadata().ok().map(|m| millis(&m)));
        if since.is_some_and(|at| at < limit) {
            forget(root, id);
        }
    }
}

fn decode(path: &str) -> Option<String> {
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

fn mime(path: &Path) -> &'static str {
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
    fn deletes_the_artifacts_of_agents_archived_for_too_long() {
        let root = root();
        let archived = root.join("archived-agents");
        std::fs::create_dir_all(&archived).unwrap();
        let day = 86_400_000u64;
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        for (id, since) in [("old", now - 31 * day), ("recent", now - 2 * day)] {
            write(&root, id, "a", "markdown", "x").unwrap();
            std::fs::write(
                archived.join(format!("{id}.json")),
                format!("{{\"archived\":{since}}}"),
            )
            .unwrap();
        }
        write(&root, "active", "a", "markdown", "x").unwrap();
        expire(&root, &archived, 0);
        assert!(root.join("old").exists());
        expire(&root, &archived, 30);
        assert!(
            !root.join("old").exists()
                && root.join("recent").exists()
                && root.join("active").exists()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sweeps_the_folders_of_agents_that_no_longer_exist() {
        let root = root();
        write(&root, "kept", "a", "markdown", "x").unwrap();
        write(&root, "gone", "a", "markdown", "x").unwrap();
        sweep(&root, &HashSet::from(["kept".to_string()]));
        assert!(root.join("kept").exists() && !root.join("gone").exists());
        forget(&root, "kept");
        assert!(!root.join("kept").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
