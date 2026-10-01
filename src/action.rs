use crossterm::event::{KeyEvent, MouseEvent};

/// Unified action enum. All state mutations flow through `Action`.
///
/// Terminal and tick events produce `Action` variants dispatched to `App::update()`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Action {
    /// Game tick with delta time in seconds.
    Tick(f64),
    /// Render a frame.
    Render,
    /// Terminal resized to (cols, rows).
    Resize(u16, u16),
    /// Keyboard event.
    Key(KeyEvent),
    /// Mouse event.
    Mouse(MouseEvent),
    /// Quit the application.
    Quit,
}
