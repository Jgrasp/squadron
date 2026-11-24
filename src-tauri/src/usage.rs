use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::value::RawValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::UNIX_EPOCH;

const IMAGE: f64 = 1850.0;
const DENSITY: f64 = 1.77;
const BIG: f64 = 4000.0;
const KEPT_DAYS: i64 = 92;
const CACHE_VERSION: u32 = 1;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hour {
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
    pub advisor: u64,
    pub turns: u64,
    pub sources: BTreeMap<String, u64>,
    pub projects: BTreeMap<String, u64>,
    pub models: BTreeMap<String, u64>,
    pub contexts: [u64; 5],
    pub calls: BTreeMap<String, u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub title: Option<String>,
    pub tokens: u64,
    pub turns: u64,
    pub peak: u64,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Big {
    pub label: String,
    pub source: String,
    pub tokens: u64,
    pub rereads: u64,
    pub project: String,
    pub at: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileStats {
    pub size: u64,
    pub modified: u64,
    pub hours: BTreeMap<i64, Hour>,
    pub sessions: Vec<Session>,
    pub bigs: Vec<Big>,
}

#[derive(Default, Serialize, Deserialize)]
struct Cache {
    version: u32,
    files: HashMap<String, FileStats>,
}

#[derive(Debug, Default, Serialize)]
pub struct Usage {
    pub hours: Vec<(i64, Hour)>,
    pub sessions: Vec<Session>,
    pub bigs: Vec<Big>,
}

#[derive(Deserialize)]
struct Line<'a> {
    #[serde(rename = "type", default)]
    kind: &'a str,
    #[serde(default)]
    subtype: Option<&'a str>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(rename = "sessionId", default)]
    session: Option<String>,
    #[serde(rename = "isSidechain", default)]
    side: bool,
    #[serde(rename = "agentId", default)]
    agent: Option<String>,
    #[serde(rename = "isMeta", default)]
    meta: bool,
    #[serde(rename = "isCompactSummary", default)]
    compact: bool,
    #[serde(default)]
    timestamp: Option<&'a str>,
    #[serde(borrow, default)]
    message: Option<Message<'a>>,
    #[serde(borrow, default)]
    attachment: Option<Attachment<'a>>,
    #[serde(default)]
    hash: Option<String>,
    #[serde(rename = "customTitle", default)]
    custom_title: Option<String>,
    #[serde(rename = "aiTitle", default)]
    ai_title: Option<String>,
}

#[derive(Deserialize)]
struct Message<'a> {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(borrow, default)]
    content: Option<&'a RawValue>,
    #[serde(default)]
    usage: Option<Tokens>,
}

#[derive(Deserialize, Default, Clone)]
struct Tokens {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    iterations: Vec<Tokens>,
}

impl Tokens {
    fn context(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}

#[derive(Deserialize)]
struct Attachment<'a> {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(borrow, default)]
    content: Option<&'a RawValue>,
    #[serde(default)]
    stdout: Option<String>,
    #[serde(rename = "hookName", default)]
    hook: Option<String>,
}

#[derive(Deserialize)]
struct Block<'a> {
    #[serde(rename = "type", default)]
    kind: &'a str,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(borrow, default)]
    content: Option<&'a RawValue>,
}

enum Content<'a> {
    Text(String),
    Blocks(Vec<Block<'a>>),
    Other,
}

fn content(raw: Option<&RawValue>) -> Content<'_> {
    let Some(raw) = raw else {
        return Content::Other;
    };
    let s = raw.get();
    if s.starts_with('"') {
        serde_json::from_str(s)
            .map(Content::Text)
            .unwrap_or(Content::Other)
    } else if s.starts_with('[') {
        serde_json::from_str(s)
            .map(Content::Blocks)
            .unwrap_or(Content::Other)
    } else {
        Content::Other
    }
}

fn text_of(raw: Option<&RawValue>) -> String {
    match content(raw) {
        Content::Text(s) => s,
        Content::Blocks(b) => b.into_iter().filter_map(|x| x.text).collect(),
        Content::Other => String::new(),
    }
}

fn estimate(s: &str) -> f64 {
    s.chars().count() as f64 / 4.0
}

