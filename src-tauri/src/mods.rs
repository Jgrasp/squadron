use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_MANIFEST: u64 = 256 * 1024;

pub struct Mods {
    pub dir: PathBuf,
    pub file: PathBuf,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Choices {
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
    #[serde(default)]
    pub approved: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Serialize, Debug)]
pub struct Found {
    pub dir: String,
    pub linked: bool,
    pub manifest: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Listing {
    pub mods: Vec<Found>,
    pub disabled: Vec<String>,
    pub approved: std::collections::BTreeMap<String, Vec<String>>,
}

impl Mods {
    pub fn choices(&self) -> Choices {
        std::fs::read_to_string(&self.file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, choices: &Choices) -> Result<(), String> {
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(choices).map_err(|e| e.to_string())?,
        )
        .and_then(|_| std::fs::rename(&tmp, &self.file))
        .map_err(|e| e.to_string())
    }

    fn folders(&self, choices: &Choices) -> Vec<(PathBuf, bool)> {
        let mut installed: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        installed.sort();
        installed
            .into_iter()
            .map(|p| (p, false))
            .chain(choices.folders.iter().map(|f| (PathBuf::from(f), true)))
            .collect()
    }

    pub fn list(&self) -> Listing {
        let choices = self.choices();
        Listing {
            mods: self
                .folders(&choices)
                .into_iter()
                .map(|(dir, linked)| read(&dir, linked))
                .collect(),
            disabled: choices.disabled,
            approved: choices.approved,
        }
    }

    pub fn link(&self, path: &str) -> Result<(), String> {
        if !Path::new(path).join("mod.json").is_file() {
            return Err("This folder has no mod.json.".into());
        }
        let mut choices = self.choices();
        if !choices.folders.iter().any(|f| f == path) {
            choices.folders.push(path.to_string());
        }
        self.save(&choices)
    }

    pub fn unlink(&self, path: &str) -> Result<(), String> {
        let mut choices = self.choices();
        choices.folders.retain(|f| f != path);
        self.save(&choices)
    }

    pub fn enable(&self, id: &str, on: bool) -> Result<(), String> {
        let mut choices = self.choices();
        choices.disabled.retain(|d| d != id);
        if !on {
            choices.disabled.push(id.to_string());
        }
        self.save(&choices)
    }

    pub fn approve(&self, id: &str, commands: Vec<String>) -> Result<(), String> {
        let mut choices = self.choices();
        choices.approved.insert(id.to_string(), commands);
        self.save(&choices)
    }

    fn stamp(&self) -> Vec<(PathBuf, Option<(SystemTime, u64)>)> {
        let modified = |p: &Path| {
            std::fs::metadata(p)
                .ok()
                .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()))
        };
        let mut seen = vec![(self.file.clone(), modified(&self.file))];
        for (dir, _) in self.folders(&self.choices()) {
            let file = dir.join("mod.json");
            seen.push((file.clone(), modified(&file)));
            let wasm = std::fs::read_to_string(&file)
                .ok()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                .and_then(|m| m["wasm"].as_str().map(|w| dir.join(w)));
            if let Some(wasm) = wasm {
                seen.push((wasm.clone(), modified(&wasm)));
            }
        }
        seen
    }

    pub fn watch(self, changed: impl Fn() + Send + 'static) {
        std::thread::spawn(move || {
            let mut seen = self.stamp();
            loop {
                std::thread::sleep(Duration::from_millis(700));
                let now = self.stamp();
                if now != seen {
                    changed();
                }
                seen = now;
            }
        });
    }

    fn page(&self, path: &str) -> Option<PathBuf> {
        let path = crate::artifacts::decode(path)?;
        let (id, rest) = path.trim_start_matches('/').split_once('/')?;
        if rest
            .split('/')
            .any(|p| p.is_empty() || p.starts_with('.') || p.contains(['\\', '\0']))
        {
            return None;
        }
        let listing = self.list();
        if listing.disabled.iter().any(|d| d == id) {
            return None;
        }
        let found = listing
            .mods
            .into_iter()
            .find(|f| f.manifest.as_ref().and_then(|m| m["id"].as_str()) == Some(id))?;
        let root = Path::new(&found.dir).canonicalize().ok()?;
        let file = root.join(rest).canonicalize().ok()?;
        (file.starts_with(&root) && file.is_file()).then_some(file)
    }

    pub fn serve(&self, path: &str) -> http::Response<Vec<u8>> {
        let (status, kind, body) = match path {
            "/_squadron/kit.js" => (200, "text/javascript; charset=utf-8", KIT_JS.into()),
            "/_squadron/kit.css" => (200, "text/css; charset=utf-8", KIT_CSS.into()),
            _ => match self
                .page(path)
                .and_then(|f| Some((std::fs::read(&f).ok()?, f)))
            {
                None => (404, "text/plain", b"Not found".to_vec()),
                Some((bytes, file)) => {
                    let kind = crate::artifacts::mime(&file);
                    let body = if kind.starts_with("text/html") {
                        inject(&String::from_utf8_lossy(&bytes)).into()
                    } else {
                        bytes
                    };
                    (200, kind, body)
                }
            },
        };
        http::Response::builder()
            .status(status)
            .header("Content-Type", kind)
            .header("Cache-Control", "no-store")
            .header("Content-Security-Policy", PANEL_CSP)
            .body(body)
            .unwrap()
    }
}

const KIT_JS: &str = include_str!("../../mods/kit/kit.js");
const KIT_CSS: &str = include_str!("../../mods/kit/kit.css");
const PANEL_CSP: &str = "default-src 'self' mod: http://mod.localhost data: blob: 'unsafe-inline'; connect-src 'none'; form-action 'none'";

