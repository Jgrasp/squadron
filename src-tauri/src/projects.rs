use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl Defaults {
    pub fn is_empty(&self) -> bool {
        self.model.is_none() && self.effort.is_none() && self.mode.is_none()
    }
}

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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub runs_off: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub runs_quiet: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs_hidden: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repository: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub review_prompt: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tickets: Vec<crate::tickets::Source>,
    #[serde(default, skip_serializing_if = "Defaults::is_empty")]
    pub defaults: Defaults,
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
    pub review_prompt: Option<String>,
    pub tickets: Option<Vec<crate::tickets::Source>>,
    pub defaults: Option<Defaults>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: String,
}

pub fn load_groups(file: &Path) -> Vec<Group> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        log::error!("{} is unreadable ({e}), set aside as .bak", file.display());
        let _ = fs::rename(file, file.with_extension("json.bak"));
        Vec::new()
    })
}

pub fn save_groups(file: &Path, groups: &[Group]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(groups)?)?;
    fs::rename(tmp, file)
}

fn within(groups: &[Group], id: &str, ancestor: &str) -> bool {
    let mut at = id;
    for _ in 0..=groups.len() {
        if at == ancestor {
            return true;
        }
        match groups.iter().find(|g| g.id == at) {
            Some(group) if !group.parent.is_empty() => at = &group.parent,
            _ => return false,
        }
    }
    false
}

pub fn check_group(groups: &[Group], id: &str) -> Result<(), String> {
    if id.is_empty() || groups.iter().any(|g| g.id == id) {
        Ok(())
    } else {
        Err("Unknown group.".into())
    }
}

pub fn add_group(
    groups: &mut Vec<Group>,
    id: String,
    name: &str,
    parent: String,
) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the group a name.".into());
    }
    if id.is_empty() || groups.iter().any(|g| g.id == id) {
        return Err("This group already exists.".into());
    }
    check_group(groups, &parent)?;
    groups.push(Group {
        id,
        name: name.into(),
        parent,
    });
    Ok(())
}

pub fn edit_group(
    groups: &mut [Group],
    id: &str,
    name: Option<&str>,
    parent: Option<String>,
) -> Result<(), String> {
    if let Some(parent) = &parent {
        check_group(groups, parent)?;
        if within(groups, parent, id) {
            return Err("A group cannot go inside itself.".into());
        }
    }
    let group = groups
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or("Unknown group.")?;
    if let Some(name) = name.map(str::trim) {
        if name.is_empty() {
            return Err("The group needs a name.".into());
        }
        group.name = name.into();
    }
    if let Some(parent) = parent {
        group.parent = parent;
    }
    Ok(())
}

pub fn dissolve(groups: &mut Vec<Group>, list: &mut [Project], id: &str) -> Result<(), String> {
    let above = groups
        .iter()
        .find(|g| g.id == id)
        .ok_or("Unknown group.")?
        .parent
        .clone();
    for group in groups.iter_mut().filter(|g| g.parent == id) {
        group.parent = above.clone();
    }
    for project in list.iter_mut().filter(|p| p.group == id) {
        project.group = above.clone();
    }
    groups.retain(|g| g.id != id);
    Ok(())
}

