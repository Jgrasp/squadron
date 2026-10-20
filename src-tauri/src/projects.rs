use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Serialize)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
    pub git: bool,
    pub branch: Option<String>,
}

impl Folder {
    pub fn of(name: String, path: &Path) -> Self {
        Self {
            name,
            path: path.to_path_buf(),
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
        log::error!("{} illisible ({e}), mis de côté en .bak", file.display());
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

fn branch(dir: &Path) -> Option<String> {
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
    fn saves_reloads_and_sets_aside_a_broken_file() {
        let dir = temp("store");
        let file = dir.join("projects.json");
        assert!(load(&file).is_empty());

        let projects = vec![Project {
            name: "bmc".into(),
            path: "/home/user/bmc".into(),
        }];
        save(&file, &projects).unwrap();
        assert_eq!(load(&file), projects);

        fs::write(&file, "{pas du json").unwrap();
        assert!(load(&file).is_empty());
        assert!(dir.join("projects.json.bak").exists());
    }
}
