use std::path::PathBuf;

use clap::Parser;

/// Claude Pixel TUI — pixel-art terminal visualizer for Claude Code agents.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directories to watch for Claude Code JSONL sessions.
    /// Can be specified multiple times.
    #[arg(long)]
    pub watch_dir: Vec<PathBuf>,

    /// Path to a custom layout JSON file.
    #[arg(long)]
    pub layout: Option<PathBuf>,
}