pub fn place(
    groups: &[Group],
    list: &mut [Project],
    paths: &[PathBuf],
    group: &str,
) -> Result<(), String> {
    check_group(groups, group)?;
    for project in list.iter_mut().filter(|p| paths.contains(&p.path)) {
        project.group = group.into();
    }
    Ok(())
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<TicketLink>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TicketLink {
    pub connection: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
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
    pub group: String,
    pub runs: bool,
    pub runs_notify: bool,
    pub runs_hidden: Vec<String>,
    pub repository: String,
    pub review_prompt: String,
    pub tickets: Vec<crate::tickets::Source>,
    pub defaults: Defaults,
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
            group: project.group.clone(),
            runs: !project.runs_off,
            runs_notify: !project.runs_quiet,
            runs_hidden: project.runs_hidden.clone(),
            repository: project.repository.clone(),
            review_prompt: project.review_prompt.clone(),
            tickets: project.tickets.clone(),
            defaults: project.defaults.clone(),
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
                ticket: None,
            }],
            color: "#A78BFA".into(),
            pinned: true,
            archived: false,
            group: "g1".into(),
            runs_off: true,
            runs_quiet: false,
            runs_hidden: vec!["Copilot".into()],
            repository: "github.com/Jgrasp/squadron".into(),
            review_prompt: "Check the Sylius upgrade notes.".into(),
            tickets: Vec::new(),
            defaults: Defaults {
                model: Some("sonnet".into()),
                effort: None,
                mode: Some("plan".into()),
            },
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

    fn group(id: &str, parent: &str) -> Group {
        Group {
            id: id.into(),
            name: id.into(),
            parent: parent.into(),
        }
    }

    #[test]
    fn nests_groups_without_cycles_or_strangers() {
        let mut groups = Vec::new();
        add_group(&mut groups, "a".into(), " Clients ", String::new()).unwrap();
        add_group(&mut groups, "b".into(), "Ring", "a".into()).unwrap();
        assert_eq!(groups[0].name, "Clients");
        assert!(add_group(&mut groups, "a".into(), "Again", String::new()).is_err());
        assert!(add_group(&mut groups, "c".into(), "  ", String::new()).is_err());
        assert!(add_group(&mut groups, "c".into(), "Lost", "nope".into()).is_err());
        assert!(edit_group(&mut groups, "a", None, Some("b".into())).is_err());
        assert!(edit_group(&mut groups, "a", None, Some("a".into())).is_err());
        edit_group(&mut groups, "b", Some("Ring Inc"), Some(String::new())).unwrap();
        assert_eq!(groups[1].name, "Ring Inc");
        assert_eq!(groups[1].parent, "");
        assert!(edit_group(&mut groups, "b", Some(" "), None).is_err());
    }

    #[test]
    fn dissolving_a_group_lifts_its_content_one_level() {
        let mut groups = vec![group("a", ""), group("b", "a"), group("c", "b")];
        let project = |name: &str, group: &str| Project {
            name: name.into(),
            path: PathBuf::from(format!("/p/{name}")),
            group: group.into(),
            ..Default::default()
        };
        let mut list = vec![project("x", "b"), project("y", "a"), project("z", "")];
        dissolve(&mut groups, &mut list, "b").unwrap();
        let ids: Vec<_> = groups
            .iter()
            .map(|g| (g.id.as_str(), g.parent.as_str()))
            .collect();
        assert_eq!(ids, [("a", ""), ("c", "a")]);
        let places: Vec<_> = list.iter().map(|p| p.group.as_str()).collect();
        assert_eq!(places, ["a", "a", ""]);
        assert!(dissolve(&mut groups, &mut list, "b").is_err());
    }

    #[test]
    fn places_several_projects_in_an_existing_group_only() {
        let groups = vec![group("a", "")];
        let project = |name: &str| Project {
            path: PathBuf::from(format!("/p/{name}")),
            ..Default::default()
        };
        let mut list = vec![project("x"), project("y"), project("z")];
        place(
            &groups,
            &mut list,
            &[PathBuf::from("/p/x"), PathBuf::from("/p/z")],
            "a",
        )
        .unwrap();
        let places: Vec<_> = list.iter().map(|p| p.group.as_str()).collect();
        assert_eq!(places, ["a", "", "a"]);
        assert!(place(&groups, &mut list, &[PathBuf::from("/p/x")], "nope").is_err());
        place(&groups, &mut list, &[PathBuf::from("/p/x")], "").unwrap();
        assert_eq!(list[0].group, "");
    }

    #[test]
    fn saves_and_reloads_groups() {
        let dir = temp("groups");
        let file = dir.join("groups.json");
        assert!(load_groups(&file).is_empty());
        let groups = vec![group("a", ""), group("b", "a")];
        save_groups(&file, &groups).unwrap();
        assert_eq!(load_groups(&file), groups);
        fs::write(&file, "[oops").unwrap();
        assert!(load_groups(&file).is_empty());
        assert!(dir.join("groups.json.bak").exists());
    }
}