fn project(cwd: &str) -> String {
    if cwd.contains("/.cache/dev-squadron") {
        return "squadron (dev scratch)".into();
    }
    let root = cwd
        .split("/.claude/worktrees/")
        .next()
        .unwrap_or(cwd)
        .trim_end_matches('/');
    match root.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.into(),
        _ => cwd.into(),
    }
}

fn field<'v>(input: &'v Value, key: &str) -> Option<&'v str> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn source_of(name: &str, input: &Value) -> String {
    let prefixed = |s: &str, kind: &str| match s.split_once(':') {
        Some((plugin, _)) => format!("Plugin · {plugin}"),
        None => format!("{kind} · {s}"),
    };
    match name {
        "Skill" => prefixed(
            field(input, "skill")
                .or(field(input, "command"))
                .unwrap_or(""),
            "Skill",
        ),
        "Task" | "Agent" => prefixed(
            field(input, "subagent_type").unwrap_or("general-purpose"),
            "Subagent",
        ),
        _ if name.starts_with("mcp__plugin_") => {
            format!("Plugin · {}", name[12..].split('_').next().unwrap_or(""))
        }
        _ if name.starts_with("mcp__") => {
            format!("MCP · {}", name.split("__").nth(1).unwrap_or(""))
        }
        _ => format!("Tool · {name}"),
    }
}

fn label_of(name: &str, input: &Value) -> String {
    for key in [
        "command",
        "file_path",
        "url",
        "query",
        "pattern",
        "skill",
        "description",
    ] {
        if let Some(v) = field(input, key) {
            let first: String = v.lines().next().unwrap_or("").chars().take(90).collect();
            return format!("{name} · {first}");
        }
    }
    name.into()
}

fn listed_plugins(body: &str) -> Vec<(String, f64)> {
    let word = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
    };
    body.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("- ")?;
            let mut parts = rest.splitn(3, ':');
            let (plugin, skill) = (parts.next()?, parts.next()?);
            parts.next()?;
            (word(plugin) && word(skill)).then(|| (plugin.to_string(), estimate(line)))
        })
        .collect()
}

