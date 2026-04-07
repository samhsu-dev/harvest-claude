use std::path::Path;
use std::process::Command;

/// Attempt to focus the terminal window running the Claude Code session
/// that owns the given JSONL path.
///
/// Strategy: extract the project working directory from the JSONL path,
/// find the `claude` process whose CWD matches, get its TTY, then use
/// AppleScript to activate the corresponding terminal window/tab.
pub fn focus_agent_window(jsonl_path: &Path) {
    let Some(project_cwd) = cwd_from_jsonl_path(jsonl_path) else {
        return;
    };

    let Some(tty) = find_claude_tty(&project_cwd) else {
        return;
    };

    focus_terminal_by_tty(&tty);
}

/// Extract the project working directory from a JSONL path.
///
/// Path pattern: `~/.claude/projects/<project_hash>/sessions/<id>/...`
/// Project hash: `-Users-foo-Projects-bar` → `/Users/foo/Projects/bar`
fn cwd_from_jsonl_path(path: &Path) -> Option<String> {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let projects_idx = components.iter().position(|&c| c == "projects")?;
    let hash = components.get(projects_idx + 1)?;

    // Convert hash back to path: leading `-` → `/`, internal `-` → `/`
    // But we need to be careful: the hash replaces `:`, `\`, `/` with `-`.
    // On macOS paths like `/Users/foo/bar` become `-Users-foo-bar`.
    let restored = hash.replacen('-', "/", 1);
    let restored = restored.replace('-', "/");

    // Verify it looks like a valid path
    if restored.starts_with('/') {
        Some(restored)
    } else {
        None
    }
}

/// Find the TTY of a `claude` process whose CWD matches the given path.
///
/// Uses `lsof` to find CWDs of claude processes, then `ps` to get TTY.
fn find_claude_tty(project_cwd: &str) -> Option<String> {
    // Get all claude process PIDs and their TTYs
    let output = Command::new("ps")
        .args(["-eo", "pid,tty,comm"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let claude_pids: Vec<u32> = stdout
        .lines()
        .filter(|line| line.contains("claude") && !line.contains("harvest"))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.first()?.parse().ok()
        })
        .collect();

    for pid in &claude_pids {
        // Use lsof to get CWD for this PID
        let output = Command::new("lsof")
            .args(["-p", &pid.to_string(), "-Fn"])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut found_cwd = false;
        for line in stdout.lines() {
            if line == "fcwd" {
                found_cwd = true;
                continue;
            }
            if found_cwd && line.starts_with('n') {
                let cwd = &line[1..];
                if cwd == project_cwd {
                    // Found matching PID, now get its TTY
                    return tty_for_pid(*pid);
                }
                found_cwd = false;
            }
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

/// Use AppleScript to focus a terminal window/tab by TTY name.
///
/// Supports Terminal.app and iTerm2.
fn focus_terminal_by_tty(tty: &str) {
    // Try Terminal.app first
    let script = format!(
        r#"
        tell application "System Events"
            set termRunning to (name of processes) contains "Terminal"
        end tell
        if termRunning then
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
        "#,
    );

    let result = Command::new("osascript").args(["-e", &script]).output();

    if let Ok(output) = result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim() == "ok" {
            return;
        }
    }

    // Try iTerm2
    let iterm_script = format!(
        r#"
        tell application "System Events"
            set itermRunning to (name of processes) contains "iTerm2"
        end tell
        if itermRunning then
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
        "#,
    );

    let _ = Command::new("osascript")
        .args(["-e", &iterm_script])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cwd_from_jsonl_path_basic() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-Projects-myapp/sessions/abc/data.jsonl",
        );
        let cwd = cwd_from_jsonl_path(&path);
        assert_eq!(cwd, Some("/Users/foo/Projects/myapp".to_owned()));
    }

    #[test]
    fn cwd_from_jsonl_path_deep_project() {
        let path = PathBuf::from(
            "/Users/foo/.claude/projects/-Users-foo-work-org-repo/sessions/x/s.jsonl",
        );
        let cwd = cwd_from_jsonl_path(&path);
        assert_eq!(cwd, Some("/Users/foo/work/org/repo".to_owned()));
    }

    #[test]
    fn cwd_from_jsonl_path_no_projects_component() {
        let path = PathBuf::from("/some/random/path.jsonl");
        assert!(cwd_from_jsonl_path(&path).is_none());
    }

    #[test]
    fn cwd_from_jsonl_path_missing_hash() {
        let path = PathBuf::from("/Users/foo/.claude/projects/");
        assert!(cwd_from_jsonl_path(&path).is_none());
    }
}
