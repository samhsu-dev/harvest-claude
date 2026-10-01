use std::io::{self, Stdout};

use color_eyre::eyre::Result;
use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

/// Owns the terminal and guarantees cleanup on drop (including panics).
pub struct TerminalGuard {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    /// Enter raw mode, alternate screen, and enable mouse capture.
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Nothing useful can be done with a restore failure while unwinding or exiting.
        let _ = restore_terminal();
    }
}

/// Leave raw mode, the alternate screen, and mouse capture, and show the cursor.
///
/// Safe to call more than once and when the terminal was never set up.
///
/// # Errors
/// Returns the first terminal I/O error; every step is still attempted.
pub fn restore_terminal() -> Result<()> {
    let raw = disable_raw_mode();
    let screen = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    );
    raw?;
    screen?;
    Ok(())
}

/// Restore the terminal before the previous panic hook prints its report, so the report is
/// readable on the normal screen.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::restore_terminal;

    #[test]
    fn restore_terminal_is_idempotent() {
        restore_terminal().unwrap();
        restore_terminal().unwrap();
    }
}
