use std::path::Path;

use color_eyre::eyre::{Result, WrapErr};
use tracing_appender::non_blocking::{self, NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

use crate::constants::{LOG_DIR_NAME, LOG_FILE_PREFIX, LOG_FILE_SUFFIX, LOG_MAX_FILES};

/// Install a file-backed tracing subscriber writing under `<dir>/logs`.
///
/// The filter comes from `RUST_LOG` and defaults to `info`. The caller keeps the returned guard
/// for the process lifetime, because dropping it discards buffered lines.
///
/// # Errors
/// Returns an error when the log directory or file cannot be created, or when a global
/// subscriber is already installed.
pub fn init(dir: &Path) -> Result<WorkerGuard> {
    let (writer, guard) = non_blocking::NonBlocking::new(build_appender(dir)?);
    install_subscriber(writer)?;
    Ok(guard)
}

fn install_subscriber(writer: NonBlocking) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| color_eyre::eyre::eyre!("failed to install the tracing subscriber: {e}"))
}

/// Daily-rolling appender that keeps at most [`LOG_MAX_FILES`] files.
fn build_appender(dir: &Path) -> Result<RollingFileAppender> {
    let logs = dir.join(LOG_DIR_NAME);
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .max_log_files(LOG_MAX_FILES)
        .build(&logs)
        .wrap_err_with(|| format!("failed to open log files in {}", logs.display()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn log_files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.join(LOG_DIR_NAME))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn appender_creates_a_dated_log_file_under_the_logs_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut appender = build_appender(dir.path()).unwrap();
        appender.write_all(b"line\n").unwrap();
        appender.flush().unwrap();

        let names = log_files(dir.path());
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("harvest-claude.") && names[0].ends_with(".log"));
    }

    #[test]
    fn appender_prunes_old_files_beyond_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join(LOG_DIR_NAME);
        std::fs::create_dir_all(&logs).unwrap();
        for day in 1..=5 {
            std::fs::write(
                logs.join(format!("harvest-claude.2000-01-0{day}.log")),
                "old",
            )
            .unwrap();
        }
        let mut appender = build_appender(dir.path()).unwrap();
        appender.write_all(b"line\n").unwrap();
        appender.flush().unwrap();

        assert_eq!(log_files(dir.path()).len(), LOG_MAX_FILES);
    }

    #[test]
    fn unwritable_directory_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        std::fs::write(&blocker, "x").unwrap();

        assert!(build_appender(&blocker).is_err());
    }
}
