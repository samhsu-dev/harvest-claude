use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};

use color_eyre::eyre::{Result, WrapErr};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::constants::{
    DISMISSED_COOLDOWN_SECS, DORMANT_THRESHOLD_SECS, EXTERNAL_THRESHOLD_SECS, GLOBAL_MIN_FILE_SIZE,
    REMOVED_THRESHOLD_SECS, STALE_THRESHOLD_SECS,
};

/// Events emitted by the directory scanner.
#[derive(Debug, Clone)]
pub enum ScanEvent {
    /// A new active JSONL session was discovered.
    NewSession {
        path: PathBuf,
        project_name: String,
        session_id: String,
    },
    /// A tracked session went dormant (no activity for DORMANT_THRESHOLD).
    SessionDormant { path: PathBuf },
    /// A tracked session is truly gone (no activity for REMOVED_THRESHOLD or deleted).
    SessionGone { path: PathBuf },
}

/// Watches directories for Claude Code JSONL session files.
#[derive(Debug)]
pub struct DirectoryScanner {
    watch_dirs: Vec<PathBuf>,
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    events_rx: Receiver<notify::Result<Event>>,
    known_files: HashMap<PathBuf, SystemTime>,
    /// Files that went dormant (stale but not removed).
    dormant_files: HashMap<PathBuf, SystemTime>,
    dismissed_files: HashMap<PathBuf, Instant>,
    clear_dismissed: HashSet<PathBuf>,
    active_threshold: Duration,
    dormant_threshold: Duration,
    removed_threshold: Duration,
    external_threshold: Duration,
    min_file_size: u64,
    pending_clear_files: HashSet<PathBuf>,
}

impl DirectoryScanner {
    /// Create a new scanner watching the given directories.
    pub fn new(dirs: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            // Ignore send errors — receiver may have been dropped.
            let _ = tx.send(res);
        })
        .wrap_err("failed to create filesystem watcher")?;

        for dir in &dirs {
            if dir.exists() {
                watcher
                    .watch(dir, RecursiveMode::Recursive)
                    .wrap_err_with(|| format!("failed to watch directory: {}", dir.display()))?;
            }
        }

        Ok(Self {
            watch_dirs: dirs,
            watcher,
            events_rx: rx,
            known_files: HashMap::new(),
            dormant_files: HashMap::new(),
            dismissed_files: HashMap::new(),
            clear_dismissed: HashSet::new(),
            active_threshold: Duration::from_secs(STALE_THRESHOLD_SECS),
            dormant_threshold: Duration::from_secs(DORMANT_THRESHOLD_SECS),
            removed_threshold: Duration::from_secs(REMOVED_THRESHOLD_SECS),
            external_threshold: Duration::from_secs(EXTERNAL_THRESHOLD_SECS),
            min_file_size: GLOBAL_MIN_FILE_SIZE,
            pending_clear_files: HashSet::new(),
        })
    }

    /// Perform a one-time scan of all watched directories for active JSONL files.
    pub fn initial_scan(&mut self) -> Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        let now = SystemTime::now();

        for dir in &self.watch_dirs.clone() {
            if !dir.exists() {
                continue;
            }
            self.scan_dir_recursive(dir, now, &mut found)?;
        }

        Ok(found)
    }

    /// Drain filesystem events, check for stale sessions, and perform two-tick adoption.
    pub fn poll(&mut self) -> Vec<ScanEvent> {
        let mut events = Vec::new();

        // Drain notify events.
        let mut changed_paths = HashSet::new();
        while let Ok(result) = self.events_rx.try_recv() {
            if let Ok(event) = result {
                for path in event.paths {
                    if is_jsonl_path(&path) {
                        changed_paths.insert(path);
                    }
                }
            }
        }

        let now = SystemTime::now();

        // Process changed paths.
        for path in changed_paths {
            if self.clear_dismissed.contains(&path) {
                continue;
            }
            if let Some(dismissed_at) = self.dismissed_files.get(&path) {
                if dismissed_at.elapsed() < Duration::from_secs(DISMISSED_COOLDOWN_SECS) {
                    continue;
                }
                self.dismissed_files.remove(&path);
            }

            if !path.exists() {
                if self.known_files.remove(&path).is_some()
                    || self.dormant_files.remove(&path).is_some()
                {
                    events.push(ScanEvent::SessionGone { path });
                }
                continue;
            }

            let meta = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let mtime = meta.modified().unwrap_or(now);

            // Wake up dormant files that become active again.
            if self.dormant_files.remove(&path).is_some() {
                self.known_files.insert(path.clone(), mtime);
                if let Some((project_name, session_id)) = extract_session_info(&path) {
                    events.push(ScanEvent::NewSession {
                        path,
                        project_name,
                        session_id,
                    });
                }
                continue;
            }

            if !self.known_files.contains_key(&path) && meta.len() > 0 {
                // Two-tick adoption for /clear files.
                if self.pending_clear_files.contains(&path) {
                    self.pending_clear_files.remove(&path);
                    if let Some((project_name, session_id)) = extract_session_info(&path) {
                        self.known_files.insert(path.clone(), mtime);
                        events.push(ScanEvent::NewSession {
                            path,
                            project_name,
                            session_id,
                        });
                    }
                } else if is_within_threshold(mtime, now, self.external_threshold) {
                    // Check if this is a new file that needs two-tick adoption.
                    if self.check_clear(&path).unwrap_or(false) {
                        self.pending_clear_files.insert(path);
                    } else if let Some((project_name, session_id)) = extract_session_info(&path) {
                        self.known_files.insert(path.clone(), mtime);
                        events.push(ScanEvent::NewSession {
                            path,
                            project_name,
                            session_id,
                        });
                    }
                }
            } else if self.known_files.contains_key(&path) {
                // Update mtime for known files.
                self.known_files.insert(path, mtime);
            }
        }

        // Stage 1: active → dormant (after DORMANT_THRESHOLD).
        let dormant: Vec<PathBuf> = self
            .known_files
            .iter()
            .filter(|(_, mtime)| !is_within_threshold(**mtime, now, self.dormant_threshold))
            .map(|(path, _)| path.clone())
            .collect();

        for path in dormant {
            let mtime = self.known_files.remove(&path).unwrap_or(now);
            self.dormant_files.insert(path.clone(), mtime);
            events.push(ScanEvent::SessionDormant { path });
        }

        // Stage 2: dormant → removed (after REMOVED_THRESHOLD).
        let removed: Vec<PathBuf> = self
            .dormant_files
            .iter()
            .filter(|(_, mtime)| !is_within_threshold(**mtime, now, self.removed_threshold))
            .map(|(path, _)| path.clone())
            .collect();

        for path in removed {
            self.dormant_files.remove(&path);
            events.push(ScanEvent::SessionGone { path });
        }

        events
    }

    /// Mark a session path as dismissed with a cooldown period.
    pub fn dismiss(&mut self, path: &Path) {
        self.known_files.remove(path);
        self.dormant_files.remove(path);
        self.dismissed_files.insert(path.to_owned(), Instant::now());
    }

    /// Permanently dismiss a /clear file.
    pub fn dismiss_clear(&mut self, path: &Path) {
        self.known_files.remove(path);
        self.dormant_files.remove(path);
        self.clear_dismissed.insert(path.to_owned());
    }

    /// Check if a JSONL file contains a /clear command in its first 8KB.
    pub fn check_clear(&self, path: &Path) -> Result<bool> {
        let mut file =
            File::open(path).wrap_err_with(|| format!("failed to open: {}", path.display()))?;

        let mut buf = vec![0u8; 8192];
        let n = file
            .read(&mut buf)
            .wrap_err_with(|| format!("failed to read: {}", path.display()))?;

        let content = String::from_utf8_lossy(&buf[..n]);
        Ok(content.contains("/clear</command-name>"))
    }

    // Recursively scan a directory for JSONL files.
    fn scan_dir_recursive(
        &mut self,
        dir: &Path,
        now: SystemTime,
        found: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if path.is_dir() {
                self.scan_dir_recursive(&path, now, found)?;
                continue;
            }

            if !is_jsonl_path(&path) {
                continue;
            }

            let meta = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.len() < self.min_file_size {
                continue;
            }

            let mtime = meta.modified().unwrap_or(now);
            if !is_within_threshold(mtime, now, self.active_threshold) {
                continue;
            }

            self.known_files.insert(path.clone(), mtime);
            found.push(path);
        }

        Ok(())
    }
}

