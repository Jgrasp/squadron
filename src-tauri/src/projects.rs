use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub cards: Vec<Card>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub runs_off: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub runs_quiet: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs_hidden: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repository: String,
}

#[derive(Deserialize)]
pub struct Edit {
    pub name: Option<String>,
    pub color: Option<String>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
    pub runs: Option<bool>,
    pub runs_notify: Option<bool>,
    pub runs_hidden: Option<Vec<String>>,
    pub repository: Option<String>,
}

pub fn reorder(list: &mut [Project], paths: &[PathBuf]) {
    list.sort_by_key(|p| {
        paths
            .iter()
            .position(|q| *q == p.path)
            .unwrap_or(usize::MAX)
    });
}

pub const COLORS: [&str; 6] = [
    "#A78BFA", "#2DD4BF", "#F472B6", "#60A5FA", "#A3E635", "#FACC15",
];

pub fn paint(list: &mut [Project]) -> bool {
    let mut changed = false;
    for i in 0..list.len() {
        if !list[i].color.is_empty() {
            continue;
        }
        let used = |c: &str| list.iter().filter(|p| p.color == c).count();
        let color = COLORS.iter().min_by_key(|c| used(c)).unwrap().to_string();
        list[i].color = color;
        changed = true;
    }
    changed
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    Todo,
    Progress,
    Review,
    Done,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    #[serde(default)]
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<PathBuf>,
    pub column: Column,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub auto: Option<Column>,
}

#[derive(Serialize)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub rules: Vec<String>,
    pub cards: Vec<Card>,
    pub color: String,
    pub pinned: bool,
    pub archived: bool,
    pub runs: bool,
    pub runs_notify: bool,
    pub runs_hidden: Vec<String>,
    pub repository: String,
    pub exists: bool,
    pub git: bool,
    pub branch: Option<String>,
}

impl Folder {
    pub fn of(project: &Project) -> Self {
        let path = &project.path;
        Self {
            name: project.name.clone(),
            path: path.clone(),
            rules: project.rules.clone(),
            cards: project.cards.clone(),
            color: project.color.clone(),
            pinned: project.pinned,
            archived: project.archived,
            runs: !project.runs_off,
            runs_notify: !project.runs_quiet,
            runs_hidden: project.runs_hidden.clone(),
            repository: project.repository.clone(),
            exists: path.is_dir(),
            git: path.join(".git").exists(),
            branch: branch(path),
        }
    }
}

pub fn load(file: &Path) -> Vec<Project> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        log::error!("{} is unreadable ({e}), set aside as .bak", file.display());
        let _ = fs::rename(file, file.with_extension("json.bak"));
        Vec::new()
    })
}

pub fn save(file: &Path, projects: &[Project]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(projects)?)?;
    fs::rename(tmp, file)
}

pub fn same_folder(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

pub fn near(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return None;
    }
    let mut base = path.parent()?;
    let mut missing = path.file_name()?;
    while !base.is_dir() {
        missing = base.file_name()?;
        base = base.parent()?;
    }
    let wanted = missing.to_string_lossy().to_lowercase();
    let distance = |name: &str| {
        let (a, b): (Vec<char>, Vec<char>) = (
            wanted.chars().collect(),
            name.to_lowercase().chars().collect(),
        );
        let mut row: Vec<usize> = (0..=b.len()).collect();
        for (i, ca) in a.iter().enumerate() {
            let mut previous = row[0];
            row[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let current = row[j + 1];
                row[j + 1] = (previous + usize::from(ca != cb))
                    .min(row[j] + 1)
                    .min(current + 1);
                previous = current;
            }
        }
        row[b.len()]
    };
    let closest = fs::read_dir(base)
        .ok()?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (distance(&name), entry.path())
        })
        .min_by_key(|(d, _)| *d)
        .filter(|(d, _)| *d <= 2.max(wanted.chars().count() / 3))
        .map(|(_, p)| p);
    closest.or_else(|| (base.parent().is_some()).then(|| base.to_path_buf()))
}

pub fn inside(project: &Path, file: &Path) -> Option<PathBuf> {
    let root = fs::canonicalize(project).ok()?;
    let file = project.join(file);
    let resolved = match fs::canonicalize(&file) {
        Ok(path) => path,
        Err(_) => fs::canonicalize(file.parent()?)
            .ok()?
            .join(file.file_name()?),
    };
    resolved.starts_with(&root).then_some(resolved)
}