pub fn epoch(ts: &str) -> Option<i64> {
    let b = ts.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| ts.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

struct Pending {
    source: String,
    size: f64,
    label: Option<String>,
}

struct Live {
    label: String,
    source: String,
    tokens: f64,
    rereads: u64,
    project: String,
    at: i64,
}

#[derive(Default)]
struct Chain {
    held: HashMap<String, f64>,
    pending: Vec<Pending>,
    prev: Option<u64>,
    out_prev: u64,
    live: Vec<Live>,
    last_skill: Option<String>,
}

#[derive(Default)]
struct Scan {
    hours: BTreeMap<i64, (Hour, HashMap<String, f64>)>,
    sessions: BTreeMap<String, Session>,
    bigs: Vec<Big>,
}

impl Scan {
    fn hour(&mut self, at: i64) -> &mut (Hour, HashMap<String, f64>) {
        self.hours.entry(at.div_euclid(3600)).or_default()
    }

    fn close(&mut self, chain: &mut Chain) {
        for b in chain.live.drain(..) {
            if b.rereads > 1 {
                self.bigs.push(Big {
                    label: b.label,
                    source: b.source,
                    tokens: b.tokens.round() as u64,
                    rereads: b.rereads,
                    project: b.project,
                    at: b.at,
                });
            }
        }
    }
}

fn slash_plugin(text: &str) -> Option<&str> {
    let name = text.split("<command-name>/").nth(1)?.split('<').next()?;
    Some(name.split_once(':')?.0).filter(|p| !p.is_empty())
}

fn invoked(text: &str, at: i64, chain: &mut Chain, scan: &mut Scan) {
    if let Some(plugin) = slash_plugin(text) {
        let source = format!("Plugin · {plugin}");
        *scan.hour(at).0.calls.entry(source.clone()).or_default() += 1;
        chain.last_skill = Some(source);
    }
}

fn bucket(context: u64) -> usize {
    match context {
        c if c < 50_000 => 0,
        c if c < 100_000 => 1,
        c if c < 200_000 => 2,
        c if c < 500_000 => 3,
        _ => 4,
    }
}

pub fn scan<R: BufRead>(reader: R, sidechain_file: bool) -> FileStats {
    let mut scan = Scan::default();
    let mut chains: HashMap<(bool, Option<String>), Chain> = HashMap::new();
    let mut blobs: HashSet<String> = HashSet::new();
    let mut tools: HashMap<String, (String, String)> = HashMap::new();
    let mut seen: HashMap<String, u64> = HashMap::new();
    let mut session: Option<String> = None;
    let mut proj = "?".to_string();
    let mut title: (Option<String>, Option<String>) = (None, None);
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.starts_with("{\"type\":\"api-request\"")
            || line.starts_with("{\"type\":\"api-request-shape\"")
        {
            continue;
        }
        let Ok(d) = serde_json::from_str::<Line>(&line) else {
            continue;
        };
        if let Some(cwd) = d.cwd.as_deref().filter(|c| !c.is_empty()) {
            proj = project(cwd);
        }
        if session.is_none() {
            session = d.session.clone();
        }
        if let Some(t) = d.custom_title.filter(|t| !t.is_empty()) {
            title.0 = Some(t);
        }
        if let Some(t) = d.ai_title.filter(|t| !t.is_empty()) {
            title.1 = Some(t);
        }
        let key = (d.side, d.agent.clone());
        let chain = chains.entry(key).or_default();
        match d.kind {
            "system" if d.subtype == Some("compact_boundary") => {
                scan.close(chain);
                chain.held.clear();
                chain.prev = None;
                continue;
            }
            "api-request-blob" => {
                if d.hash.is_some_and(|h| blobs.insert(h)) {
                    let size = estimate(&text_of(d.message.as_ref().and_then(|m| m.content)));
                    chain.pending.push(Pending {
                        source: "Your setup (CLAUDE.md, memory, env)".into(),
                        size,
                        label: None,
                    });
                }
                continue;
            }
            "attachment" => {
                let Some(a) = d.attachment else { continue };
                let mut body = text_of(a.content);
                match a.kind.as_str() {
                    "hook_success" => {
                        let out = a.stdout.as_deref().unwrap_or("").trim();
                        if !out.is_empty() && out != "{}" {
                            body.push_str(out);
                        }
                        if !body.trim().is_empty() {
                            let hook = a
                                .hook
                                .as_deref()
                                .unwrap_or("?")
                                .split(':')
                                .next()
                                .unwrap_or("?")
                                .to_string();
                            chain.pending.push(Pending {
                                source: format!("Hooks · {hook}"),
                                size: estimate(&body),
                                label: None,
                            });
                        }
                    }
                    "skill_listing" => {
                        let mut rest = estimate(&body);
                        for (plugin, size) in listed_plugins(&body) {
                            rest -= size;
                            chain.pending.push(Pending {
                                source: format!("Plugin · {plugin}"),
                                size,
                                label: None,
                            });
                        }
                        chain.pending.push(Pending {
                            source: "Claude Code · skill listing".into(),
                            size: rest.max(0.0),
                            label: None,
                        });
                    }
                    kind if !body.trim().is_empty() => {
                        chain.pending.push(Pending {
                            source: format!("Reminders · {kind}"),
                            size: estimate(&body),
                            label: None,
                        });
                    }
                    _ => {}
                }
                continue;
            }
            "user" => {
                let Some(m) = d.message else { continue };
                if d.compact {
                    chain.pending.push(Pending {
                        source: "Compaction summary".into(),
                        size: estimate(&text_of(m.content)),
                        label: None,
                    });
                    continue;
                }
                let meta_source = |chain: &Chain| match (&chain.last_skill, d.meta) {
                    (Some(skill), true) => skill.clone(),
                    _ => "Your messages".into(),
                };
                let at = d.timestamp.and_then(epoch).unwrap_or(0);
                match content(m.content) {
                    Content::Text(s) => {
                        invoked(&s, at, chain, &mut scan);
                        let source = if d.meta && chain.last_skill.is_none() {
                            "Reminders · meta".into()
                        } else {
                            meta_source(chain)
                        };
                        chain.pending.push(Pending {
                            source,
                            size: estimate(&s),
                            label: None,
                        });
                    }
                    Content::Blocks(blocks) => {
                        for b in blocks {
                            match b.kind {
                                "tool_result" => {
                                    let (source, label) = b
                                        .tool_use_id
                                        .as_ref()
                                        .and_then(|id| tools.get(id).cloned())
                                        .unwrap_or_else(|| ("Tool · ?".into(), "?".into()));
                                    let size = match content(b.content) {
                                        Content::Text(s) => estimate(&s),
                                        Content::Blocks(inner) => inner
                                            .iter()
                                            .map(|y| {
                                                if y.kind == "image" {
                                                    IMAGE
                                                } else {
                                                    estimate(y.text.as_deref().unwrap_or(""))
                                                }
                                            })
                                            .sum(),
                                        Content::Other => 0.0,
                                    };
                                    chain.pending.push(Pending {
                                        source,
                                        size,
                                        label: Some(label),
                                    });
                                }
                                "image" => chain.pending.push(Pending {
                                    source: "Your messages".into(),
                                    size: IMAGE,
                                    label: None,
                                }),
                                "text" => {
                                    invoked(b.text.as_deref().unwrap_or(""), at, chain, &mut scan);
                                    let source = meta_source(chain);
                                    chain.pending.push(Pending {
                                        source,
                                        size: estimate(b.text.as_deref().unwrap_or("")),
                                        label: None,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    Content::Other => {}
                }
                continue;
            }
            "assistant" => {}
            _ => continue,
        }
        let Some(m) = d.message else { continue };
        let at = d.timestamp.and_then(epoch).unwrap_or(0);
        if let Content::Blocks(blocks) = content(m.content) {
            for b in blocks.into_iter().filter(|b| b.kind == "tool_use") {
                let Some(id) = b.id else { continue };
                if tools.contains_key(&id) {
                    continue;
                }
                let name = b.name.unwrap_or_else(|| "?".into());
                let input = b.input.unwrap_or(Value::Null);
                let source = source_of(&name, &input);
                if name == "Skill" {
                    chain.last_skill = Some(source.clone());
                }
                *scan.hour(at).0.calls.entry(source.clone()).or_default() += 1;
                tools.insert(id, (source, label_of(&name, &input)));
            }
        }
        let (Some(id), Some(u)) = (m.id, m.usage) else {
            continue;
        };
        if let Some(before) = seen.get_mut(&id) {
            if u.output_tokens > *before {
                let diff = u.output_tokens - *before;
                *before = u.output_tokens;
                chain.out_prev += diff;
                let (h, sources) = scan.hour(at);
                h.output += diff;
                *sources.entry("Replies · generated".into()).or_default() += diff as f64;
            }
            continue;
        }
        seen.insert(id, u.output_tokens);
        let ctx = u.context();
        let out = u.output_tokens;
        let messages: Vec<&Tokens> = u
            .iterations
            .iter()
            .filter(|x| x.kind.as_deref() == Some("message"))
            .collect();
        let advisor: u64 = u
            .iterations
            .iter()
            .filter(|x| x.kind.as_deref() != Some("message"))
            .map(|x| x.context() + x.output_tokens)
            .sum();
        let advisor_calls = u
            .iterations
            .iter()
            .filter(|x| x.kind.as_deref() == Some("advisor_message"))
            .count() as u64;
        let last = messages.last().copied();
        let ctx_last = last.map_or(ctx, Tokens::context);
        if ctx_last == 0 {
            continue;
        }
        let pending = std::mem::take(&mut chain.pending);
        match chain.prev {
            None => {
                let base = if pending
                    .first()
                    .is_some_and(|p| p.source == "Compaction summary")
                {
                    "Compaction summary"
                } else {
                    "Claude Code · system prompt, tools, MCP schemas"
                };
                let mut room = ctx_last as f64;
                for p in &pending {
                    let n = (p.size * DENSITY).min(room);
                    *chain.held.entry(p.source.clone()).or_default() += n;
                    room -= n;
                }
                *chain.held.entry(base.into()).or_default() += room;
            }
            Some(prev) => {
                if chain.out_prev > 0 {
                    *chain.held.entry("Replies".into()).or_default() += chain.out_prev as f64;
                }
                let delta = ctx_last as f64 - prev as f64 - chain.out_prev as f64;
                if delta < 0.0 {
                    let sum: f64 = chain.held.values().sum();
                    let f = if sum > 0.0 {
                        ctx_last as f64 / sum
                    } else {
                        1.0
                    };
                    chain.held.values_mut().for_each(|v| *v *= f);
                    chain.live.iter_mut().for_each(|b| b.tokens *= f);
                } else if delta > 0.0 {
                    let total: f64 = pending.iter().map(|p| p.size).sum();
                    if total <= 0.0 {
                        *chain.held.entry("Not attributed".into()).or_default() += delta;
                    } else {
                        let cap = total * DENSITY * 3.0 + 20_000.0;
                        let delta = if delta > cap {
                            *chain.held.entry("Not attributed".into()).or_default() += delta - cap;
                            cap
                        } else {
                            delta
                        };
                        for p in pending {
                            let share = delta * p.size / total;
                            *chain.held.entry(p.source.clone()).or_default() += share;
                            if let Some(label) = p.label.filter(|_| share >= BIG) {
                                chain.live.push(Live {
                                    label,
                                    source: p.source,
                                    tokens: share,
                                    rereads: 0,
                                    project: proj.clone(),
                                    at,
                                });
                            }
                        }
                    }
                }
            }
        }
        chain.prev = Some(ctx_last);
        chain.out_prev = last.map_or(out, |l| l.output_tokens);
        chain.live.iter_mut().for_each(|b| b.rereads += 1);
        let spent = ctx + out + advisor;
        let s = scan
            .sessions
            .entry(session.clone().unwrap_or_default())
            .or_insert_with(|| Session {
                start: at,
                ..Default::default()
            });
        s.tokens += spent;
        s.turns += 1;
        s.peak = s.peak.max(ctx_last);
        s.end = s.end.max(at);
        if s.project.is_empty() || s.project == "?" {
            s.project = proj.clone();
        }
        let held_sum: f64 = chain.held.values().sum();
        let f = if held_sum > 0.0 {
            ctx as f64 / held_sum
        } else {
            0.0
        };
        let held: Vec<(String, f64)> = chain.held.iter().map(|(k, v)| (k.clone(), v * f)).collect();
        let model = m.model.unwrap_or_else(|| "?".into());
        let (h, sources) = scan.hour(at);
        h.input += u.input_tokens;
        h.cache_write += u.cache_creation_input_tokens;
        h.cache_read += u.cache_read_input_tokens;
        h.output += out;
        h.advisor += advisor;
        h.turns += 1;
        *h.projects.entry(proj.clone()).or_default() += spent;
        *h.models.entry(model).or_default() += spent;
        h.contexts[bucket(ctx_last)] += spent;
        if advisor_calls > 0 {
            *h.calls.entry("Tool · advisor".into()).or_default() += advisor_calls;
        }
        if advisor > 0 {
            *sources.entry("Advisor".into()).or_default() += advisor as f64;
        }
        *sources.entry("Replies · generated".into()).or_default() += out as f64;
        for (k, v) in held {
            *sources.entry(k).or_default() += v;
        }
    }
    let mut keys: Vec<_> = chains.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(mut c) = chains.remove(&k) {
            scan.close(&mut c);
        }
    }
    let title = title.0.or(title.1);
    let mut sessions: Vec<Session> = scan
        .sessions
        .into_iter()
        .map(|(id, mut s)| {
            s.id = id;
            s.title = title.clone();
            s
        })
        .collect();
    if sidechain_file {
        sessions.iter_mut().for_each(|s| s.title = None);
    }
    FileStats {
        size: 0,
        modified: 0,
        hours: scan
            .hours
            .into_iter()
            .map(|(k, (mut h, sources))| {
                h.sources = sources
                    .into_iter()
                    .map(|(k, v)| (k, v.round() as u64))
                    .filter(|(_, v)| *v > 0)
                    .collect();
                (k, h)
            })
            .collect(),
        sessions,
        bigs: scan.bigs,
    }
}

fn transcripts(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => transcripts(&path, out),
            Ok(t) if t.is_file() && path.extension().is_some_and(|x| x == "jsonl") => {
                out.push(path)
            }
            _ => {}
        }
    }
}

fn stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((meta.len(), modified))
}

fn add_hour(into: &mut Hour, from: &Hour) {
    into.input += from.input;
    into.cache_write += from.cache_write;
    into.cache_read += from.cache_read;
    into.output += from.output;
    into.advisor += from.advisor;
    into.turns += from.turns;
    for (map, other) in [
        (&mut into.sources, &from.sources),
        (&mut into.projects, &from.projects),
        (&mut into.models, &from.models),
        (&mut into.calls, &from.calls),
    ] {
        for (k, v) in other {
            *map.entry(k.clone()).or_default() += v;
        }
    }
    for (a, b) in into.contexts.iter_mut().zip(from.contexts) {
        *a += b;
    }
}

pub fn merge(files: &HashMap<String, FileStats>, since: i64) -> Usage {
    let mut hours: BTreeMap<i64, Hour> = BTreeMap::new();
    let mut sessions: HashMap<String, Session> = HashMap::new();
    let mut bigs: Vec<Big> = Vec::new();
    for f in files.values() {
        for (k, h) in &f.hours {
            add_hour(hours.entry(*k).or_default(), h);
        }
        for s in &f.sessions {
            match sessions.get_mut(&s.id) {
                None => {
                    sessions.insert(s.id.clone(), s.clone());
                }
                Some(t) => {
                    t.tokens += s.tokens;
                    t.turns += s.turns;
                    t.peak = t.peak.max(s.peak);
                    t.start = t.start.min(s.start);
                    t.end = t.end.max(s.end);
                    if t.title.is_none() {
                        t.title.clone_from(&s.title);
                    }
                }
            }
        }
        bigs.extend(f.bigs.iter().filter(|b| b.at >= since).cloned());
    }
    bigs.sort_by_key(|b| std::cmp::Reverse(b.tokens * b.rereads));
    let mut sessions: Vec<Session> = sessions.into_values().filter(|s| s.turns > 0).collect();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.tokens));
    Usage {
        hours: hours.into_iter().collect(),
        sessions,
        bigs,
    }
}

