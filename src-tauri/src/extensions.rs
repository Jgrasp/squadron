use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_MANIFEST: u64 = 256 * 1024;
const MAX_LOGO: u64 = 64 * 1024;

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
    #[serde(default)]
    pub options: BTreeMap<String, BTreeMap<String, Value>>,
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

fn option_specs(manifest: &Value) -> impl Iterator<Item = (&str, &Value)> {
    manifest["contributes"]["settings"]["options"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| Some((o["id"].as_str().filter(|id| valid_id(id))?, o)))
}

fn fits(spec: &Value, value: &Value) -> bool {
    match spec["kind"].as_str() {
        Some("toggle") => value.is_boolean(),
        Some("number") => value.as_f64().is_some_and(|n| {
            spec["min"].as_f64().is_none_or(|min| n >= min)
                && spec["max"].as_f64().is_none_or(|max| n <= max)
        }),
        Some("select") => spec["choices"]
            .as_array()
            .is_some_and(|choices| choices.iter().any(|c| c["value"] == *value)),
        Some("text") => value
            .as_str()
            .is_some_and(|t| t.len() <= 4096 && !t.chars().any(char::is_control)),
        _ => false,
    }
}

fn fallback(spec: &Value) -> Value {
    if fits(spec, &spec["default"]) {
        return spec["default"].clone();
    }
    match spec["kind"].as_str() {
        Some("toggle") => Value::Bool(false),
        Some("number") if spec["min"].is_number() => spec["min"].clone(),
        Some("number") => Value::from(0),
        Some("select") => spec["choices"][0]["value"].clone(),
        _ => Value::from(""),
    }
}

