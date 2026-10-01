use std::path::PathBuf;

use clap::Parser;

/// Harvest Claude — pixel-art terminal farm that visualizes Claude Code agents.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directories to watch for Claude Code JSONL sessions, in addition to
    /// `~/.claude/projects`. Can be specified multiple times.
    #[arg(long)]
    pub watch_dir: Vec<PathBuf>,

    /// Path to a custom layout JSON file.
    #[arg(long)]
    pub layout: Option<PathBuf>,
}