pub fn collect(root: &Path, cache: &Path) -> Usage {
    let mut known = fs::read(cache)
        .ok()
        .and_then(|b| serde_json::from_slice::<Cache>(&b).ok())
        .filter(|c| c.version == CACHE_VERSION)
        .map(|c| c.files)
        .unwrap_or_default();
    let mut paths = Vec::new();
    transcripts(root, &mut paths);
    let stale: Vec<(PathBuf, u64, u64)> = paths
        .into_iter()
        .filter_map(|p| {
            let (size, modified) = stamp(&p)?;
            let key = p.to_string_lossy().to_string();
            let fresh = known
                .get(&key)
                .is_some_and(|f| f.size == size && f.modified == modified);
            (!fresh).then_some((p, size, modified))
        })
        .collect();
    if !stale.is_empty() {
        let next = AtomicUsize::new(0);
        let done = Mutex::new(Vec::new());
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(8);
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some((path, size, modified)) = stale.get(i) else {
                            break;
                        };
                        let Ok(file) = fs::File::open(path) else {
                            continue;
                        };
                        let side = path.components().any(|c| c.as_os_str() == "subagents");
                        let mut stats = scan(BufReader::with_capacity(1 << 20, file), side);
                        stats.size = *size;
                        stats.modified = *modified;
                        done.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push((path.to_string_lossy().to_string(), stats));
                    }
                });
            }
        });
        known.extend(done.into_inner().unwrap_or_else(|e| e.into_inner()));
        let saved = Cache {
            version: CACHE_VERSION,
            files: known,
        };
        if let Ok(bytes) = serde_json::to_vec(&saved) {
            let tmp = cache.with_extension("tmp");
            if fs::write(&tmp, bytes).is_ok() {
                let _ = fs::rename(&tmp, cache);
            }
        }
        known = saved.files;
    }
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    merge(&known, now - KEPT_DAYS * 86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lines: &[&str]) -> FileStats {
        scan(lines.join("\n").as_bytes(), false)
    }

    fn total(h: &Hour) -> u64 {
        h.input + h.cache_write + h.cache_read + h.output + h.advisor
    }

    const START: &str = r#"{"type":"api-request-blob","hash":"a","message":{"role":"user","content":[{"type":"text","text":"0123456789012345678901234567890123456789"}]}}"#;

    fn turn(id: &str, ts: &str, read: u64, write: u64, out: u64, content: &str) -> String {
        format!(
            r#"{{"type":"assistant","sessionId":"s","cwd":"/home/q/Projects/app/.claude/worktrees/x","timestamp":"{ts}","message":{{"id":"{id}","model":"m","content":{content},"usage":{{"input_tokens":0,"cache_creation_input_tokens":{write},"cache_read_input_tokens":{read},"output_tokens":{out}}}}}}}"#
        )
    }

    #[test]
    fn parses_timestamps() {
        assert_eq!(epoch("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(epoch("2026-11-24T10:30:15.123Z"), Some(1_795_516_215));
        assert_eq!(epoch("bad"), None);
    }

    #[test]
    fn attributes_growth_to_what_was_added() {
        let first = turn(
            "m1",
            "2026-11-24T10:00:00Z",
            0,
            10_000,
            100,
            r#"[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"npm test\nmore"}}]"#,
        );
        let result = format!(
            r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t1","content":"{}"}}]}}}}"#,
            "x".repeat(20_000)
        );
        let second = turn("m2", "2026-11-24T10:01:00Z", 10_000, 9_100, 50, "[]");
        let third = turn("m3", "2026-11-24T10:02:00Z", 19_100, 50, 50, "[]");
        let stats = run(&[START, &first, &result, &second, &third]);
        let h = stats.hours.values().next().unwrap();
        assert_eq!(h.turns, 3);
        assert_eq!(h.projects.keys().collect::<Vec<_>>(), ["app"]);
        assert_eq!(h.calls.get("Tool · Bash"), Some(&1));
        let sum: u64 = h.sources.values().sum();
        assert!(sum.abs_diff(total(h)) <= 3, "{sum} vs {}", total(h));
        assert_eq!(h.sources.get("Tool · Bash"), Some(&18_000));
        assert_eq!(stats.bigs.len(), 1);
        assert_eq!(stats.bigs[0].label, "Bash · npm test");
        assert_eq!(stats.bigs[0].rereads, 2);
        assert_eq!(stats.sessions[0].peak, 19_150);
    }

    #[test]
    fn counts_advisor_and_skips_empty_and_duplicate_turns() {
        let first = turn("m1", "2026-11-24T10:00:00Z", 0, 10_000, 100, "[]");
        let again = turn("m1", "2026-11-24T10:00:00Z", 0, 10_000, 160, "[]");
        let empty = turn("m0", "2026-11-24T10:00:30Z", 0, 0, 0, "[]");
        let advised = r#"{"type":"assistant","sessionId":"s","timestamp":"2026-11-24T10:01:00Z","message":{"id":"m2","content":[],"usage":{"input_tokens":0,"cache_creation_input_tokens":200,"cache_read_input_tokens":20160,"output_tokens":30,"iterations":[{"type":"message","input_tokens":0,"cache_read_input_tokens":10000,"cache_creation_input_tokens":80,"output_tokens":10},{"type":"advisor_message","input_tokens":10100,"output_tokens":500},{"type":"message","input_tokens":0,"cache_read_input_tokens":10160,"cache_creation_input_tokens":120,"output_tokens":20}]}}}"#;
        let stats = run(&[START, &first, &again, &empty, advised]);
        let h = stats.hours.values().next().unwrap();
        assert_eq!(h.turns, 2);
        assert_eq!(h.output, 190);
        assert_eq!(h.advisor, 10_600);
        assert_eq!(h.sources.get("Advisor"), Some(&10_600));
        assert_eq!(h.calls.get("Tool · advisor"), Some(&1));
        assert!(h.sources.contains_key("Not attributed"));
        assert_eq!(h.contexts[0] + 60, total(h));
    }

    #[test]
    fn compaction_and_sidechains_restart_their_own_chain() {
        let main1 = turn("m1", "2026-11-24T10:00:00Z", 0, 50_000, 10, "[]");
        let side = r#"{"type":"assistant","isSidechain":true,"agentId":"a1","sessionId":"s","timestamp":"2026-11-24T10:00:10Z","message":{"id":"x1","content":[],"usage":{"input_tokens":0,"cache_creation_input_tokens":5000,"cache_read_input_tokens":0,"output_tokens":10}}}"#;
        let main2 = turn("m2", "2026-11-24T10:00:20Z", 50_000, 10, 10, "[]");
        let boundary = r#"{"type":"system","subtype":"compact_boundary"}"#;
        let summary = r#"{"type":"user","isCompactSummary":true,"message":{"content":"summary of everything"}}"#;
        let main3 = turn("m3", "2026-11-24T10:01:00Z", 0, 8_000, 10, "[]");
        let stats = run(&[&main1, side, &main2, boundary, summary, &main3]);
        let h = stats.hours.values().next().unwrap();
        assert!(!h.sources.contains_key("Not attributed"));
        assert!(h.sources.get("Compaction summary").copied().unwrap_or(0) > 7_000);
    }

    #[test]
    fn counts_plugins_invoked_by_slash_command() {
        let first = turn("m1", "2026-11-24T10:00:00Z", 0, 10_000, 10, "[]");
        let command = r#"{"type":"user","timestamp":"2026-11-24T10:00:30Z","message":{"content":"<command-message>humanizer</command-message>\n<command-name>/humanizer:humanizer</command-name>"}}"#;
        let body = format!(
            r#"{{"type":"user","isMeta":true,"timestamp":"2026-11-24T10:00:31Z","message":{{"content":[{{"type":"text","text":"{}"}}]}}}}"#,
            "y".repeat(8000)
        );
        let second = turn("m2", "2026-11-24T10:01:00Z", 10_000, 3_600, 10, "[]");
        let stats = run(&[&first, command, &body, &second]);
        let h = stats.hours.values().next().unwrap();
        assert_eq!(h.calls.get("Plugin · humanizer"), Some(&1));
        assert!(h.sources.get("Plugin · humanizer").copied().unwrap_or(0) > 3_000);
        assert_eq!(slash_plugin("<command-name>/run</command-name>"), None);
    }

    #[test]
    fn names_sources_like_the_context_panel() {
        let v = |s: &str| serde_json::from_str::<Value>(s).unwrap();
        assert_eq!(
            source_of("Skill", &v(r#"{"skill":"ponytail:ponytail"}"#)),
            "Plugin · ponytail"
        );
        assert_eq!(source_of("Skill", &v(r#"{"skill":"run"}"#)), "Skill · run");
        assert_eq!(source_of("Agent", &v("{}")), "Subagent · general-purpose");
        assert_eq!(
            source_of("mcp__plugin_crew_tools__x", &v("{}")),
            "Plugin · crew"
        );
        assert_eq!(
            source_of("mcp__squadron-pilot__overview", &v("{}")),
            "MCP · squadron-pilot"
        );
        assert_eq!(
            listed_plugins("- a:b: desc\n- solo: desc\n- p-x:s.y: d"),
            vec![("a".into(), 2.75), ("p-x".into(), 3.0)]
        );
        assert_eq!(
            project("/home/q/.cache/dev-squadron/x"),
            "squadron (dev scratch)"
        );
    }

    #[test]
    fn keeps_files_that_disappeared() {
        let dir = std::env::temp_dir().join(format!("usage-test-{}", std::process::id()));
        let root = dir.join("projects");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("s.jsonl");
        fs::write(
            &file,
            [
                START,
                &turn("m1", "2026-11-24T10:00:00Z", 0, 1000, 10, "[]"),
            ]
            .join("\n"),
        )
        .unwrap();
        let cache = dir.join("usage.json");
        assert_eq!(collect(&root, &cache).hours.len(), 1);
        fs::remove_file(&file).unwrap();
        assert_eq!(collect(&root, &cache).hours.len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
