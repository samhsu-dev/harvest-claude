pub mod action;
pub mod app;
pub mod cli;
pub mod constants;
pub mod engine;
pub mod event;
pub mod layout;
pub mod render;
pub mod tui;
pub mod types;
pub mod ui;
pub mod watcher;

use clap::Parser;
use color_eyre::eyre::Result;

/// Entry point: install error handling, parse CLI, run the application.
pub fn run() -> Result<()> {
    color_eyre::install()?;
    let args = cli::Args::parse();
    let mut terminal_guard = tui::TerminalGuard::new()?;
    let mut app = app::App::new(args)?;
    app.run(&mut terminal_guard)?;
    Ok(())
}
