use serde::Serialize;
use std::path::Path;
use std::process::Command;

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
const WALK_LIMIT: usize = 20_000;
const GREP_LIMIT: usize = 500;
const WALK_SKIPPED: [&str; 9] = [
    "node_modules",
    "vendor",
    "bower_components",
    "venv",
    "__pycache__",
    "target",
    "dist",
    "build",
    "out",
];

#[derive(Debug, PartialEq, Serialize)]
pub struct Change {
    pub path: String,
    pub status: String,
    pub add: Option<u32>,
    pub del: Option<u32>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Hit {
    pub path: String,
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Changes {
    pub from: String,
    pub now: String,
    pub files: Vec<Change>,
}

fn git(dir: &Path) -> Command {
    let mut cmd = crate::process::command("git");
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

fn walk(dir: &Path) -> Result<Vec<String>, String> {
    std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    let mut paths = Vec::new();
    let mut queue = std::collections::VecDeque::from([String::new()]);
    'walk: while let Some(parent) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(dir.join(&parent)) else {
            continue;
        };
        for entry in entries.flatten() {
            if paths.len() >= WALK_LIMIT {
                break 'walk;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                paths.push(format!("{parent}{name}"));
            } else if !name.starts_with('.') && !WALK_SKIPPED.contains(&name.as_str()) {
                let path = format!("{parent}{name}/");
                paths.push(path.clone());
                queue.push_back(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn files(dir: &Path) -> Result<Vec<String>, String> {
    let Ok(out) = run(git(dir).args([
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "--deduplicate",
    ])) else {
        return walk(dir);
    };
    let mut paths = std::collections::BTreeSet::new();
    for file in out.lines().filter(|l| !l.is_empty()) {
        let mut end = 0;
        while let Some(i) = file[end..].find('/') {
            end += i + 1;
            paths.insert(file[..end].to_string());
        }
        paths.insert(file.to_string());
    }
    Ok(paths.into_iter().collect())
}

pub fn grep(
    dir: &Path,
    query: &str,
    case: bool,
    word: bool,
    regex: bool,
) -> Result<Vec<Hit>, String> {
    use std::io::{BufRead, Read};
    let mut scope = "--untracked";
    loop {
        let mut cmd = git(dir);
        cmd.args(["grep", "-n", "-I", "-z", "--no-color", scope])
            .arg(if regex { "-E" } else { "-F" });
        if !case {
            cmd.arg("-i");
        }
        if word {
            cmd.arg("-w");
        }
        let mut child = cmd
            .args(["-e", query, "--"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut hits = Vec::new();
        let out = std::io::BufReader::new(child.stdout.take().ok_or("No output.")?);
        for raw in out.split(b'\n').map_while(Result::ok) {
            let raw = String::from_utf8_lossy(&raw);
            let mut parts = raw.splitn(3, '\0');
            if let (Some(path), Some(Ok(line)), Some(text)) =
                (parts.next(), parts.next().map(str::parse), parts.next())
            {
                let text = text.chars().take(300).collect();
                hits.push(Hit {
                    path: path.into(),
                    line,
                    text,
                });
            }
            if hits.len() >= GREP_LIMIT {
                let _ = child.kill();
                break;
            }
        }
        let mut err = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut err);
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() || status.code() == Some(1) || hits.len() >= GREP_LIMIT {
            return Ok(hits);
        }
        if scope == "--untracked" && err.contains("not a git repository") {
            scope = "--no-index";
            continue;
        }
        return Err(err.trim().to_string());
    }
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

#[derive(Serialize)]
pub struct Branched {
    pub changes: Changes,
    pub base: String,
    pub commits: u32,
}

fn base_branch(dir: &Path) -> Option<String> {
    run(git(dir).args([
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ]))
    .ok()
    .map(|name| name.trim().to_string())
    .filter(|name| commit(dir, name).is_some())
    .or_else(|| {
        ["origin/main", "main", "origin/master", "master"]
            .into_iter()
            .find(|name| commit(dir, name).is_some())
            .map(str::to_string)
    })
}

pub fn branch_changes(dir: &Path) -> Result<Branched, String> {
    let base = base_branch(dir).ok_or("No main branch to compare with.")?;
    let from = run(git(dir).args(["merge-base", &base, "HEAD"]))?
        .trim()
        .to_string();
    let commits = run(git(dir).args(["rev-list", "--count", &format!("{from}..HEAD")]))
        .ok()
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(0);
    Ok(Branched {
        changes: between(dir, from, snapshot(dir)?)?,
        base: base.trim_start_matches("origin/").to_string(),
        commits,
    })
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
    fn files_of_a_folder_without_git_skip_hidden_dependency_and_build_folders() {
        let dir = std::env::temp_dir().join(format!("squadron-walk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for sub in [
            "src/lib",
            ".git",
            ".cache",
            "node_modules/a",
            "vendor",
            "target",
        ] {
            fs::create_dir_all(dir.join(sub)).unwrap();
            fs::write(dir.join(sub).join("x.txt"), "x").unwrap();
        }
        fs::write(dir.join("README.md"), "x").unwrap();
        fs::write(dir.join(".env"), "x").unwrap();
        assert_eq!(
            files(&dir).unwrap(),
            [".env", "README.md", "src/", "src/lib/", "src/lib/x.txt"]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn grep_finds_lines_in_tracked_and_untracked_files_and_outside_git() {
        let dir = std::env::temp_dir().join(format!("squadron-grep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/a.ts"), "one\nopenFile(x)\n").unwrap();
        fs::write(dir.join("b.txt"), "OpenFiles\n").unwrap();
        let hit = |path: &str, line, text: &str| Hit {
            path: path.into(),
            line,
            text: text.into(),
        };
        assert_eq!(
            grep(&dir, "openfile", false, false, false).unwrap(),
            [
                hit("b.txt", 1, "OpenFiles"),
                hit("src/a.ts", 2, "openFile(x)")
            ]
        );
        run(git(&dir).args(["init", "-q"])).unwrap();
        fs::write(dir.join(".gitignore"), "b.txt\n").unwrap();
        assert_eq!(
            grep(&dir, "open.ile\\(", true, false, true).unwrap(),
            [hit("src/a.ts", 2, "openFile(x)")]
        );
        assert!(grep(&dir, "openFile", true, true, false).unwrap().len() == 1);
        assert!(
            grep(&dir, "nothing", true, false, false)
                .unwrap()
                .is_empty()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn branch_changes_cover_commits_and_uncommitted_work_since_main() {
        let dir = std::env::temp_dir().join(format!("squadron-branch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let sh = |args: &[&str]| run(git(&dir).args(args)).unwrap();
        sh(&["init", "-q", "-b", "main"]);
        sh(&["config", "user.email", "t@t"]);
        sh(&["config", "user.name", "t"]);
        let commit = |file: &str| {
            fs::write(dir.join(file), "x\n").unwrap();
            sh(&["add", "-A"]);
            sh(&["-c", "commit.gpgsign=false", "commit", "-qnm", file]);
        };
        commit("base.txt");
        sh(&["checkout", "-qb", "feat"]);
        commit("one.txt");
        commit("two.txt");
        fs::write(dir.join("draft.txt"), "y\n").unwrap();
        let found = branch_changes(&dir).unwrap();
        let paths: Vec<&str> = found
            .changes
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!((found.base.as_str(), found.commits), ("main", 2));
        assert_eq!(paths, ["draft.txt", "one.txt", "two.txt"]);
        let _ = fs::remove_dir_all(&dir);
    }

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
