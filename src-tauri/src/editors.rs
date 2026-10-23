use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub command: String,
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
const JETBRAINS: [&str; 9] = [
    "phpstorm",
    "idea",
    "webstorm",
    "pycharm",
    "goland",
    "rustrover",
    "clion",
    "rider",
    "rubymine",
];

#[cfg_attr(target_os = "macos", allow(dead_code))]
fn field(entry: &str, key: &str) -> Option<String> {
    entry
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().to_string())
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn from_desktop(text: &str) -> Option<App> {
    let entry = text.split("\n[").next()?;
    let categories = field(entry, "Categories").unwrap_or_default();
    let exec = field(entry, "Exec")?;
    if field(entry, "Terminal").as_deref() == Some("true")
        || field(entry, "NoDisplay").as_deref() == Some("true")
    {
        return None;
    }
    let program = shell_words::split(&exec)
        .ok()?
        .into_iter()
        .filter(|word| !(word.len() == 2 && word.starts_with('%')))
        .collect::<Vec<_>>();
    let base = Path::new(program.first()?)
        .file_stem()?
        .to_string_lossy()
        .to_lowercase();
    let jetbrains = JETBRAINS.iter().any(|name| base.starts_with(name));
    if !jetbrains
        && !categories
            .split(';')
            .any(|c| c == "TextEditor" || c == "IDE")
    {
        return None;
    }
    let target = if jetbrains {
        "{project}"
    } else {
        "{project} {file}"
    };
    Some(App {
        name: field(entry, "Name")?,
        command: format!("{} {target}", shell_words::join(program)),
    })
}

#[cfg(target_os = "macos")]
pub fn detect() -> Vec<App> {
    const KNOWN: [(&str, bool); 19] = [
        ("Visual Studio Code", false),
        ("Cursor", false),
        ("Zed", false),
        ("Windsurf", false),
        ("VSCodium", false),
        ("Sublime Text", false),
        ("PhpStorm", true),
        ("IntelliJ IDEA", true),
        ("IntelliJ IDEA Ultimate", true),
        ("IntelliJ IDEA CE", true),
        ("WebStorm", true),
        ("PyCharm", true),
        ("PyCharm Professional Edition", true),
        ("PyCharm CE", true),
        ("GoLand", true),
        ("RustRover", true),
        ("CLion", true),
        ("Rider", true),
        ("RubyMine", true),
    ];
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    let folders = [PathBuf::from("/Applications"), home.join("Applications")];
    KNOWN
        .iter()
        .filter(|(name, _)| {
            folders
                .iter()
                .any(|dir| dir.join(format!("{name}.app")).exists())
        })
        .map(|(name, whole)| App {
            name: name.to_string(),
            command: format!(
                "open -a {} {}",
                shell_words::quote(name),
                if *whole {
                    "{project}"
                } else {
                    "{project} {file}"
                }
            ),
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn detect() -> Vec<App> {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    let dirs = [
        home.join(".local/share/applications"),
        home.join(".local/share/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/usr/share/applications"),
    ];
    let mut apps: Vec<App> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let Some(app) = std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|text| from_desktop(&text))
            else {
                continue;
            };
            if !apps.iter().any(|a| a.name == app.name) {
                apps.push(app);
            }
        }
    }
    apps
}

pub fn arguments(
    command: &str,
    project: &Path,
    file: Option<&Path>,
) -> Result<Vec<String>, String> {
    let words = shell_words::split(command).map_err(|e| e.to_string())?;
    if words.is_empty() {
        return Err("Empty command.".into());
    }
    let placed = words
        .iter()
        .any(|w| w.contains("{project}") || w.contains("{file}"));
    let mut args: Vec<String> = words
        .into_iter()
        .filter(|w| file.is_some() || !w.contains("{file}"))
        .map(|w| {
            let w = w.replace("{project}", &project.to_string_lossy());
            match file {
                Some(file) => w.replace("{file}", &file.to_string_lossy()),
                None => w,
            }
        })
        .collect();
    if !placed {
        args.push(file.unwrap_or(project).to_string_lossy().into_owned());
    }
    Ok(args)
}

pub fn open(command: &str, project: &Path, file: Option<&Path>) -> Result<(), String> {
    let args = arguments(command, project, file)?;
    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", args[0]))?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_editors_from_desktop_entries() {
        let zed = "[Desktop Entry]\nName=Zed\nExec=/home/u/.local/zed.app/bin/zed %U\nCategories=Utility;TextEditor;Development;IDE;\n\n[Desktop Action NewWorkspace]\nExec=/home/u/.local/zed.app/bin/zed --new %U\nName=Open a new workspace\n";
        assert_eq!(
            from_desktop(zed),
            Some(App {
                name: "Zed".into(),
                command: "/home/u/.local/zed.app/bin/zed {project} {file}".into()
            })
        );
        let storm = "[Desktop Entry]\nName=PhpStorm 2026.2\nExec=\"/opt/Php Storm/bin/phpstorm.sh\" %f\nCategories=Development;\n";
        assert_eq!(
            from_desktop(storm).unwrap().command,
            "'/opt/Php Storm/bin/phpstorm.sh' {project}"
        );
        assert_eq!(
            from_desktop(
                "[Desktop Entry]\nName=Vim\nExec=vim %F\nTerminal=true\nCategories=TextEditor;\n"
            ),
            None
        );
        assert_eq!(
            from_desktop("[Desktop Entry]\nName=Slack\nExec=slack %U\nCategories=Network;\n"),
            None
        );
    }

    #[test]
    fn fills_the_command_with_the_project_and_the_file() {
        let project = Path::new("/p");
        let file = Path::new("/p/src/a.php");
        assert_eq!(
            arguments("zed {project} {file}", project, Some(file)).unwrap(),
            ["zed", "/p", "/p/src/a.php"]
        );
        assert_eq!(
            arguments("zed {project} {file}", project, None).unwrap(),
            ["zed", "/p"]
        );
        assert_eq!(
            arguments("subl", project, Some(file)).unwrap(),
            ["subl", "/p/src/a.php"]
        );
        assert_eq!(
            arguments("phpstorm {project}", project, Some(file)).unwrap(),
            ["phpstorm", "/p"]
        );
        assert!(arguments("", project, None).is_err());
    }
}