/// Replace `:`, `\`, `/` with `-` to create a filesystem-safe project hash.
pub fn project_hash(path: &str) -> String {
    path.chars()
        .map(|c| match c {
            ':' | '\\' | '/' => '-',
            _ => c,
        })
        .collect()
}

// Extract project name and session ID from a JSONL file path.
//
// Expected path pattern: `.../projects/<project_hash>/sessions/<session_id>/...`
fn extract_session_info(path: &Path) -> Option<(String, String)> {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Look for "sessions" component and extract session_id after it.
    let sessions_idx = components.iter().position(|&c| c == "sessions")?;
    let session_id = components.get(sessions_idx + 1)?.to_string();

    // Look for "projects" component and extract project_name after it.
    let projects_idx = components.iter().position(|&c| c == "projects");
    let project_name = match projects_idx {
        Some(idx) => components.get(idx + 1).unwrap_or(&"unknown").to_string(),
        None => "unknown".to_string(),
    };

    Some((project_name, session_id))
}

// Check if a path ends with .jsonl extension and is not a sub-agent session.
//
// Sub-agent sessions live under `.../subagents/agent-*.jsonl` and should not
// be treated as top-level sessions.
fn is_jsonl_path(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "jsonl")
        && !path
            .components()
            .any(|c| c.as_os_str().to_str().is_some_and(|s| s == "subagents"))
}

// Check if a SystemTime is within a threshold from now.
fn is_within_threshold(mtime: SystemTime, now: SystemTime, threshold: Duration) -> bool {
    match now.duration_since(mtime) {
        Ok(age) => age <= threshold,
        // mtime is in the future — treat as recent.
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_hash_replaces_separators() {
        assert_eq!(project_hash("/home/user/project"), "-home-user-project");
        assert_eq!(project_hash("C:\\Users\\project"), "C--Users-project");
        assert_eq!(project_hash("simple"), "simple");
    }

    #[test]
    fn is_jsonl_path_check() {
        assert!(is_jsonl_path(Path::new("session.jsonl")));
        assert!(!is_jsonl_path(Path::new("session.json")));
        assert!(!is_jsonl_path(Path::new("session")));
    }

    #[test]
    fn check_clear_detects_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "some data /clear</command-name> more").unwrap();

        let scanner = DirectoryScanner::new(vec![dir.path().to_owned()]).unwrap();
        assert!(scanner.check_clear(&path).unwrap());
    }

    #[test]
    fn check_clear_negative() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "normal session data without clear").unwrap();

        let scanner = DirectoryScanner::new(vec![dir.path().to_owned()]).unwrap();
        assert!(!scanner.check_clear(&path).unwrap());
    }

    #[test]
    fn extract_session_info_from_path() {
        let path = PathBuf::from("/home/.claude/projects/my-project/sessions/abc123/data.jsonl");
        let (project, session) = extract_session_info(&path).unwrap();
        assert_eq!(project, "my-project");
        assert_eq!(session, "abc123");
    }
}