fn inject(html: &str) -> String {
    let tag = r#"<script src="/_squadron/kit.js"></script>"#;
    let lower = html.to_ascii_lowercase();
    let after = |open: &str| {
        let at = lower.find(open)?;
        Some(at + lower[at..].find('>')? + 1)
    };
    match after("<head").or_else(|| after("<!doctype")) {
        Some(at) => format!("{}{tag}{}", &html[..at], &html[at..]),
        None => format!("{tag}{html}"),
    }
}

pub fn read(dir: &Path, linked: bool) -> Found {
    let file = dir.join("mod.json");
    let manifest = match std::fs::metadata(&file) {
        Err(_) => Err("mod.json is missing.".to_string()),
        Ok(m) if m.len() > MAX_MANIFEST => Err("mod.json is larger than 256 KB.".to_string()),
        Ok(_) => std::fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|text| {
                serde_json::from_str(&text).map_err(|e| format!("mod.json is not valid JSON: {e}"))
            }),
    };
    let (manifest, error) = match manifest {
        Ok(value) => (Some(value), None),
        Err(e) => (None, Some(e)),
    };
    Found {
        dir: dir.to_string_lossy().to_string(),
        linked,
        manifest,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods() -> Mods {
        let root = std::env::temp_dir().join(format!("squadron-mods-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("mods")).unwrap();
        Mods {
            dir: root.join("mods"),
            file: root.join("mods.json"),
        }
    }

    #[test]
    fn serves_the_pages_of_enabled_mods_only() {
        let m = mods();
        std::fs::create_dir_all(m.dir.join("good/panel")).unwrap();
        std::fs::write(m.dir.join("good/mod.json"), r#"{"id":"good"}"#).unwrap();
        std::fs::write(
            m.dir.join("good/panel/index.html"),
            "<!doctype html><html><head><title>x</title></head></html>",
        )
        .unwrap();
        std::fs::write(m.dir.join("secret.txt"), "no").unwrap();
        let body = |path: &str| {
            let r = m.serve(path);
            (
                r.status().as_u16(),
                String::from_utf8_lossy(r.body()).to_string(),
            )
        };
        let (status, html) = body("/good/panel/index.html");
        assert_eq!(status, 200);
        assert!(html.starts_with(
            r#"<!doctype html><html><head><script src="/_squadron/kit.js"></script><title>"#
        ));
        assert!(
            m.serve("/good/panel/index.html").headers()["Content-Security-Policy"]
                .to_str()
                .unwrap()
                .contains("connect-src 'none'")
        );
        assert!(body("/_squadron/kit.js").1.contains("window.squadron"));
        for refused in [
            "/good/../secret.txt",
            "/good/%2e%2e/secret.txt",
            "/good//etc/passwd",
            "/other/panel/index.html",
            "/good/panel",
        ] {
            assert_eq!(body(refused).0, 404, "{refused}");
        }
        m.enable("good", false).unwrap();
        assert_eq!(body("/good/panel/index.html").0, 404);
        assert_eq!(
            inject("<p>"),
            r#"<script src="/_squadron/kit.js"></script><p>"#
        );
    }

    #[test]
    fn lists_installed_and_linked_folders_with_their_manifest_or_error() {
        let m = mods();
        std::fs::create_dir_all(m.dir.join("good")).unwrap();
        std::fs::write(m.dir.join("good/mod.json"), r#"{"id":"good"}"#).unwrap();
        std::fs::create_dir_all(m.dir.join("broken")).unwrap();
        std::fs::write(m.dir.join("broken/mod.json"), "{").unwrap();
        std::fs::create_dir_all(m.dir.join(".hidden")).unwrap();
        let linked = m.dir.parent().unwrap().join("dev");
        std::fs::create_dir_all(&linked).unwrap();
        assert!(m.link(&linked.to_string_lossy()).is_err());
        std::fs::write(linked.join("mod.json"), r#"{"id":"dev"}"#).unwrap();
        m.link(&linked.to_string_lossy()).unwrap();
        m.link(&linked.to_string_lossy()).unwrap();

        let list = m.list().mods;
        assert_eq!(list.len(), 3);
        assert!(
            list[0].dir.ends_with("broken")
                && list[0]
                    .error
                    .as_deref()
                    .unwrap()
                    .starts_with("mod.json is not valid JSON")
        );
        assert_eq!(list[1].manifest.as_ref().unwrap()["id"], "good");
        assert!(list[2].linked && list[2].manifest.as_ref().unwrap()["id"] == "dev");

        m.unlink(&linked.to_string_lossy()).unwrap();
        assert_eq!(m.list().mods.len(), 2);
    }

    #[test]
    fn remembers_disabled_mods_once() {
        let m = mods();
        m.enable("lando", false).unwrap();
        m.enable("lando", false).unwrap();
        assert_eq!(m.list().disabled, vec!["lando"]);
        m.enable("lando", true).unwrap();
        assert!(m.list().disabled.is_empty());
    }

    #[test]
    fn an_approval_replaces_the_previous_one() {
        let m = mods();
        m.approve("lando", vec!["lando logs".into()]).unwrap();
        m.approve("lando", vec!["lando info".into()]).unwrap();
        assert_eq!(m.list().approved["lando"], vec!["lando info"]);
    }

    #[test]
    fn a_change_to_a_manifest_changes_the_stamp() {
        let m = mods();
        std::fs::create_dir_all(m.dir.join("a")).unwrap();
        let before = m.stamp();
        std::fs::write(m.dir.join("a/mod.json"), "{}").unwrap();
        assert_ne!(before, m.stamp());
    }
}
