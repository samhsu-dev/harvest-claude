use std::path::PathBuf;

use crossterm::event::{KeyEvent, MouseEvent};

use crate::types::AgentEvent;

/// Unified action enum. All state mutations flow through `Action`.
///
/// Terminal events, watcher events, and timer events all produce `Action`
/// variants dispatched to `App::update()`.
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

    // -- Watcher events (from mpsc channel) --
    /// New agent session discovered.
    AgentDiscovered {
        path: PathBuf,
        project: String,
        session_id: String,
    },
    /// Agent session gone stale or removed.
    AgentGone { path: PathBuf },
    /// Event from an agent's JSONL stream.
    AgentEvent { agent_id: usize, event: AgentEvent },

    // -- Timer events --
    /// Permission timer expired for an agent.
    PermissionTimeout { agent_id: usize },
    /// Text-idle timer expired for an agent.
    TextIdleTimeout { agent_id: usize },
    /// Tool-done delay completed, ready for UI update.
    ToolDoneReady { agent_id: usize, tool_id: String },
}
