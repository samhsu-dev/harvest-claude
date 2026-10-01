use std::process::Command;

/// Focus the terminal tab running the session whose working directory is `cwd`.
///
/// Runs on a spawned thread so the UI never waits for `lsof` or `osascript`.
/// Only macOS Terminal.app and iTerm2 are supported; elsewhere this logs and returns.
pub fn focus_agent_window(cwd: String) {
    if !cfg!(target_os = "macos") {
        tracing::info!("window focus is only supported on macOS");
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("focus-window".into())
        .spawn(move || focus_by_cwd(&cwd));
    if let Err(e) = spawned {
        tracing::warn!("failed to start the focus thread: {e}");
    }
}

fn focus_by_cwd(cwd: &str) {
    let Some(tty) = find_tty_by_cwd(cwd) else {
        tracing::info!(cwd, "no process with this working directory has a terminal");
        return;
    };
    let focused = run_osascript(&terminal_script(&tty)) || run_osascript(&iterm_script(&tty));
    if !focused {
        tracing::info!(
            cwd,
            tty,
            "no Terminal or iTerm2 tab owns this terminal device"
        );
    }
}

/// Escape text for placement inside an AppleScript string literal.
fn escape_applescript(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// AppleScript that selects the Terminal.app tab attached to `tty`.
fn terminal_script(tty: &str) -> String {
    let tty = escape_applescript(tty);
    format!(
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
end tell"#
    )
}

/// AppleScript that selects the iTerm2 session attached to `tty`.
fn iterm_script(tty: &str) -> String {
    let tty = escape_applescript(tty);
    format!(
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
end tell"#
    )
}

/// Terminal device of a process whose working directory is exactly `target_cwd`.
///
/// Uses `lsof -d cwd` to enumerate every process working directory.
fn find_tty_by_cwd(target_cwd: &str) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-d", "cwd", "-Fpn"])
        .output()
        .ok()?;
    pids_with_cwd(&String::from_utf8_lossy(&output.stdout), target_cwd)
        .into_iter()
        .find_map(tty_for_pid)
}

/// Process ids from `lsof -Fpn` output whose working directory equals `target_cwd`.
fn pids_with_cwd(lsof_output: &str, target_cwd: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    let mut current: Option<u32> = None;
    for line in lsof_output.lines() {
        if let Some(pid) = line.strip_prefix('p') {
            current = pid.parse().ok();
        } else if line.strip_prefix('n') == Some(target_cwd)
            && let Some(pid) = current
        {
            pids.push(pid);
        }
    }
    pids
}

/// Terminal device name of `pid`, or `None` for a process without one.
fn tty_for_pid(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-o", "tty=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let tty = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!tty.is_empty() && tty != "??").then_some(tty)
}

/// Run an AppleScript and return true if it prints "ok".
fn run_osascript(script: &str) -> bool {
    Command::new("osascript")
        .args(["-e", script])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("ttys003", "ttys003")] // plain device name
    #[case(r#"a"b"#, r#"a\"b"#)] // quote
    #[case(r"a\b", r"a\\b")] // backslash
    #[case(
        r#"\" & (do shell script "x") & ""#,
        r#"\\\" & (do shell script \"x\") & \""#
    )] // breakout attempt
    fn applescript_text_is_escaped(#[case] text: &str, #[case] want: &str) {
        assert_eq!(escape_applescript(text), want);
    }

    #[rstest]
    #[case(terminal_script(r#"x" & "y"#))]
    #[case(iterm_script(r#"x" & "y"#))]
    fn scripts_embed_the_escaped_device(#[case] script: String) {
        assert!(script.contains(r#"contains "x\" & \"y""#), "{script}");
    }

    #[test]
    fn lsof_output_yields_pids_with_the_exact_directory() {
        let output = "p101\nn/Users/u/proj\np102\nn/Users/u/proj-other\np103\nn/Users/u/proj\n";
        assert_eq!(pids_with_cwd(output, "/Users/u/proj"), vec![101, 103]);
    }

    #[test]
    fn lsof_output_without_a_match_yields_no_pids() {
        assert!(pids_with_cwd("p1\nn/tmp\n", "/Users/u/proj").is_empty());
    }
}
