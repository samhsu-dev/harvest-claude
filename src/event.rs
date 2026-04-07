use std::sync::mpsc;
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};

use crate::action::Action;
use crate::constants::TICK_RATE_MS;

/// Bridges crossterm terminal events to `Action` variants via an mpsc channel.
///
/// Spawns a background thread that polls crossterm events and sends `Action`
/// values. The watcher thread uses `sender()` to inject its own actions.
pub struct EventHandler {
    rx: mpsc::Receiver<Action>,
    tx: mpsc::Sender<Action>,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    /// Create a new event handler. Spawns a crossterm polling thread.
    pub fn new() -> Self {
        let tick_rate = Duration::from_millis(TICK_RATE_MS);
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        std::thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate.saturating_sub(last_tick.elapsed());

                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if key.kind == KeyEventKind::Press
                                && event_tx.send(Action::Key(key)).is_err()
                            {
                                return;
                            }
                        }
                        Ok(Event::Mouse(mouse)) => {
                            if event_tx.send(Action::Mouse(mouse)).is_err() {
                                return;
                            }
                        }
                        Ok(Event::Resize(cols, rows)) => {
                            if event_tx.send(Action::Resize(cols, rows)).is_err() {
                                return;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => return,
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    let dt = last_tick.elapsed().as_secs_f64();
                    last_tick = Instant::now();
                    if event_tx.send(Action::Tick(dt)).is_err() {
                        return;
                    }
                    if event_tx.send(Action::Render).is_err() {
                        return;
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// Blocking receive of the next action.
    pub fn next(&self) -> Result<Action> {
        let action = self.rx.recv()?;
        Ok(action)
    }

    /// Clone of the sender for use by the watcher thread.
    pub fn sender(&self) -> mpsc::Sender<Action> {
        self.tx.clone()
    }
}
