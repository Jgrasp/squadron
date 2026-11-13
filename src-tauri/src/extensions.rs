use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_MANIFEST: u64 = 256 * 1024;

pub struct Extensions {
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
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub origins: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub accounts: BTreeMap<String, Vec<Account>>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub secrets: Vec<String>,
}

#[derive(PartialEq)]
enum Kind {
    Text,
    Host,
    Secret,
}

fn account_fields(manifest: &serde_json::Value) -> Vec<(String, Kind)> {
    manifest["contributes"]["accounts"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let id = f["id"].as_str().filter(|id| valid_id(id))?;
            let kind = match &f["kind"] {
                serde_json::Value::Null => Kind::Text,
                k if k == "text" => Kind::Text,
                k if k == "host" => Kind::Host,
                _ => Kind::Secret,
            };
            Some((id.to_string(), kind))
        })
        .collect()
}

pub fn host_of(input: &str) -> Option<String> {
    let site = input.trim().trim_end_matches('/');
    let site = site
        .strip_prefix("https://")
        .or_else(|| site.strip_prefix("http://"))
        .unwrap_or(site);
    let host = site.split('/').next()?.to_ascii_lowercase();
    let label = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    (host.len() <= 253 && host.contains('.') && host.split('.').all(label)).then_some(host)
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
    pub extensions: Vec<Found>,
    pub disabled: Vec<String>,
    pub approved: std::collections::BTreeMap<String, Vec<String>>,
    pub sources: Vec<String>,
    pub origins: std::collections::BTreeMap<String, String>,
    pub accounts: BTreeMap<String, Vec<Account>>,
}

pub type Keyring<'a> = &'a dyn Fn(&str, &str, Option<&str>) -> Result<(), String>;

