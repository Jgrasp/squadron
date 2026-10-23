use serde::Serialize;
use std::path::Path;
use std::process::Command;

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[derive(Debug, PartialEq, Serialize)]
pub struct Change {
    pub path: String,
    pub status: String,
    pub add: Option<u32>,
    pub del: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Changes {
    pub from: String,
    pub now: String,
    pub files: Vec<Change>,
}

fn git(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .args(["--literal-pathspecs", "-c", "core.quotepath=false"]);
    cmd
}

fn run(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn is_rev(rev: &str) -> bool {
    rev == "HEAD" || (matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn snapshot(dir: &Path) -> Result<String, String> {
    let index = run(git(dir).args(["rev-parse", "--path-format=absolute", "--git-path", "index"]))?;
    let temp =
        std::env::temp_dir().join(format!("squadron-index-{}", uuid::Uuid::new_v4().simple()));
    let _ = std::fs::copy(index.trim(), &temp);
    let tree = run(git(dir).env("GIT_INDEX_FILE", &temp).args(["add", "-A"]))
        .and_then(|_| run(git(dir).env("GIT_INDEX_FILE", &temp).arg("write-tree")));
    let _ = std::fs::remove_file(&temp);
    Ok(tree?.trim().to_string())
}

fn head(dir: &Path) -> String {
    let found = run(git(dir).args(["rev-parse", "--verify", "--quiet", "HEAD^{tree}"])).is_ok();
    if found { "HEAD" } else { EMPTY_TREE }.to_string()
}

fn diff(dir: &Path, args: &[&str]) -> Result<String, String> {
    run(git(dir)
        .args([
            "diff",
            "--no-renames",
            "--relative",
            "--no-color",
            "--no-ext-diff",
        ])
        .args(args))
}

pub fn changes(dir: &Path) -> Result<Changes, String> {
    between(dir, head(dir), snapshot(dir)?)
}

pub fn origin(dir: &Path) -> Option<String> {
    run(git(dir).args(["remote", "get-url", "origin"]))
        .ok()
        .map(|url| url.trim().to_string())
}

fn commit(dir: &Path, rev: &str) -> Option<String> {
    run(git(dir).args([
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{rev}^{{commit}}"),
    ]))
    .ok()
    .map(|sha| sha.trim().to_string())
}

fn fetch(dir: &Path, branches: &[&str]) {
    let _ = run(git(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .args(["fetch", "--quiet", "--no-tags", "origin"])
        .args(branches));
}

#[derive(Serialize)]
pub struct Pulled {
    pub changes: Changes,
    pub ahead: u32,
    pub added: u32,
}

pub fn pull_changes(
    dir: &Path,
    base: &str,
    branch: &str,
    head: &str,
    since: Option<&str>,
) -> Result<Pulled, String> {
    let valid = |name: &str| !name.is_empty() && !name.starts_with('-');
    if !valid(base) || !valid(branch) || !head.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Unknown branch.".into());
    }
    fetch(dir, &[base, branch]);
    let head = commit(dir, head).ok_or("The pull request commit is not on this computer.")?;
    let target = commit(dir, &format!("origin/{base}"))
        .ok_or_else(|| format!("origin/{base} is not on this computer."))?;
    let mut from = run(git(dir).args(["merge-base", &target, &head]))?
        .trim()
        .to_string();
    let mut added = 0;
    if let Some(since) = since {
        let reviewed = if since.bytes().all(|b| b.is_ascii_hexdigit()) {
            commit(dir, since)
        } else if since
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-:.+".contains(&b))
        {
            run(git(dir).args([
                "rev-list",
                "-1",
                "--first-parent",
                &format!("--before={since}"),
                &head,
            ]))
            .ok()
            .map(|sha| sha.trim().to_string())
            .filter(|sha| !sha.is_empty())
        } else {
            None
        };
        let kept = reviewed
            .filter(|sha| run(git(dir).args(["merge-base", "--is-ancestor", sha, &head])).is_ok());
        let reviewed = kept.ok_or(
            "The commit you reviewed is no longer in the branch: it was rewritten. Here is the whole pull request.",
        )?;
        added = run(git(dir).args(["rev-list", "--count", &format!("{reviewed}..{head}")]))
            .ok()
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0);
        from = reviewed;
    }
    let ahead = run(git(dir).args(["rev-list", "--count", &format!("{head}..HEAD")]))
        .ok()
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(0);
    Ok(Pulled {
        changes: between(dir, from, head)?,
        ahead,
        added,
    })
}

pub fn changed_files(dir: &Path, base: &str) -> Result<Vec<String>, String> {
    if base.is_empty() || base.starts_with('-') {
        return Err("Unknown branch.".into());
    }
    let names = diff(
        dir,
        &["--name-only", "-z", &format!("origin/{base}...HEAD")],
    )?;
    Ok(names
        .split('\0')
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn review_checkout(
    project: &Path,
    into: &Path,
    branch: &str,
    base: &str,
) -> Result<(), String> {
    if [branch, base]
        .iter()
        .any(|b| b.is_empty() || b.starts_with('-'))
    {
        return Err("Unknown branch.".into());
    }
    fetch(project, &[branch, base]);
    let target = format!("origin/{branch}");
    if into.join(".git").exists() {
        run(git(into).args(["checkout", "--quiet", "--detach", &target]))?;
    } else {
        let into = into.to_string_lossy();
        run(git(project).args(["worktree", "add", "--quiet", "--detach", &into, &target]))?;
    }
    Ok(())
}

fn between(dir: &Path, from: String, now: String) -> Result<Changes, String> {
    let names = diff(dir, &["--name-status", "-z", &from, &now])?;
    let stats = diff(dir, &["--numstat", "-z", &from, &now])?;
    let mut counts = std::collections::HashMap::new();
    for record in stats.split('\0').filter(|r| !r.is_empty()) {
        let mut parts = record.splitn(3, '\t');
        let (Some(add), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        counts.insert(path, (add.parse().ok(), del.parse().ok()));
    }
    let fields: Vec<&str> = names.split('\0').collect();
    let files = fields
        .chunks(2)
        .filter(|pair| pair.len() == 2 && !pair[1].is_empty())
        .map(|pair| {
            let (add, del) = counts.get(pair[1]).copied().unwrap_or((None, None));
            Change {
                path: pair[1].to_string(),
                status: pair[0].chars().take(1).collect(),
                add,
                del,
            }
        })
        .collect();
    Ok(Changes { from, now, files })
}

pub fn show(dir: &Path, rev: &str, path: &str) -> Result<Option<String>, String> {
    if !is_rev(rev) {
        return Err("Unknown revision.".into());
    }
    Ok(run(git(dir).args(["show", &format!("{rev}:./{path}")])).ok())
}

pub fn patch(dir: &Path, from: &str, now: &str, path: &str) -> Result<String, String> {
    if !is_rev(from) || !is_rev(now) {
        return Err("Unknown revision.".into());
    }
    diff(dir, &["-U3", from, now, "--", path])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn diffs_a_linked_worktree_apart_from_the_main_checkout() {
        let root = std::env::temp_dir().join(format!("squadron-gitwt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let main = root.join("main");
        let tree = root.join("main/.worktrees/T70");
        fs::create_dir_all(&main).unwrap();
        let sh = |dir: &Path, args: &[&str]| run(git(dir).args(args)).unwrap();
        sh(&main, &["init", "-q"]);
        sh(&main, &["config", "user.email", "t@t"]);
        sh(&main, &["config", "user.name", "t"]);
        fs::write(main.join(".gitignore"), ".worktrees/\n").unwrap();
        fs::write(main.join("a.txt"), "one\n").unwrap();
        sh(&main, &["add", "-A"]);
        sh(
            &main,
            &["-c", "commit.gpgsign=false", "commit", "-qnm", "init"],
        );
        sh(
            &main,
            &[
                "worktree",
                "add",
                "-q",
                tree.to_str().unwrap(),
                "-b",
                "ia/T70",
            ],
        );
        fs::write(tree.join("a.txt"), "one\ntwo\n").unwrap();
        fs::write(tree.join("b.txt"), "new\n").unwrap();

        let paths = |dir: &Path| -> Vec<String> {
            changes(dir)
                .unwrap()
                .files
                .into_iter()
                .map(|f| f.path)
                .collect()
        };
        assert_eq!(paths(&tree), ["a.txt", "b.txt"]);
        assert!(paths(&main).is_empty());
        assert_eq!(crate::projects::branch(&tree).as_deref(), Some("ia/T70"));
    }

    #[test]
    fn diffs_everything_not_committed() {
        let dir = std::env::temp_dir().join(format!("squadron-git-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        let sh = |args: &[&str]| run(git(&dir).args(args)).unwrap();
        sh(&["init", "-q"]);
        sh(&["config", "user.email", "t@t"]);
        sh(&["config", "user.name", "t"]);
        fs::write(dir.join("src/a.php"), "one\ntwo\n").unwrap();
        fs::write(dir.join("gone.txt"), "x\n").unwrap();
        fs::write(dir.join(".gitignore"), "cache/\n").unwrap();
        sh(&["add", "-A"]);
        sh(&["-c", "commit.gpgsign=false", "commit", "-qnm", "init"]);
        let base = snapshot(&dir).unwrap();
        assert!(is_rev(&base));

        fs::write(dir.join("committed.txt"), "done\n").unwrap();
        sh(&["add", "committed.txt"]);
        sh(&[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qnm",
            "agent commit",
        ]);
        fs::write(dir.join("src/a.php"), "one\n2\nthree\n").unwrap();
        sh(&["add", "src/a.php"]);
        fs::remove_file(dir.join("gone.txt")).unwrap();
        fs::write(dir.join("new.md"), "hi\n").unwrap();
        fs::create_dir(dir.join("cache")).unwrap();
        fs::write(dir.join("cache/x"), "ignored\n").unwrap();

        let found = changes(&dir).unwrap();
        let summary: Vec<_> = found
            .files
            .iter()
            .map(|c| (c.path.as_str(), c.status.as_str(), c.add, c.del))
            .collect();
        assert_eq!(
            summary,
            [
                ("gone.txt", "D", Some(0), Some(1)),
                ("new.md", "A", Some(1), Some(0)),
                ("src/a.php", "M", Some(2), Some(1)),
            ]
        );
        let text = patch(&dir, &found.from, &found.now, "src/a.php").unwrap();
        assert!(text.contains("-two\n+2\n+three"));

        assert_eq!(
            show(&dir, &base, "src/a.php").unwrap().as_deref(),
            Some("one\ntwo\n")
        );
        assert_eq!(
            show(&dir.join("src"), &base, "a.php").unwrap().as_deref(),
            Some("one\ntwo\n")
        );
        assert_eq!(show(&dir, &base, "new.md").unwrap(), None);

        let sub = changes(&dir.join("src")).unwrap();
        assert_eq!(sub.files.len(), 1);
        assert_eq!(sub.files[0].path, "a.php");
        assert!(patch(&dir, "--output=/tmp/x", &found.now, "a").is_err());
        assert!(snapshot(&std::env::temp_dir().join("squadron-not-a-repo")).is_err());
    }
}