pub fn options_of(manifest: &Value, stored: &BTreeMap<String, Value>) -> Map<String, Value> {
    option_specs(manifest)
        .map(|(id, spec)| {
            let value = stored
                .get(id)
                .filter(|v| fits(spec, v))
                .cloned()
                .unwrap_or_else(|| fallback(spec));
            (id.to_string(), value)
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

#[derive(Debug, PartialEq)]
pub struct Unfit {
    pub message: String,
    pub field: Option<String>,
}

impl From<&str> for Unfit {
    fn from(message: &str) -> Self {
        Unfit {
            message: message.into(),
            field: None,
        }
    }
}

impl From<String> for Unfit {
    fn from(message: String) -> Self {
        Unfit {
            message,
            field: None,
        }
    }
}

fn existing(list: &[Account], account: Option<&str>) -> Result<Option<Account>, String> {
    account
        .map(|account| {
            list.iter()
                .find(|a| a.id == account)
                .cloned()
                .ok_or_else(|| "This account no longer exists.".to_string())
        })
        .transpose()
}

fn filled(
    fields: &[(String, Kind)],
    old: Option<&Account>,
    input: &BTreeMap<String, String>,
    fresh: impl Fn() -> String,
) -> Result<(Account, Vec<(String, String)>), Unfit> {
    let unfit = |message: String, field: &str| Unfit {
        message,
        field: Some(field.to_string()),
    };
    let mut next = Account {
        id: old.map_or_else(fresh, |o| o.id.clone()),
        ..Default::default()
    };
    let mut secrets = Vec::new();
    for (field, kind) in fields {
        let value = input.get(field).map(|v| v.trim()).unwrap_or_default();
        match kind {
            Kind::Secret if value.is_empty() => {
                if !old.is_some_and(|o| o.secrets.contains(field)) {
                    return Err(unfit(format!("Fill in the field {field}."), field));
                }
                next.secrets.push(field.clone());
            }
            Kind::Secret => {
                secrets.push((field.clone(), value.to_string()));
                next.secrets.push(field.clone());
            }
            _ if value.is_empty() => {
                return Err(unfit(format!("Fill in the field {field}."), field));
            }
            Kind::Host => {
                let host = host_of(value)
                    .ok_or_else(|| unfit(format!("{value} is not a site address."), field))?;
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
    Ok((next, secrets))
}

#[derive(Serialize, Debug)]
pub struct Found {
    pub dir: String,
    pub linked: bool,
    pub manifest: Option<serde_json::Value>,
    pub error: Option<String>,
    pub logo: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Listing {
    pub extensions: Vec<Found>,
    pub disabled: Vec<String>,
    pub approved: std::collections::BTreeMap<String, Vec<String>>,
    pub sources: Vec<String>,
    pub origins: std::collections::BTreeMap<String, String>,
    pub accounts: BTreeMap<String, Vec<Account>>,
    pub options: BTreeMap<String, Map<String, Value>>,
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
        let extensions: Vec<Found> = self
            .folders(&choices)
            .into_iter()
            .map(|(dir, linked)| read(&dir, linked))
            .collect();
        let none = BTreeMap::new();
        let options = extensions
            .iter()
            .filter_map(|f| {
                let manifest = f.manifest.as_ref()?;
                let id = manifest["id"].as_str()?;
                let values = options_of(manifest, choices.options.get(id).unwrap_or(&none));
                (!values.is_empty()).then(|| (id.to_string(), values))
            })
            .collect();
        Listing {
            extensions,
            disabled: choices.disabled,
            approved: choices.approved,
            sources: choices.sources,
            origins: choices.origins,
            accounts: choices.accounts,
            options,
        }
    }

    pub fn options(&self, id: &str) -> Map<String, Value> {
        let stored = self.choices().options.remove(id).unwrap_or_default();
        self.manifest(id)
            .map(|m| options_of(&m, &stored))
            .unwrap_or_default()
    }

    pub fn set_option(
        &self,
        id: &str,
        option: &str,
        value: Option<Value>,
    ) -> Result<Map<String, Value>, String> {
        let manifest = self
            .manifest(id)
            .ok_or_else(|| format!("{id} is not an extension of Squadron."))?;
        let spec = option_specs(&manifest)
            .find(|(o, _)| *o == option)
            .map(|(_, spec)| spec.clone())
            .ok_or_else(|| format!("This extension has no option {option}."))?;
        let mut choices = self.choices();
        let stored = choices.options.entry(id.to_string()).or_default();
        match value {
            Some(v) if !fits(&spec, &v) => {
                return Err(format!("This value does not fit the option {option}."));
            }
            Some(v) => {
                stored.insert(option.to_string(), v);
            }
            None => {
                stored.remove(option);
            }
        }
        let values = options_of(&manifest, stored);
        if stored.is_empty() {
            choices.options.remove(id);
        }
        self.save(&choices)?;
        Ok(values)
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

    pub fn field_url(&self, id: &str, field: &str) -> Option<String> {
        let manifest = self.manifest(id)?;
        let contributes = &manifest["contributes"];
        let fields = contributes["accounts"]["fields"]
            .as_array()
            .into_iter()
            .flatten();
        let secrets = contributes["secrets"].as_array().into_iter().flatten();
        let url = fields.chain(secrets).find(|f| f["id"] == field)?["url"].as_str()?;
        let rest = url.strip_prefix("https://")?;
        (!rest.is_empty() && !url.contains(char::is_whitespace)).then(|| url.to_string())
    }

    fn account_spec(&self, id: &str) -> Result<Vec<(String, Kind)>, String> {
        self.manifest(id)
            .map(|m| account_fields(&m))
            .filter(|f| !f.is_empty())
            .ok_or_else(|| "This extension has no accounts.".into())
    }

    pub fn checks_accounts(&self, id: &str) -> bool {
        self.manifest(id)
            .is_some_and(|m| m["contributes"]["accounts"]["check"] == true)
    }

    pub fn account_field_ids(&self, id: &str) -> Vec<String> {
        self.manifest(id)
            .map(|m| account_fields(&m).into_iter().map(|(f, _)| f).collect())
            .unwrap_or_default()
    }

    pub fn trial_account(
        &self,
        id: &str,
        account: Option<&str>,
        input: &BTreeMap<String, String>,
    ) -> Result<(Account, BTreeMap<String, String>), Unfit> {
        let fields = self.account_spec(id).map_err(Unfit::from)?;
        let list = self.accounts(id);
        let old = existing(&list, account).map_err(Unfit::from)?;
        let fresh = || format!("trial-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let (next, secrets) = filled(&fields, old.as_ref(), input, fresh)?;
        Ok((next, secrets.into_iter().collect()))
    }

    pub fn save_account(
        &self,
        id: &str,
        account: Option<&str>,
        input: &BTreeMap<String, String>,
        keyring: Keyring,
    ) -> Result<String, String> {
        let fields = self.account_spec(id)?;
        let mut choices = self.choices();
        let list = choices.accounts.entry(id.to_string()).or_default();
        let old = existing(list, account)?;
        let fresh = || uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let (next, secrets) = filled(&fields, old.as_ref(), input, fresh).map_err(|u| u.message)?;
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
        return Err(format!(
            "{} answered {}.",
            answer.url().host_str().unwrap_or("The server"),
            answer.status()
        ));
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

#[derive(Debug, PartialEq)]
pub enum Place {
    GitHub(String),
    GitLab(String),
    Bitbucket(String),
    File(String),
}

impl Place {
    pub fn parse(source: &str) -> Option<Place> {
        let fine = |part: &str| {
            !part.is_empty()
                && part.len() <= 100
                && !part.starts_with('.')
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        };
        if !source.contains("://") {
            let (owner, repo) = source.split_once('/')?;
            return (fine(owner) && fine(repo)).then(|| Place::GitHub(source.into()));
        }
        let url = reqwest::Url::parse(source).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return None;
        }
        let path = url.path().trim_end_matches('/');
        let parts: Vec<&str> = path.split('/').skip(1).collect();
        if parts.is_empty() || !parts.iter().all(|p| fine(p)) {
            return None;
        }
        if path.ends_with(".json") {
            return Some(Place::File(url.as_str().into()));
        }
        let repo = path.trim_end_matches(".git");
        let base = format!("https://{}{repo}", url.authority().to_ascii_lowercase());
        match host.as_str() {
            "github.com" if parts.len() == 2 => Some(Place::GitHub(repo[1..].into())),
            "bitbucket.org" if parts.len() == 2 => Some(Place::Bitbucket(base)),
            "github.com" | "bitbucket.org" => None,
            _ if parts.len() >= 2 => Some(Place::GitLab(base)),
            _ => None,
        }
    }

    fn web(&self) -> String {
        match self {
            Place::GitHub(repo) => format!("https://github.com/{repo}"),
            Place::GitLab(url) | Place::Bitbucket(url) | Place::File(url) => url.clone(),
        }
    }

    fn root(&self) -> String {
        let root = match self {
            Place::File(url) => reqwest::Url::parse(url)
                .map(|u| u.origin().ascii_serialization())
                .unwrap_or_default(),
            _ => self.web(),
        };
        format!("{root}/").to_ascii_lowercase()
    }

    fn file(&self, name: &str) -> String {
        match self {
            Place::GitHub(repo) => format!("https://raw.githubusercontent.com/{repo}/HEAD/{name}"),
            Place::GitLab(url) => format!("{url}/-/raw/HEAD/{name}"),
            Place::Bitbucket(url) => format!("{url}/raw/HEAD/{name}"),
            Place::File(url) => url.clone(),
        }
    }

    fn release(&self) -> String {
        match self {
            Place::GitHub(repo) => {
                format!("https://github.com/{repo}/releases/latest/download/catalog.json")
            }
            Place::GitLab(url) => {
                format!("{url}/-/releases/permalink/latest/downloads/catalog.json")
            }
            Place::Bitbucket(url) => format!("{url}/downloads/catalog.json"),
            Place::File(url) => url.clone(),
        }
    }
}

pub fn valid_source(source: &str) -> bool {
    Place::parse(source).is_some()
}

pub fn canonical(source: &str) -> String {
    let source = source.trim().trim_end_matches('/');
    source
        .strip_suffix(".git")
        .filter(|_| !source.ends_with(".json"))
        .unwrap_or(source)
        .into()
}

async fn fetch_json(url: &str, name: &str) -> Result<serde_json::Value, String> {
    let body = download(url, 2 << 20).await?;
    serde_json::from_slice(&body).map_err(|e| format!("{name} is not valid JSON: {e}"))
}

fn single(entry: serde_json::Value, id: Option<&str>) -> Result<serde_json::Value, String> {
    match entry["id"].as_str() {
        Some(found) if valid_id(found) && id.is_none_or(|id| id == found) => Ok(entry),
        _ => Err("The catalog.json is not the expected extension.".into()),
    }
}

async fn fetch_release(place: &Place, id: Option<&str>) -> Result<serde_json::Value, String> {
    single(fetch_json(&place.release(), "catalog.json").await?, id)
}

fn semver_newer(a: Option<&str>, b: Option<&str>) -> bool {
    let parts = |v: Option<&str>| -> Vec<u64> {
        v.unwrap_or("")
            .split('.')
            .map(|n| n.parse().unwrap_or(0))
            .collect()
    };
    parts(a) > parts(b)
}

pub fn registry_repos(registry: &serde_json::Value) -> Vec<(String, Place)> {
    registry
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(id, entry)| valid_id(id) && entry.get("path").is_none())
        .filter_map(|(id, entry)| {
            let place = Place::parse(&canonical(entry["repository"].as_str()?))?;
            (!matches!(place, Place::File(_))).then(|| (id.clone(), place))
        })
        .collect()
}

async fn fetch_registry(place: &Place) -> Result<serde_json::Value, String> {
    let file = fetch_json(&place.file("registry.json"), "registry.json").await?;
    if is_registry(&file) {
        Ok(file)
    } else {
        Err("registry.json does not list extension repositories.".into())
    }
}

pub async fn registry_repo(source: &str, id: &str) -> Option<Place> {
    let registry = fetch_registry(&Place::parse(source)?).await.ok()?;
    registry_repos(&registry)
        .into_iter()
        .find_map(|(found, place)| (found == id).then_some(place))
}

async fn from_registry(
    registry: &serde_json::Value,
    legacy: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut set = tokio::task::JoinSet::new();
    for (id, place) in registry_repos(registry) {
        set.spawn(async move { fetch_release(&place, Some(&id)).await });
    }
    let mut fresh = Vec::new();
    while let Some(done) = set.join_next().await {
        if let Ok(Ok(entry)) = done {
            fresh.push(entry);
        }
    }
    let old = legacy
        .and_then(|index| index["extensions"].as_array().cloned())
        .unwrap_or_default();
    let older = |a: &serde_json::Value, b: &serde_json::Value| {
        a["id"] == b["id"] && semver_newer(b["version"].as_str(), a["version"].as_str())
    };
    let kept: Vec<_> = fresh
        .iter()
        .filter(|f| !old.iter().any(|e| older(f, e)))
        .cloned()
        .collect();
    let mut extensions: Vec<_> = old
        .into_iter()
        .filter(|e| !kept.iter().any(|f| f["id"] == e["id"]))
        .collect();
    extensions.extend(kept);
    serde_json::json!({ "api": 1, "extensions": extensions })
}

pub async fn fetch_index(source: &str) -> Result<serde_json::Value, String> {
    let place = Place::parse(source).ok_or_else(|| not_a_source(source))?;
    if let Place::File(url) = &place {
        let file = fetch_json(url, "The file").await?;
        return if file["extensions"].is_array() {
            Ok(file)
        } else if file["id"].is_string() {
            single(file, None).map(|entry| serde_json::json!({ "api": 1, "extensions": [entry] }))
        } else if is_registry(&file) {
            Ok(from_registry(&file, None).await)
        } else {
            Err("The file is neither a registry.json, an index.json nor a catalog.json.".into())
        };
    }
    let index = place.file("index.json");
    let (registry, legacy) = tokio::join!(fetch_registry(&place), fetch_json(&index, "index.json"));
    if let Ok(registry) = registry {
        return Ok(from_registry(&registry, legacy.ok()).await);
    }
    match legacy {
        Ok(index) => Ok(index),
        Err(error) => fetch_release(&place, None)
            .await
            .map(|entry| serde_json::json!({ "api": 1, "extensions": [entry] }))
            .map_err(|_| error),
    }
}

fn is_registry(file: &serde_json::Value) -> bool {
    file.as_object()
        .is_some_and(|map| map.values().all(|e| e["repository"].is_string()))
}

pub fn not_a_source(source: &str) -> String {
    format!("{source} is neither a repository URL, a .json URL nor a GitHub owner/repo.")
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

pub fn archive_allowed(source: &Place, repo: Option<&Place>, archive: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(archive) else {
        return false;
    };
    let url = url.as_str().to_ascii_lowercase();
    url.starts_with("https://")
        && std::iter::once(source)
            .chain(repo)
            .any(|place| url.starts_with(&place.root()))
}

pub async fn fetch_archive(source: &str, id: &str, archive: &str) -> Result<Vec<u8>, String> {
    let place = Place::parse(source).ok_or_else(|| not_a_source(source))?;
    let repo = registry_repo(source, id).await;
    if !archive_allowed(&place, repo.as_ref(), archive) {
        return Err(format!(
            "The archive is not hosted next to {source} or to the repository it lists for {id}."
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
        let source = &canonical(source);
        if !valid_source(source) {
            return Err(not_a_source(source));
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
        let twin = self.folders(&self.choices()).into_iter().any(|(dir, _)| {
            dir != target && read(&dir, false).manifest.is_some_and(|m| m["id"] == id)
        });
        if twin {
            std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
            let mut choices = self.choices();
            choices.origins.remove(id);
            return self.save(&choices);
        }
        for account in self.accounts(id) {
            self.remove_account(id, &account.id, keyring)?;
        }
        std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        let mut choices = self.choices();
        choices.disabled.retain(|d| d != id);
        choices.approved.remove(id);
        choices.origins.remove(id);
        choices.options.remove(id);
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
                serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("extension.json is not valid JSON: {e}"))
            }),
    };
    let (manifest, mut error) = match manifest {
        Ok(value) => (Some(value), None),
        Err(e) => (None, Some(e)),
    };
    let logo = manifest
        .as_ref()
        .and_then(|m| m["logo"].as_str())
        .and_then(|path| logo_of(dir, path).map_err(|e| error = Some(e)).ok());
    Found {
        dir: dir.to_string_lossy().to_string(),
        linked,
        manifest,
        error,
        logo,
    }
}

fn logo_of(dir: &Path, path: &str) -> Result<String, String> {
    use base64::Engine;
    let plain = path.ends_with(".svg")
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|p| !p.is_empty() && !p.starts_with('.') && !p.contains(['\\', '\0', ':']));
    if !plain {
        return Err(format!(
            "The logo {path} is not an .svg file of the extension folder."
        ));
    }
    let root = dir.canonicalize().map_err(|e| e.to_string())?;
    let file = root
        .join(path)
        .canonicalize()
        .ok()
        .filter(|f| f.starts_with(&root) && f.is_file())
        .ok_or(format!("The logo {path} is missing."))?;
    if std::fs::metadata(&file).map_err(|e| e.to_string())?.len() > MAX_LOGO {
        return Err(format!("The logo {path} is larger than 64 KB."));
    }
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    if !String::from_utf8_lossy(&bytes).contains("<svg") {
        return Err(format!("The logo {path} is not an SVG image."));
    }
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
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
    fn keeps_option_values_that_fit_and_falls_back_on_the_defaults() {
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("jira")).unwrap();
        std::fs::write(
            m.dir.join("jira/extension.json"),
            r#"{"id":"jira","contributes":{"settings":{"options":[
                {"id":"sprint","kind":"select","default":"open","choices":[{"value":"open"},{"value":"next"}]},
                {"id":"chips","kind":"toggle","default":true},
                {"id":"limit","kind":"number","min":1,"max":100},
                {"id":"folder","kind":"text","default":"Shared"}]}}}"#,
        )
        .unwrap();
        assert_eq!(
            Value::Object(m.options("jira")),
            serde_json::json!({"sprint": "open", "chips": true, "limit": 1, "folder": "Shared"})
        );
        let values = m
            .set_option("jira", "sprint", Some(Value::from("next")))
            .unwrap();
        assert_eq!(values["sprint"], "next");
        assert!(
            m.set_option("jira", "sprint", Some(Value::from("later")))
                .is_err()
        );
        assert!(
            m.set_option("jira", "limit", Some(Value::from(500)))
                .is_err()
        );
        assert!(
            m.set_option("jira", "chips", Some(Value::from("yes")))
                .is_err()
        );
        assert!(
            m.set_option("jira", "missing", Some(Value::from(1)))
                .is_err()
        );
        m.set_option("jira", "limit", Some(Value::from(20)))
            .unwrap();
        assert_eq!(m.options("jira")["limit"], 20);
        assert_eq!(m.list().options["jira"]["limit"], 20);
        assert_eq!(m.choices().options["jira"].len(), 2);
        m.set_option("jira", "sprint", None).unwrap();
        m.set_option("jira", "limit", None).unwrap();
        assert_eq!(m.options("jira")["sprint"], "open");
        assert!(!m.choices().options.contains_key("jira"));
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

        let before = ring.lock().unwrap().clone();
        let (trial, secrets) = m
            .trial_account(
                "jira",
                Some(&a),
                &input(&[
                    ("site", "https://jira.acme.com"),
                    ("email", "x@acme.com"),
                    ("pin", "9"),
                ]),
            )
            .unwrap();
        assert_eq!(
            (trial.id.as_str(), trial.host.as_deref()),
            (a.as_str(), Some("jira.acme.com"))
        );
        assert_eq!(trial.secrets, ["token", "pin"]);
        assert_eq!(
            secrets,
            BTreeMap::from([("pin".to_string(), "9".to_string())])
        );
        let (fresh, _) = m.trial_account("jira", None, &full).unwrap();
        assert!(fresh.id.starts_with("trial-") && fresh.id != a);
        assert_eq!(
            m.trial_account("jira", None, &bad).unwrap_err(),
            Unfit {
                message: "not a host is not a site address.".into(),
                field: Some("site".into())
            }
        );
        assert_eq!(
            m.trial_account("jira", None, &edit)
                .unwrap_err()
                .field
                .as_deref(),
            Some("token")
        );
        assert_eq!(*ring.lock().unwrap(), before);
        assert_eq!(m.accounts("jira").len(), 1);
        assert!(!m.checks_accounts("jira"));
        assert_eq!(
            m.account_field_ids("jira"),
            ["site", "email", "token", "pin"]
        );

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
    fn opens_only_the_https_link_a_field_declares() {
        let m = extensions();
        std::fs::create_dir_all(m.dir.join("jira")).unwrap();
        std::fs::write(
            m.dir.join("jira/extension.json"),
            r#"{"id":"jira","contributes":{
                "accounts":{"fields":[
                    {"id":"token","kind":"secret","url":"https://id.atlassian.com/manage-profile/security/api-tokens"},
                    {"id":"email","url":"http://id.atlassian.com/manage-profile/security/api-tokens"},
                    {"id":"site","url":"https://"},
                    {"id":"pin","url":"https://a.b/x y"},
                    {"id":"code","url":"javascript:alert(1)"},
                    {"id":"name"}]},
                "secrets":[{"id":"key","url":"https://github.com/settings/tokens"},{"id":"old","url":"http://github.com/settings/tokens"}]}}"#,
        )
        .unwrap();
        assert_eq!(
            m.field_url("jira", "token").as_deref(),
            Some("https://id.atlassian.com/manage-profile/security/api-tokens")
        );
        assert_eq!(
            m.field_url("jira", "key").as_deref(),
            Some("https://github.com/settings/tokens")
        );
        for field in ["email", "site", "pin", "code", "name", "old", "missing"] {
            assert_eq!(m.field_url("jira", field), None, "{field}");
        }
        assert_eq!(m.field_url("other", "token"), None);
    }

    #[test]
    fn reads_the_logo_as_an_image_and_refuses_files_outside_the_extension() {
        let m = extensions();
        let dir = m.dir.join("jira");
        std::fs::create_dir_all(dir.join("panel")).unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M0 0h16v16H0z"/></svg>"#;
        std::fs::write(dir.join("panel/logo.svg"), svg).unwrap();
        std::fs::write(dir.join(".logo.svg"), svg).unwrap();
        std::fs::write(dir.join("logo.png.svg"), "not an image").unwrap();
        std::fs::write(
            dir.join("big.svg"),
            format!("<svg>{}</svg>", " ".repeat(65 * 1024)),
        )
        .unwrap();
        std::fs::write(m.dir.join("outside.svg"), svg).unwrap();
        let with = |logo: &str| {
            std::fs::write(
                dir.join("extension.json"),
                serde_json::json!({"id": "jira", "logo": logo}).to_string(),
            )
            .unwrap();
            read(&dir, false)
        };
        let found = with("panel/logo.svg");
        assert_eq!(found.error, None);
        let logo = found.logo.unwrap();
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(logo.strip_prefix("data:image/svg+xml;base64,").unwrap())
            .unwrap();
        assert_eq!(bytes, svg.as_bytes());
        for (bad, why) in [
            ("../outside.svg", "not an .svg file"),
            ("/etc/logo.svg", "not an .svg file"),
            (".logo.svg", "not an .svg file"),
            ("panel/logo.png", "not an .svg file"),
            ("panel/none.svg", "is missing"),
            ("big.svg", "larger than 64 KB"),
            ("logo.png.svg", "not an SVG image"),
        ] {
            let found = with(bad);
            assert_eq!(found.logo, None, "{bad}");
            assert!(
                found.error.as_deref().unwrap_or_default().contains(why),
                "{bad}: {:?}",
                found.error
            );
        }
        std::fs::write(dir.join("extension.json"), r#"{"id":"jira"}"#).unwrap();
        let found = read(&dir, false);
        assert_eq!((found.logo, found.error), (None, None));
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
    fn understands_sources_on_any_forge() {
        let gh = |r: &str| Some(Place::GitHub(r.into()));
        assert_eq!(Place::parse("me/ext"), gh("me/ext"));
        assert_eq!(Place::parse("https://github.com/Me/Ext"), gh("Me/Ext"));
        assert_eq!(
            Place::parse(&canonical("https://github.com/me/ext.git/")),
            gh("me/ext")
        );
        assert_eq!(
            Place::parse("https://GitLab.com/group/sub/ext"),
            Some(Place::GitLab("https://gitlab.com/group/sub/ext".into()))
        );
        assert_eq!(
            Place::parse("https://git.acme.com:8443/team/ext"),
            Some(Place::GitLab("https://git.acme.com:8443/team/ext".into()))
        );
        assert_eq!(
            Place::parse("https://bitbucket.org/team/ext"),
            Some(Place::Bitbucket("https://bitbucket.org/team/ext".into()))
        );
        assert_eq!(
            Place::parse("https://acme.com/squadron/index.json"),
            Some(Place::File("https://acme.com/squadron/index.json".into()))
        );
        for bad in [
            "http://gitlab.com/me/ext",
            "https://github.com/me/ext/tree/main",
            "https://bitbucket.org/me",
            "https://acme.com/ext",
            "https://user:pw@gitlab.com/me/ext",
            "https://gitlab.com/me/ext?x=1",
            "https://gitlab.com/me/%2e%2e/ext",
            "ftp://acme.com/me/ext",
        ] {
            assert_eq!(Place::parse(bad), None, "{bad}");
        }
        let lab = Place::parse("https://gitlab.com/me/ext").unwrap();
        assert_eq!(
            lab.file("registry.json"),
            "https://gitlab.com/me/ext/-/raw/HEAD/registry.json"
        );
        assert_eq!(
            lab.release(),
            "https://gitlab.com/me/ext/-/releases/permalink/latest/downloads/catalog.json"
        );
        let bb = Place::parse("https://bitbucket.org/me/ext").unwrap();
        assert_eq!(
            bb.file("index.json"),
            "https://bitbucket.org/me/ext/raw/HEAD/index.json"
        );
        assert_eq!(
            bb.release(),
            "https://bitbucket.org/me/ext/downloads/catalog.json"
        );
        assert_eq!(
            gh("me/ext").unwrap().release(),
            "https://github.com/me/ext/releases/latest/download/catalog.json"
        );
    }

    #[test]
    fn reads_the_repositories_of_a_registry() {
        let registry = serde_json::json!({
            "hello": { "repository": "https://github.com/Jgrasp/squadron-extensions", "path": "examples/hello" },
            "jira": { "repository": "https://github.com/Jgrasp/squadron-jira" },
            "git": { "repository": "https://github.com/me/ext.git/" },
            "Bad": { "repository": "https://github.com/me/x" },
            "lab": { "repository": "https://gitlab.com/me/x" },
            "bb": { "repository": "https://bitbucket.org/me/x" },
            "file": { "repository": "https://acme.com/x/catalog.json" },
        });
        let mut repos: Vec<_> = registry_repos(&registry)
            .into_iter()
            .map(|(id, place)| (id, place.web()))
            .collect();
        repos.sort();
        assert_eq!(
            repos,
            [
                ("bb".into(), "https://bitbucket.org/me/x".into()),
                ("git".into(), "https://github.com/me/ext".into()),
                (
                    "jira".into(),
                    "https://github.com/Jgrasp/squadron-jira".into()
                ),
                ("lab".into(), "https://gitlab.com/me/x".into()),
            ]
        );
    }

    #[test]
    fn takes_archives_only_from_the_source_or_the_listed_repository() {
        let place = |s: &str| Place::parse(s).unwrap();
        let official = place(OFFICIAL);
        let jira = place("Jgrasp/squadron-jira");
        let ok =
            "https://github.com/Jgrasp/squadron-jira/releases/download/v0.5.0/jira-0.5.0.tar.gz";
        assert!(archive_allowed(&official, Some(&jira), ok));
        assert!(!archive_allowed(&official, None, ok));
        assert!(archive_allowed(
            &official,
            None,
            "https://github.com/Jgrasp/squadron-extensions/releases/download/jira-0.5.0/jira-0.5.0.tar.gz"
        ));
        assert!(archive_allowed(
            &place("jgrasp/squadron-extensions"),
            Some(&place("jgrasp/squadron-jira")),
            ok
        ));
        for evil in [
            "https://github.com/Jgrasp/squadron-jira/releases/download/../../../../evil/x/a.tar.gz",
            "https://github.com/Jgrasp/squadron-jira/releases/download/%2e%2e/%2E%2E/%2e%2e/%2e%2e/evil/a.tar.gz",
            "https://github.com/Jgrasp/squadron-jira-evil/releases/download/v1/a.tar.gz",
            "http://github.com/Jgrasp/squadron-jira/releases/download/v1/a.tar.gz",
            "https://github.com.evil.com/Jgrasp/squadron-jira/a.tar.gz",
        ] {
            assert!(!archive_allowed(&official, Some(&jira), evil), "{evil}");
        }
        let lab = place("https://gitlab.com/me/ext");
        assert!(archive_allowed(
            &lab,
            None,
            "https://gitlab.com/me/ext/-/releases/v1/downloads/ext-1.0.0.tar.gz"
        ));
        assert!(!archive_allowed(
            &lab,
            None,
            "https://gitlab.com/other/x/a.tar.gz"
        ));
        let file = place("https://acme.com/squadron/index.json");
        assert!(archive_allowed(&file, None, "https://acme.com/dl/a.tar.gz"));
        assert!(!archive_allowed(
            &file,
            None,
            "https://acme.com:8443/dl/a.tar.gz"
        ));
        assert!(semver_newer(Some("0.10.0"), Some("0.9.1")));
        assert!(!semver_newer(Some("0.5.0"), Some("0.5.0")));
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
        m.add_source("https://gitlab.com/me/ext.git").unwrap();
        assert_eq!(m.list().sources, ["me/ext", "https://gitlab.com/me/ext"]);
        assert!(m.known_source("me/ext"));
        for bad in [
            "me",
            "me/",
            "/ext",
            "me/../x",
            "me/ext/more",
            "me/.git",
            "a b/c",
            "https://gitlab.com/me",
            "http://gitlab.com/me/ext",
        ] {
            assert!(m.add_source(bad).is_err(), "{bad}");
        }
        m.remove_source("me/ext").unwrap();
        m.remove_source("https://gitlab.com/me/ext").unwrap();
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
    fn uninstalling_a_copy_keeps_the_state_of_the_linked_folder_with_the_same_id() {
        let m = extensions();
        let manifest = r#"{"id":"jira","contributes":{"accounts":{"fields":[{"id":"site"}]}}}"#;
        std::fs::create_dir_all(m.dir.join("jira")).unwrap();
        std::fs::write(m.dir.join("jira/extension.json"), manifest).unwrap();
        let linked = m.dir.parent().unwrap().join("dev");
        std::fs::create_dir_all(&linked).unwrap();
        std::fs::write(linked.join("extension.json"), manifest).unwrap();
        m.link(&linked.to_string_lossy()).unwrap();
        let none = |_: &str, _: &str, _: Option<&str>| Ok(());
        let site = BTreeMap::from([("site".to_string(), "a.atlassian.net".to_string())]);
        m.save_account("jira", None, &site, &none).unwrap();
        m.enable("jira", false).unwrap();
        let mut choices = m.choices();
        choices.origins.insert("jira".into(), OFFICIAL.into());
        m.save(&choices).unwrap();

        m.uninstall("jira", &|_, _, _| panic!("keyring touched"))
            .unwrap();

        let listing = m.list();
        assert!(!m.dir.join("jira").exists());
        assert_eq!(listing.extensions.len(), 1);
        assert_eq!(listing.accounts["jira"].len(), 1);
        assert_eq!(listing.disabled, vec!["jira"]);
        assert!(listing.origins.is_empty());
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
