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
    fn a_change_to_a_manifest_changes_the_stamp() {
        let m = mods();
        std::fs::create_dir_all(m.dir.join("a")).unwrap();
        let before = m.stamp();
        std::fs::write(m.dir.join("a/mod.json"), "{}").unwrap();
        assert_ne!(before, m.stamp());
    }
}