impl Extensions {
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
        choices
            .folders
            .iter()
            .map(|f| (PathBuf::from(f), true))
            .chain(installed.into_iter().map(|p| (p, false)))
            .collect()
    }

    pub fn list(&self) -> Listing {
        let choices = self.choices();
        Listing {
            extensions: self
                .folders(&choices)
                .into_iter()
                .map(|(dir, linked)| read(&dir, linked))
                .collect(),
            disabled: choices.disabled,
            approved: choices.approved,
            sources: choices.sources,
            origins: choices.origins,
            accounts: choices.accounts,
        }
    }

    pub fn accounts(&self, id: &str) -> Vec<Account> {
        self.choices().accounts.remove(id).unwrap_or_default()
    }

    fn manifest(&self, id: &str) -> Option<serde_json::Value> {
        self.list()
            .extensions
            .into_iter()
            .filter_map(|f| f.manifest)
            .find(|m| m["id"].as_str() == Some(id))
    }

    pub fn save_account(
        &self,
        id: &str,
        account: Option<&str>,
        input: &BTreeMap<String, String>,
        keyring: Keyring,
    ) -> Result<String, String> {
        let fields = self
            .manifest(id)
            .map(|m| account_fields(&m))
            .filter(|f| !f.is_empty())
            .ok_or("This extension has no accounts.")?;
        let mut choices = self.choices();
        let list = choices.accounts.entry(id.to_string()).or_default();
        let old = match account {
            Some(account) => Some(
                list.iter()
                    .find(|a| a.id == account)
                    .cloned()
                    .ok_or("This account no longer exists.")?,
            ),
            None => None,
        };
        let mut next = Account {
            id: old.as_ref().map_or_else(
                || uuid::Uuid::new_v4().simple().to_string()[..8].to_string(),
                |o| o.id.clone(),
            ),
            ..Default::default()
        };
        let mut secrets = Vec::new();
        for (field, kind) in &fields {
            let value = input.get(field).map(|v| v.trim()).unwrap_or_default();
            match kind {
                Kind::Secret if value.is_empty() => {
                    if !old.as_ref().is_some_and(|o| o.secrets.contains(field)) {
                        return Err(format!("Fill in the field {field}."));
                    }
                    next.secrets.push(field.clone());
                }
                Kind::Secret => {
                    secrets.push((field.clone(), value.to_string()));
                    next.secrets.push(field.clone());
                }
                _ if value.is_empty() => return Err(format!("Fill in the field {field}.")),
                Kind::Host => {
                    let host =
                        host_of(value).ok_or_else(|| format!("{value} is not a site address."))?;
                    if next.host.is_some() {
                        return Err("An account has one host field at most.".into());
                    }
                    next.values.insert(field.clone(), host.clone());
                    next.host = Some(host);
                }
                Kind::Text => {
                    next.values.insert(field.clone(), value.to_string());
                }
            }
        }
        for (field, value) in &secrets {
            keyring(&next.id, field, Some(value))?;
        }
        for gone in old.iter().flat_map(|o| &o.secrets) {
            if !next.secrets.contains(gone) {
                keyring(&next.id, gone, None)?;
            }
        }
        let account = next.id.clone();
        match list.iter_mut().find(|a| a.id == account) {
            Some(slot) => *slot = next,
            None => list.push(next),
        }
        self.save(&choices)?;
        Ok(account)
    }

    pub fn remove_account(&self, id: &str, account: &str, keyring: Keyring) -> Result<(), String> {
        let mut choices = self.choices();
        let list = choices.accounts.entry(id.to_string()).or_default();
        let Some(at) = list.iter().position(|a| a.id == account) else {
            return Ok(());
        };
        for field in &list[at].secrets {
            keyring(account, field, None)?;
        }
        list.remove(at);
        if list.is_empty() {
            choices.accounts.remove(id);
        }
        self.save(&choices)
    }

    pub fn link(&self, path: &str) -> Result<(), String> {
        if !Path::new(path).join("extension.json").is_file() {
            return Err("This folder has no extension.json.".into());
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
            let file = dir.join("extension.json");
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
            .extensions
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

pub const OFFICIAL: &str = "Jgrasp/squadron-extensions";
const MAX_ARCHIVE: usize = 50 << 20;
const MAX_UNPACKED: u64 = 200 << 20;
const MAX_FILES: usize = 2000;

pub fn valid_id(id: &str) -> bool {
    let mut bytes = id.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && id.len() <= 40
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub async fn download(url: &str, max: usize) -> Result<Vec<u8>, String> {
    let mut answer = crate::forge::client()
        .get(url)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?;
    if !answer.status().is_success() {
        return Err(format!("GitHub answered {}.", answer.status()));
    }
    let mut body = Vec::new();
    while let Some(chunk) = answer.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > max {
            return Err("The download is too large.".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub fn valid_source(source: &str) -> bool {
    let fine = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && !part.starts_with('.')
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    matches!(source.split_once('/'), Some((owner, repo)) if fine(owner) && fine(repo))
}

fn index_url(source: &str) -> String {
    format!("https://raw.githubusercontent.com/{source}/main/index.json")
}

pub async fn fetch_index(source: &str) -> Result<serde_json::Value, String> {
    let body = download(&index_url(source), 2 << 20).await?;
    serde_json::from_slice(&body).map_err(|e| format!("index.json is not valid JSON: {e}"))
}

pub async fn catalog(sources: Vec<String>) -> Vec<serde_json::Value> {
    let mut all = Vec::new();
    for source in std::iter::once(OFFICIAL.to_string()).chain(sources) {
        let official = source == OFFICIAL;
        all.push(match fetch_index(&source).await {
            Ok(index) => {
                serde_json::json!({"source": source, "official": official, "index": index})
            }
            Err(error) => {
                serde_json::json!({"source": source, "official": official, "error": error})
            }
        });
    }
    all
}

pub async fn fetch_archive(source: &str, archive: &str) -> Result<Vec<u8>, String> {
    if !archive.starts_with(&format!("https://github.com/{source}/releases/download/")) {
        return Err(format!(
            "The archive does not come from the releases of {source}."
        ));
    }
    download(archive, MAX_ARCHIVE).await
}

fn extract(archive: &[u8], into: &Path) -> Result<(), String> {
    use std::path::Component;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    std::fs::create_dir_all(into).map_err(|e| e.to_string())?;
    let (mut total, mut count) = (0u64, 0usize);
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        count += 1;
        total += entry.size();
        if count > MAX_FILES || total > MAX_UNPACKED {
            return Err("The archive is too large.".into());
        }
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if !path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err("The archive holds a path outside its folder.".into());
        }
        let file = into.join(&path);
        match entry.header().entry_type() {
            tar::EntryType::Directory => {
                std::fs::create_dir_all(&file).map_err(|e| e.to_string())?
            }
            tar::EntryType::Regular => {
                if let Some(parent) = file.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let size = entry.size();
                let mut out = std::fs::File::create(&file).map_err(|e| e.to_string())?;
                std::io::copy(&mut std::io::Read::take(&mut entry, size), &mut out)
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err("The archive holds a link or a special file.".into()),
        }
    }
    Ok(())
}

impl Extensions {
    pub fn known_source(&self, source: &str) -> bool {
        source == OFFICIAL || self.choices().sources.iter().any(|s| s == source)
    }

    pub fn add_source(&self, source: &str) -> Result<(), String> {
        if !valid_source(source) {
            return Err(format!(
                "{source} is not a GitHub repository like owner/repo."
            ));
        }
        let mut choices = self.choices();
        if !source.eq_ignore_ascii_case(OFFICIAL)
            && !choices
                .sources
                .iter()
                .any(|s| s.eq_ignore_ascii_case(source))
        {
            choices.sources.push(source.into());
        }
        self.save(&choices)
    }

    pub fn remove_source(&self, source: &str) -> Result<(), String> {
        let mut choices = self.choices();
        choices.sources.retain(|s| s != source);
        self.save(&choices)
    }

    pub fn install(
        &self,
        source: &str,
        id: &str,
        archive: &[u8],
        sha256: &str,
    ) -> Result<(), String> {
        use sha2::Digest;
        if !valid_id(id) {
            return Err(format!("{id} is not an extension id."));
        }
        if format!("{:x}", sha2::Sha256::digest(archive)) != sha256.to_ascii_lowercase() {
            return Err("The archive does not match the catalog. Nothing was installed.".into());
        }
        let staging = self.dir.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        let unpacked = extract(archive, &staging).and_then(|()| {
            match read(&staging, false)
                .manifest
                .as_ref()
                .and_then(|m| m["id"].as_str())
            {
                Some(found) if found == id => Ok(()),
                _ => Err(format!("The archive does not hold the extension {id}.")),
            }
        });
        if let Err(e) = unpacked {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        let (target, old) = (
            self.dir.join(id),
            self.dir.join(format!(".old-{}", uuid::Uuid::new_v4())),
        );
        if target.exists() {
            std::fs::rename(&target, &old).map_err(|e| e.to_string())?;
        }
        if let Err(e) = std::fs::rename(&staging, &target) {
            let _ = std::fs::rename(&old, &target);
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e.to_string());
        }
        let _ = std::fs::remove_dir_all(&old);
        let mut choices = self.choices();
        choices.origins.insert(id.into(), source.into());
        self.save(&choices)
    }

    pub fn uninstall(&self, id: &str, keyring: Keyring) -> Result<(), String> {
        let target = self.dir.join(id);
        if !valid_id(id) || target.is_symlink() || !target.is_dir() {
            return Err(format!("{id} is not an installed extension."));
        }
        for account in self.accounts(id) {
            self.remove_account(id, &account.id, keyring)?;
        }
        std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        let mut choices = self.choices();
        choices.disabled.retain(|d| d != id);
        choices.approved.remove(id);
        choices.origins.remove(id);
        self.save(&choices)
    }
}

const KIT_JS: &str = include_str!("../../extensions/kit/kit.js");
const KIT_CSS: &str = include_str!("../../extensions/kit/kit.css");
const PANEL_CSP: &str = "default-src 'self' extension: http://extension.localhost data: blob: 'unsafe-inline'; connect-src 'none'; form-action 'none'";

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
    let file = dir.join("extension.json");
    let manifest = match std::fs::metadata(&file) {
        Err(_) => Err("extension.json is missing.".to_string()),
        Ok(m) if m.len() > MAX_MANIFEST => Err("extension.json is larger than 256 KB.".to_string()),
        Ok(_) => std::fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|text| {
                serde_json::from_str(&text)
                    .map_err(|e| format!("extension.json is not valid JSON: {e}"))
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

#[derive(Deserialize)]
pub struct SkillSet {
    pub id: String,
    pub dir: String,
    pub skills: Vec<String>,
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for item in std::fs::read_dir(from)?.flatten() {
        let kind = item.file_type()?;
        if kind.is_dir() {
            copy_dir(&item.path(), &to.join(item.file_name()))?;
        } else if kind.is_file() {
            std::fs::copy(item.path(), to.join(item.file_name()))?;
        }
    }
    Ok(())
}

fn plugin_of(root: &Path, set: &SkillSet, errors: &mut Vec<String>) -> Option<PathBuf> {
    if !valid_id(&set.id) {
        return None;
    }
    let base = Path::new(&set.dir).canonicalize().ok()?;
    let plugin = root.join(&set.id);
    let _ = std::fs::remove_dir_all(&plugin);
    let mut any = false;
    for skill in &set.skills {
        let source = base
            .join(skill)
            .canonicalize()
            .ok()
            .filter(|s| s.starts_with(&base) && s.join("SKILL.md").is_file());
        let (Some(source), Some(name)) = (source.as_ref(), Path::new(skill).file_name()) else {
            errors.push(format!(
                "{}: {skill} is not a folder of the extension with a SKILL.md.",
                set.id
            ));
            continue;
        };
        match copy_dir(source, &plugin.join("skills").join(name)) {
            Ok(()) => any = true,
            Err(e) => errors.push(format!("{}: {skill}: {e}", set.id)),
        }
    }
    let manifest = plugin.join(".claude-plugin");
    (any && std::fs::create_dir_all(&manifest).is_ok()
        && std::fs::write(
            manifest.join("plugin.json"),
            serde_json::json!({ "name": set.id }).to_string(),
        )
        .is_ok())
    .then_some(plugin)
}

pub fn sync_skills(root: &Path, list: &[SkillSet]) -> (Vec<PathBuf>, Vec<String>) {
    let mut errors = Vec::new();
    let plugins: Vec<PathBuf> = list
        .iter()
        .filter_map(|set| plugin_of(root, set, &mut errors))
        .collect();
    if let Ok(read) = std::fs::read_dir(root) {
        for item in read.flatten() {
            if !plugins.contains(&item.path()) {
                let _ = std::fs::remove_dir_all(item.path());
            }
        }
    }
    (plugins, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extensions() -> Extensions {
        let root =
            std::env::temp_dir().join(format!("squadron-extensions-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("extensions")).unwrap();
        Extensions {
            dir: root.join("extensions"),
            file: root.join("extensions.json"),
        }
    }

    fn archive(entries: &[(&str, &[u8])], link: Option<&str>) -> Vec<u8> {
        let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        for (path, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, path, *data).unwrap();
        }
        if let Some(name) = link {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            tar.append_link(&mut header, name, "/etc").unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    fn sum(bytes: &[u8]) -> String {
        use sha2::Digest;
        format!("{:x}", sha2::Sha256::digest(bytes))
    }

    #[test]
    fn installs_a_checked_archive_and_replaces_the_previous_version() {
        let m = extensions();
        let good = archive(
            &[
                ("extension.json", br#"{"id":"hello","version":"1"}"#),
                ("panel/index.html", b"<p>"),
            ],
            None,
        );
        assert!(
            m.install("o/r", "hello", &good, &"0".repeat(64))
                .unwrap_err()
                .contains("does not match")
        );
        m.install("o/r", "hello", &good, &sum(&good)).unwrap();
        assert!(m.dir.join("hello/panel/index.html").is_file());

        let next = archive(
            &[("extension.json", br#"{"id":"hello","version":"2"}"#)],
            None,
        );
        m.install("o/r", "hello", &next, &sum(&next)).unwrap();
        assert!(!m.dir.join("hello/panel").exists());
        assert!(
            std::fs::read_to_string(m.dir.join("hello/extension.json"))
                .unwrap()
                .contains(r#""2""#)
        );

        let other = archive(&[("extension.json", br#"{"id":"other"}"#)], None);
        assert!(m.install("o/r", "hello", &other, &sum(&other)).is_err());
        let linked = archive(&[("extension.json", br#"{"id":"evil"}"#)], Some("home"));
        assert!(
            m.install("o/r", "evil", &linked, &sum(&linked))
                .unwrap_err()
                .contains("link")
        );
        assert!(m.install("o/r", "../x", &good, &sum(&good)).is_err());
        assert!(
            std::fs::read_dir(&m.dir)
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with('.'))
        );

        assert_eq!(m.list().origins["hello"], "o/r");
        m.approve("hello", vec!["exec:git".into()]).unwrap();
        m.uninstall("hello", &|_, _, _| Ok(())).unwrap();
        assert!(!m.list().origins.contains_key("hello"));
        assert!(!m.dir.join("hello").exists());
        assert!(!m.list().approved.contains_key("hello"));
        assert!(m.uninstall("hello", &|_, _, _| Ok(())).is_err());
        assert!(m.uninstall("..", &|_, _, _| Ok(())).is_err());
    }

    #[test]
    fn keeps_account_secrets_in_the_keyring_and_the_rest_in_the_file() {
        use std::sync::Mutex;
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("jira")).unwrap();
        std::fs::write(
            m.dir.join("jira/extension.json"),
            r#"{"id":"jira","contributes":{"accounts":{"fields":[
                {"id":"site","kind":"host"},{"id":"email"},{"id":"token","kind":"secret"},{"id":"pin","kind":"Secret"}]}}}"#,
        )
        .unwrap();
        let ring = Mutex::new(BTreeMap::<String, String>::new());
        let keyring = |account: &str, field: &str, value: Option<&str>| {
            let mut ring = ring.lock().unwrap();
            match value {
                Some(v) => ring.insert(format!("{account}.{field}"), v.to_string()),
                None => ring.remove(&format!("{account}.{field}")),
            };
            Ok(())
        };
        let input = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>()
        };
        let full = input(&[
            ("site", "https://Acme.atlassian.net/jira/"),
            ("email", "me@acme.com"),
            ("token", "t0k"),
            ("pin", "1234"),
        ]);
        let a = m.save_account("jira", None, &full, &keyring).unwrap();
        let file = std::fs::read_to_string(&m.file).unwrap();
        assert!(!file.contains("t0k") && !file.contains("1234"));
        let saved = &m.accounts("jira")[0];
        assert_eq!(saved.host.as_deref(), Some("acme.atlassian.net"));
        assert_eq!(saved.values["email"], "me@acme.com");
        assert_eq!(ring.lock().unwrap()[&format!("{a}.token")], "t0k");

        let edit = input(&[("site", "acme.atlassian.net"), ("email", "you@acme.com")]);
        m.save_account("jira", Some(&a), &edit, &keyring).unwrap();
        assert_eq!(m.accounts("jira")[0].values["email"], "you@acme.com");
        assert_eq!(ring.lock().unwrap()[&format!("{a}.token")], "t0k");
        assert!(m.save_account("jira", None, &edit, &keyring).is_err());
        let bad = input(&[
            ("site", "not a host"),
            ("email", "x"),
            ("token", "t"),
            ("pin", "p"),
        ]);
        assert!(m.save_account("jira", None, &bad, &keyring).is_err());
        assert!(m.save_account("other", None, &full, &keyring).is_err());

        let b = m.save_account("jira", None, &full, &keyring).unwrap();
        m.remove_account("jira", &b, &keyring).unwrap();
        assert_eq!(m.accounts("jira").len(), 1);
        m.uninstall("jira", &|account, field, value| {
            keyring(account, field, value)
        })
        .unwrap();
        assert!(ring.lock().unwrap().is_empty());
        assert!(m.list().accounts.is_empty());
    }

    #[test]
    fn reads_a_host_from_what_the_user_typed() {
        assert_eq!(
            host_of(" https://Acme.atlassian.net/ ").as_deref(),
            Some("acme.atlassian.net")
        );
        assert_eq!(
            host_of("jira.acme.com/secure").as_deref(),
            Some("jira.acme.com")
        );
        for bad in [
            "",
            "localhost",
            "a b.com",
            "-x.com",
            "x..com",
            "acme.com:8443",
            "user@acme.com",
        ] {
            assert_eq!(host_of(bad), None, "{bad}");
        }
    }

    #[test]
    fn keeps_the_sources_the_user_added() {
        let m = extensions();
        assert!(m.known_source(OFFICIAL) && !m.known_source("me/ext"));
        m.add_source("me/ext").unwrap();
        m.add_source("me/ext").unwrap();
        m.add_source(OFFICIAL).unwrap();
        m.add_source("jgrasp/Squadron-Extensions").unwrap();
        m.add_source("ME/EXT").unwrap();
        assert_eq!(m.list().sources, ["me/ext"]);
        assert!(m.known_source("me/ext"));
        for bad in [
            "me",
            "me/",
            "/ext",
            "me/../x",
            "me/ext/more",
            "me/.git",
            "a b/c",
        ] {
            assert!(m.add_source(bad).is_err(), "{bad}");
        }
        m.remove_source("me/ext").unwrap();
        assert!(m.list().sources.is_empty());
    }

    #[test]
    fn refuses_paths_that_leave_the_archive() {
        let mut raw = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.as_gnu_mut().unwrap().name[..9].copy_from_slice(b"../escape");
        header.set_size(1);
        header.set_cksum();
        raw.append(&header, &b"x"[..]).unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &raw.into_inner().unwrap()).unwrap();
        let bytes = gz.finish().unwrap();
        let dir = std::env::temp_dir().join(format!("squadron-unpack-{}", uuid::Uuid::new_v4()));
        assert!(extract(&bytes, &dir).unwrap_err().contains("outside"));
        assert!(!dir.parent().unwrap().join("escape").exists());
    }

    #[test]
    fn serves_the_pages_of_enabled_extensions_only() {
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("good/panel")).unwrap();
        std::fs::write(m.dir.join("good/extension.json"), r#"{"id":"good"}"#).unwrap();
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
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("good")).unwrap();
        std::fs::write(m.dir.join("good/extension.json"), r#"{"id":"good"}"#).unwrap();
        std::fs::create_dir_all(m.dir.join("broken")).unwrap();
        std::fs::write(m.dir.join("broken/extension.json"), "{").unwrap();
        std::fs::create_dir_all(m.dir.join(".hidden")).unwrap();
        let linked = m.dir.parent().unwrap().join("dev");
        std::fs::create_dir_all(&linked).unwrap();
        assert!(m.link(&linked.to_string_lossy()).is_err());
        std::fs::write(linked.join("extension.json"), r#"{"id":"dev"}"#).unwrap();
        m.link(&linked.to_string_lossy()).unwrap();
        m.link(&linked.to_string_lossy()).unwrap();

        let list = m.list().extensions;
        assert_eq!(list.len(), 3);
        assert!(list[0].linked && list[0].manifest.as_ref().unwrap()["id"] == "dev");
        assert!(
            list[1].dir.ends_with("broken")
                && list[1]
                    .error
                    .as_deref()
                    .unwrap()
                    .starts_with("extension.json is not valid JSON")
        );
        assert_eq!(list[2].manifest.as_ref().unwrap()["id"], "good");

        m.unlink(&linked.to_string_lossy()).unwrap();
        assert_eq!(m.list().extensions.len(), 2);
    }

    #[test]
    fn remembers_disabled_extensions_once() {
        let m = extensions();
        m.enable("lando", false).unwrap();
        m.enable("lando", false).unwrap();
        assert_eq!(m.list().disabled, vec!["lando"]);
        m.enable("lando", true).unwrap();
        assert!(m.list().disabled.is_empty());
    }

    #[test]
    fn an_approval_replaces_the_previous_one() {
        let m = extensions();
        m.approve("lando", vec!["lando logs".into()]).unwrap();
        m.approve("lando", vec!["lando info".into()]).unwrap();
        assert_eq!(m.list().approved["lando"], vec!["lando info"]);
    }

    #[test]
    fn a_change_to_a_manifest_changes_the_stamp() {
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("a")).unwrap();
        let before = m.stamp();
        std::fs::write(m.dir.join("a/extension.json"), "{}").unwrap();
        assert_ne!(before, m.stamp());
    }

    #[test]
    fn turns_the_skills_of_each_extension_into_a_plugin_folder() {
        let root = std::env::temp_dir().join(format!("squadron-skills-{}", uuid::Uuid::new_v4()));
        let ext = root.join("ext");
        std::fs::create_dir_all(ext.join("skills/share/scripts")).unwrap();
        std::fs::write(ext.join("skills/share/SKILL.md"), "---\nname: share\n---").unwrap();
        std::fs::write(ext.join("skills/share/scripts/deploy.sh"), "echo").unwrap();
        std::fs::create_dir_all(ext.join("empty")).unwrap();
        std::fs::create_dir_all(root.join("outside")).unwrap();
        std::fs::write(root.join("outside/SKILL.md"), "x").unwrap();
        let plugins = root.join("plugins");
        std::fs::create_dir_all(plugins.join("gone")).unwrap();
        let set = SkillSet {
            id: "team-share".into(),
            dir: ext.to_string_lossy().into(),
            skills: vec!["skills/share".into(), "empty".into(), "../outside".into()],
        };
        let (made, errors) = sync_skills(&plugins, &[set]);
        assert_eq!(made, [plugins.join("team-share")]);
        assert_eq!(errors.len(), 2);
        assert!(
            plugins
                .join("team-share/skills/share/scripts/deploy.sh")
                .is_file()
        );
        assert!(
            plugins
                .join("team-share/.claude-plugin/plugin.json")
                .is_file()
        );
        assert!(!plugins.join("gone").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
