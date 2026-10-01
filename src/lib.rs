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

/// Entry point: install error handling, parse CLI, build the app, run it.
///
/// The app is built before the terminal enters raw mode, so startup errors print on the normal
/// screen.
pub fn run() -> Result<()> {
    color_eyre::install()?;
    tui::install_panic_hook();
    let args = cli::Args::parse();
    let mut app = app::App::new(args)?;
    let mut terminal_guard = tui::TerminalGuard::new()?;
    app.run(&mut terminal_guard)?;
    Ok(())
}
