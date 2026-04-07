use std::path::Path;
use std::process::Command;

/// Attempt to focus the terminal window running the Claude Code session
/// that owns the given JSONL path.
///
/// Strategy: extract the project folder name from the JSONL path, then
/// use AppleScript to find and activate a terminal window whose title
/// contains that folder name.
pub fn focus_agent_window(jsonl_path: &Path) {
    let Some(folder) = project_folder_from_path(jsonl_path) else {
        return;
    };

    // Primary: search terminal window titles for the project folder
    if focus_terminal_by_title(&folder) {
        return;
    }

    // Fallback: try CWD-based process matching
    if let Some(cwd) = cwd_from_jsonl_path(jsonl_path)
        && let Some(tty) = find_tty_by_cwd(&cwd)
    {
        focus_terminal_by_tty(&tty);
    }
}

/// Extract the project folder name from a JSONL session path.
///
/// Path: `~/.claude/projects/-Users-foo-Projects-bar/sessions/<id>/...`
/// Hash: `-Users-foo-Projects-bar` → folder: `bar`
fn project_folder_from_path(path: &Path) -> Option<String> {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let projects_idx = components.iter().position(|&c| c == "projects")?;
    let hash = *components.get(projects_idx + 1)?;

    let parts: Vec<&str> = hash.split('-').filter(|s| !s.is_empty()).collect();

    // Find the last well-known directory name and take everything after it
    // as the project folder (re-joined with `-` to handle hyphenated names).
    let marker_idx = parts.iter().rposition(|p| {
        matches!(
            *p,
            "Users"
                | "home"
                | "root"
                | "Projects"
                | "Repos"
                | "repos"
                | "src"
                | "Documents"
                | "Desktop"
                | "Work"
                | "code"
                | "dev"
                | "workspace"
                | "Code"
        )
    });

    match marker_idx {
        Some(idx) if idx + 1 < parts.len() => Some(parts[idx + 1..].join("-")),
        _ => parts.last().map(|s| s.to_string()),
    }
}

/// Reconstruct the project CWD from the JSONL path hash.
///
/// Hash: `-Users-foo-Projects-bar` → `/Users/foo/Projects/bar`.
/// Lossy for paths containing literal hyphens.
fn cwd_from_jsonl_path(path: &Path) -> Option<String> {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let projects_idx = components.iter().position(|&c| c == "projects")?;
    let hash = components.get(projects_idx + 1)?;

    let restored = hash.replacen('-', "/", 1);
    let restored = restored.replace('-', "/");

    if restored.starts_with('/') {
        Some(restored)
    } else {
        None
    }
}

/// Find a TTY by searching for processes whose CWD matches the target path.
///
/// Uses `lsof -d cwd` to enumerate all process working directories.
fn find_tty_by_cwd(target_cwd: &str) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-d", "cwd", "-Fpn"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut current_pid: Option<u32> = None;

    for line in stdout.lines() {
        if let Some(pid_str) = line.strip_prefix('p') {
            current_pid = pid_str.parse().ok();
        } else if let Some(name) = line.strip_prefix('n')
            && name == target_cwd
            && let Some(pid) = current_pid
            && let Some(tty) = tty_for_pid(pid)
        {
            return Some(tty);
        }
    }

    None
}

/// Get the TTY name for a given PID.
fn tty_for_pid(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-o", "tty=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    let tty = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if tty.is_empty() || tty == "??" {
        None
    } else {
        Some(tty)
    }
}

/// Search terminal window titles for the project folder name and activate the match.
///
/// Returns true if a matching window was found and activated.
fn focus_terminal_by_title(project_folder: &str) -> bool {
    // Try Terminal.app
    let term_script = format!(
        r#"
tell application "System Events"
    if (name of processes) contains "Terminal" then
        tell application "Terminal"
            repeat with w in windows
                if name of w contains "{folder}" then
                    activate
                    set index of w to 1
                    return "ok"
                end if
            end repeat
        end tell
    end if
end tell"#,
        folder = project_folder
    );

    if run_osascript(&term_script) {
        return true;
    }

    // Try iTerm2
    let iterm_script = format!(
        r#"
tell application "System Events"
    if (name of processes) contains "iTerm2" then
        tell application "iTerm2"
            repeat with w in windows
                repeat with t in tabs of w
                    repeat with s in sessions of t
                        if name of s contains "{folder}" then
                            activate
                            select t
                            select s
                            return "ok"
                        end if
                    end repeat
                end repeat
            end repeat
        end tell
    end if
end tell"#,
        folder = project_folder
    );

    run_osascript(&iterm_script)
}

/// Use AppleScript to focus a terminal window/tab by TTY name.
fn focus_terminal_by_tty(tty: &str) {
    let term_script = format!(
        r#"
tell application "System Events"
    if (name of processes) contains "Terminal" then
        tell application "Terminal"
            repeat with w in windows
                repeat with t in tabs of w
                    if tty of t contains "{tty}" then
                        activate
                        set selected tab of w to t
                        set index of w to 1
                        return "ok"
                    end if
                end repeat
            end repeat
        end tell
    end if
end tell"#,
    );

    if run_osascript(&term_script) {
        return;
    }

    let iterm_script = format!(
        r#"
tell application "System Events"
    if (name of processes) contains "iTerm2" then
        tell application "iTerm2"
            repeat with w in windows
                repeat with t in tabs of w
                    repeat with s in sessions of t
                        if tty of s contains "{tty}" then
                            activate
                            select t
                            select s
                            return "ok"
                        end if
                    end repeat
                end repeat
            end repeat
        end tell
    end if
end tell"#,
    );

    let _ = run_osascript(&iterm_script);
}

/// Run an AppleScript and return true if it outputs "ok".
fn run_osascript(script: &str) -> bool {
    Command::new("osascript")
        .args(["-e", script])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn project_folder_basic() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-Projects-myapp/sessions/abc/data.jsonl",
        );
        assert_eq!(project_folder_from_path(&path), Some("myapp".to_owned()));
    }

    #[test]
    fn project_folder_with_dots() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-Projects-claude.pixel/sessions/x/s.jsonl",
        );
        assert_eq!(
            project_folder_from_path(&path),
            Some("claude.pixel".to_owned())
        );
    }

    #[test]
    fn project_folder_deep_path() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-work-org-repo/sessions/x/s.jsonl",
        );
        // After "Users" marker: "foo-work-org-repo"
        // Terminal title search still matches since it contains "repo"
        assert_eq!(
            project_folder_from_path(&path),
            Some("foo-work-org-repo".to_owned())
        );
    }

    #[test]
    fn project_folder_hyphenated_name() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-Projects-my-cool-app/sessions/x/s.jsonl",
        );
        // Everything after "Projects" marker
        assert_eq!(
            project_folder_from_path(&path),
            Some("my-cool-app".to_owned())
        );
    }

    #[test]
    fn project_folder_no_projects_component() {
        let path = PathBuf::from("/some/random/path.jsonl");
        assert!(project_folder_from_path(&path).is_none());
    }

    #[test]
    fn cwd_from_jsonl_path_basic() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-Projects-myapp/sessions/abc/data.jsonl",
        );
        let cwd = cwd_from_jsonl_path(&path);
        assert_eq!(cwd, Some("/Users/foo/Projects/myapp".to_owned()));
    }

    #[test]
    fn cwd_from_jsonl_path_missing_hash() {
        let path = PathBuf::from("/Users/foo/.claude/projects/");
        assert!(cwd_from_jsonl_path(&path).is_none());
    }
}