pub fn branch(dir: &Path) -> Option<String> {
    let dot_git = dir.join(".git");
    let git_dir = if dot_git.is_file() {
        let link = fs::read_to_string(&dot_git).ok()?;
        dir.join(link.strip_prefix("gitdir:")?.trim())
    } else {
        dot_git
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    Some(match head.strip_prefix("ref: refs/heads/") {
        Some(name) => name.to_string(),
        None => head.chars().take(7).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("squadron-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reorders_projects_and_keeps_unknown_ones_last() {
        let project = |name: &str| Project {
            name: name.into(),
            path: PathBuf::from(format!("/p/{name}")),
            ..Default::default()
        };
        let mut list = vec![project("a"), project("b"), project("c")];
        reorder(&mut list, &[PathBuf::from("/p/c"), PathBuf::from("/p/a")]);
        let names: Vec<_> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["c", "a", "b"]);
    }

    #[test]
    fn suggests_the_closest_existing_folder() {
        let dir = temp("near");
        fs::create_dir_all(dir.join("agence/hermes")).unwrap();
        fs::create_dir_all(dir.join("agence/orbit")).unwrap();
        assert_eq!(
            near(&dir.join("agence/herms")),
            Some(dir.join("agence/hermes"))
        );
        assert_eq!(
            near(&dir.join("agence/Hermes2/src")),
            Some(dir.join("agence/hermes"))
        );
        assert_eq!(
            near(&dir.join("agence/something-else")),
            Some(dir.join("agence"))
        );
        assert_eq!(near(&dir.join("agence/orbit")), None);
    }

    #[test]
    fn reads_the_branch_of_a_repo_and_of_a_worktree() {
        let repo = temp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feat/T15\n").unwrap();
        assert_eq!(branch(&repo).as_deref(), Some("feat/T15"));

        let worktree = temp("worktree");
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", repo.join(".git").display()),
        )
        .unwrap();
        assert_eq!(branch(&worktree).as_deref(), Some("feat/T15"));

        fs::write(repo.join(".git/HEAD"), "4eb5853abcdef\n").unwrap();
        assert_eq!(branch(&repo).as_deref(), Some("4eb5853"));

        assert_eq!(branch(&temp("plain")), None);
    }

    #[test]
    fn keeps_file_access_inside_the_project() {
        let root = temp("inside");
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/a.php"), "").unwrap();
        assert!(inside(&root, Path::new("src/a.php")).is_some());
        assert!(inside(&root, &root.join("src/new.php")).is_some());
        assert!(inside(&root, Path::new("../escape.txt")).is_none());
        assert!(inside(&root, Path::new("/etc/passwd")).is_none());
        std::os::unix::fs::symlink("/etc", root.join("src/etc")).unwrap();
        assert!(inside(&root, Path::new("src/etc/passwd")).is_none());
    }

    #[test]
    fn gives_each_project_the_least_used_color() {
        let project = |color: &str| Project {
            color: color.into(),
            ..Default::default()
        };
        let mut list = vec![project(""), project(COLORS[0]), project("")];
        assert!(paint(&mut list));
        let colors: Vec<_> = list.iter().map(|p| p.color.as_str()).collect();
        assert_eq!(colors, [COLORS[1], COLORS[0], COLORS[2]]);
        assert!(!paint(&mut list));
    }

    #[test]
    fn saves_reloads_and_sets_aside_a_broken_file() {
        let dir = temp("store");
        let file = dir.join("projects.json");
        assert!(load(&file).is_empty());

        let projects = vec![Project {
            name: "orbit".into(),
            path: "/home/user/orbit".into(),
            rules: vec!["Bash(git push:*)".into()],
            cards: vec![Card {
                id: "c1".into(),
                key: "T16".into(),
                title: "Segment export".into(),
                prompt: String::new(),
                goal: "the export tests pass".into(),
                files: Vec::new(),
                column: Column::Review,
                agent: Some("a1".into()),
                auto: Some(Column::Review),
            }],
            color: "#A78BFA".into(),
            pinned: true,
            archived: false,
            runs_off: true,
            runs_quiet: false,
            runs_hidden: vec!["Copilot".into()],
            repository: "github.com/Jgrasp/squadron".into(),
        }];
        save(&file, &projects).unwrap();
        assert!(
            fs::read_to_string(&file)
                .unwrap()
                .contains(r#""column": "review""#)
        );
        assert_eq!(load(&file), projects);

        fs::write(&file, "{not json").unwrap();
        assert!(load(&file).is_empty());
        assert!(dir.join("projects.json.bak").exists());
    }
}
