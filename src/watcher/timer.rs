use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::constants::{PERMISSION_TIMER_MS, TEXT_IDLE_TIMER_MS, TOOL_DONE_DELAY_MS};

/// Events emitted when heuristic timers expire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerEvent {
    /// Permission timer expired — agent likely waiting for user approval.
    PermissionTimeout { agent_id: usize },
    /// Text idle timer expired — agent stopped producing text.
    TextIdleTimeout { agent_id: usize },
    /// Tool done delay elapsed — safe to update UI without flicker.
    ToolDoneReady { agent_id: usize, tool_id: String },
}

/// Per-agent heuristic timers checked each frame via `Instant` comparison.
#[derive(Debug, Clone)]
pub struct TimerManager {
    permission_timers: HashMap<usize, Instant>,
    text_idle_timers: HashMap<usize, Instant>,
    tool_done_delays: HashMap<(usize, String), Instant>,
}

impl TimerManager {
    /// Create an empty timer set.
    pub fn new() -> Self {
        Self {
            permission_timers: HashMap::new(),
            text_idle_timers: HashMap::new(),
            tool_done_delays: HashMap::new(),
        }
    }

    /// Start or restart the permission timer for an agent (7s).
    pub fn start_permission(&mut self, agent_id: usize) {
        let deadline = Instant::now() + Duration::from_millis(PERMISSION_TIMER_MS);
        self.permission_timers.insert(agent_id, deadline);
    }

    /// Restart the permission timer only if one already exists.
    ///
    /// Used for bash/mcp progress events that indicate the tool is still executing.
    pub fn restart_permission(&mut self, agent_id: usize) {
        if self.permission_timers.contains_key(&agent_id) {
            self.start_permission(agent_id);
        }
    }

    /// Cancel the permission timer for an agent.
    pub fn cancel_permission(&mut self, agent_id: usize) {
        self.permission_timers.remove(&agent_id);
    }

    /// Start or restart the text idle timer for an agent (5s).
    pub fn start_text_idle(&mut self, agent_id: usize) {
        let deadline = Instant::now() + Duration::from_millis(TEXT_IDLE_TIMER_MS);
        self.text_idle_timers.insert(agent_id, deadline);
    }

    /// Cancel the text idle timer for an agent.
    pub fn cancel_text_idle(&mut self, agent_id: usize) {
        self.text_idle_timers.remove(&agent_id);
    }

    /// Cancel all timers for an agent.
    pub fn cancel_all(&mut self, agent_id: usize) {
        self.permission_timers.remove(&agent_id);
        self.text_idle_timers.remove(&agent_id);
        self.tool_done_delays.retain(|(id, _), _| *id != agent_id);
    }

    /// Start a 300ms delay before reporting a tool as done.
    pub fn delay_tool_done(&mut self, agent_id: usize, tool_id: String) {
        let deadline = Instant::now() + Duration::from_millis(TOOL_DONE_DELAY_MS);
        self.tool_done_delays.insert((agent_id, tool_id), deadline);
    }

    /// Check all timers and return events for any that have expired.
    ///
    /// Expired timers are removed from the manager.
    pub fn check_expired(&mut self) -> Vec<TimerEvent> {
        let now = Instant::now();
        let mut events = Vec::new();

        let expired_permission: Vec<usize> = self
            .permission_timers
            .iter()
            .filter(|(_, deadline)| now >= **deadline)
            .map(|(id, _)| *id)
            .collect();

        for agent_id in expired_permission {
            self.permission_timers.remove(&agent_id);
            events.push(TimerEvent::PermissionTimeout { agent_id });
        }

        let expired_idle: Vec<usize> = self
            .text_idle_timers
            .iter()
            .filter(|(_, deadline)| now >= **deadline)
            .map(|(id, _)| *id)
            .collect();

        for agent_id in expired_idle {
            self.text_idle_timers.remove(&agent_id);
            events.push(TimerEvent::TextIdleTimeout { agent_id });
        }

        let expired_done: Vec<(usize, String)> = self
            .tool_done_delays
            .iter()
            .filter(|(_, deadline)| now >= **deadline)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_done {
            self.tool_done_delays.remove(&key);
            events.push(TimerEvent::ToolDoneReady {
                agent_id: key.0,
                tool_id: key.1,
            });
        }

        events
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_timer_expires() {
        let mut mgr = TimerManager::new();
        // Manually insert a deadline in the past.
        mgr.permission_timers
            .insert(1, Instant::now() - Duration::from_secs(1));
        let events = mgr.check_expired();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], TimerEvent::PermissionTimeout { agent_id: 1 });
        assert!(mgr.permission_timers.is_empty());
    }

    #[test]
    fn cancel_all_removes_everything() {
        let mut mgr = TimerManager::new();
        mgr.start_permission(1);
        mgr.start_text_idle(1);
        mgr.delay_tool_done(1, "t1".to_owned());
        mgr.cancel_all(1);
        assert!(mgr.permission_timers.is_empty());
        assert!(mgr.text_idle_timers.is_empty());
        assert!(mgr.tool_done_delays.is_empty());
    }

    #[test]
    fn restart_permission_only_if_exists() {
        let mut mgr = TimerManager::new();
        mgr.restart_permission(1);
        assert!(mgr.permission_timers.is_empty());

        mgr.start_permission(1);
        mgr.restart_permission(1);
        assert!(mgr.permission_timers.contains_key(&1));
    }
}
