use reqwest::Url;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Serialize, Debug, PartialEq)]
pub struct Preview {
    pub title: String,
    pub description: String,
    pub site: String,
    pub image: Option<String>,
    pub icon: Option<String>,
}

const LIMIT: usize = 512 * 1024;

pub async fn fetch(url: &str) -> Option<Preview> {
    let target = Url::parse(url)
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (compatible; Squadron)")
        .build()
        .ok()?;
    let mut response = client
        .get(target)
        .header("Accept", "text/html")
        .send()
        .await
        .ok()?;
    let html = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|t| t.contains("html"));
    if !response.status().is_success() || !html {
        return None;
    }
    let base = response.url().clone();
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        body.extend_from_slice(&chunk);
        if body.len() > LIMIT || find(&body, b"</head") {
            break;
        }
    }
    parse(&String::from_utf8_lossy(&body), &base)
}

fn find(body: &[u8], needle: &[u8]) -> bool {
    body.windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

pub fn parse(html: &str, base: &Url) -> Option<Preview> {
    let mut meta = HashMap::<String, String>::new();
    let mut icon = None;
    let mut title = String::new();
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while let Some(open) = lower[at..].find('<').map(|i| at + i) {
        let Some(close) = lower[open..].find('>').map(|i| open + i) else {
            break;
        };
        let tag = &html[open + 1..close];
        let name = tag
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        at = close + 1;
        match name.as_str() {
            "meta" => {
                let attrs = attributes(tag);
                let key = attrs
                    .get("property")
                    .or_else(|| attrs.get("name"))
                    .map(|k| k.to_ascii_lowercase());
                if let (Some(key), Some(content)) = (key, attrs.get("content")) {
                    meta.entry(key)
                        .or_insert_with(|| content.trim().to_string());
                }
            }
            "link" if icon.is_none() => {
                let attrs = attributes(tag);
                let rel = attrs
                    .get("rel")
                    .map(|r| r.to_ascii_lowercase())
                    .unwrap_or_default();
                if rel.split_whitespace().any(|r| r == "icon") {
                    icon = attrs.get("href").and_then(|h| base.join(h).ok());
                }
            }
            "title" if title.is_empty() => {
                if let Some(end) = lower[at..].find("</title").map(|i| at + i) {
                    title = decode(html[at..end].trim());
                    at = end;
                }
            }
            "/head" | "body" => break,
            _ => {}
        }
    }
    let pick = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| meta.get(*k).filter(|v| !v.is_empty()).cloned())
    };
    let title = pick(&["og:title", "twitter:title"]).unwrap_or(title);
    if title.is_empty() {
        return None;
    }
    let host = base
        .host_str()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string();
    Some(Preview {
        title,
        description: pick(&["og:description", "twitter:description", "description"])
            .unwrap_or_default(),
        site: pick(&["og:site_name"]).unwrap_or(host),
        image: pick(&["og:image", "og:image:url", "twitter:image"])
            .and_then(|i| base.join(&i).ok())
            .map(String::from),
        icon: icon
            .or_else(|| base.join("/favicon.ico").ok())
            .map(String::from),
    })
}

fn attributes(tag: &str) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let chars: Vec<char> = tag.chars().collect();
    let mut i = chars
        .iter()
        .position(|c| c.is_whitespace())
        .unwrap_or(chars.len());
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '/') {
            i += 1;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '/' {
            i += 1;
        }
        let name: String = chars[start..i]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let quote = chars[i];
                i += 1;
                let from = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                value = chars[from..i].iter().collect();
                i += 1;
            } else {
                let from = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                value = chars[from..i].iter().collect();
            }
        }
        if !name.is_empty() {
            found.entry(name).or_insert_with(|| decode(&value));
        }
        if i == start {
            i += 1;
        }
    }
    found
}

fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let entity = rest
            .find(';')
            .filter(|&end| end <= 10)
            .map(|end| &rest[1..end]);
        let decoded = entity.and_then(|e| match e {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => e
                .strip_prefix("#x")
                .or_else(|| e.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| e.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        });
        match (entity, decoded) {
            (Some(e), Some(c)) => {
                out.push(c);
                rest = &rest[e.len() + 2..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_open_graph_tags_and_resolves_relative_urls() {
        let html = r#"<html><head>
            <title>Fallback</title>
            <meta property="og:site_name" content="GitHub">
            <meta property='og:title' content="nektos/act: Run your GitHub Actions locally &#x1F680;">
            <meta name="description" content="Plain">
            <meta property="og:description" content="Run &amp; test &quot;locally&quot;">
            <meta property="og:image" content="/social.png" />
            <link rel="shortcut icon" href="https://github.githubassets.com/favicons/favicon.svg">
        </head><body><meta property="og:title" content="ignored"></body></html>"#;
        let base = Url::parse("https://github.com/nektos/act").unwrap();
        assert_eq!(
            parse(html, &base),
            Some(Preview {
                title: "nektos/act: Run your GitHub Actions locally 🚀".into(),
                description: "Run & test \"locally\"".into(),
                site: "GitHub".into(),
                image: Some("https://github.com/social.png".into()),
                icon: Some("https://github.githubassets.com/favicons/favicon.svg".into()),
            })
        );
    }

    #[test]
    fn falls_back_to_the_title_the_host_and_the_favicon() {
        let base = Url::parse("https://www.nektosact.com/usage/index.html").unwrap();
        let found = parse(
            "<head><TITLE>Usage guide - act</TITLE><meta name=description content=Docs></head>",
            &base,
        )
        .unwrap();
        assert_eq!(found.title, "Usage guide - act");
        assert_eq!(found.description, "Docs");
        assert_eq!(found.site, "nektosact.com");
        assert_eq!(
            found.icon.as_deref(),
            Some("https://www.nektosact.com/favicon.ico")
        );
        assert_eq!(found.image, None);
    }

    #[test]
    fn gives_nothing_without_a_title() {
        let base = Url::parse("https://example.com").unwrap();
        assert_eq!(parse("<head><meta charset=utf-8></head>", &base), None);
    }
}
