use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::constants::{PERMISSION_TIMER_MS, TEXT_IDLE_TIMER_MS};

/// Events emitted when heuristic timers expire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerEvent {
    /// Permission timer expired — agent likely waiting for user approval.
    PermissionTimeout { agent_id: usize },
    /// Text idle timer expired — agent stopped producing text.
    TextIdleTimeout { agent_id: usize },
}

/// Fires once per period when polled each frame, so work runs at its own cadence.
#[derive(Debug, Clone)]
pub struct PollInterval {
    period: Duration,
    last: Instant,
}

impl PollInterval {
    /// Create an interval of `period_ms` milliseconds, starting at the current instant.
    pub fn new(period_ms: u64) -> Self {
        Self {
            period: Duration::from_millis(period_ms),
            last: Instant::now(),
        }
    }

    /// Whether a full period has passed since the last time this returned true.
    ///
    /// Returning true restarts the period at `now`.
    pub fn due(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last) < self.period {
            return false;
        }
        self.last = now;
        true
    }
}

/// Per-agent heuristic timers checked each frame via `Instant` comparison.
#[derive(Debug, Clone)]
pub struct TimerManager {
    permission_timers: HashMap<usize, Instant>,
    text_idle_timers: HashMap<usize, Instant>,
}

impl TimerManager {
    /// Create an empty timer set.
    pub fn new() -> Self {
        Self {
            permission_timers: HashMap::new(),
            text_idle_timers: HashMap::new(),
        }
    }

    /// Start or restart the permission timer for an agent (7s).
    pub fn start_permission(&mut self, agent_id: usize) {
        let deadline = Instant::now() + Duration::from_millis(PERMISSION_TIMER_MS);
        self.permission_timers.insert(agent_id, deadline);
    }

    /// Whether a permission timer is pending for an agent.
    #[cfg(test)]
    pub(crate) fn permission_pending(&self, agent_id: usize) -> bool {
        self.permission_timers.contains_key(&agent_id)
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
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0, false)] // same instant
    #[case(499, false)] // just below the period
    #[case(500, true)] // exactly the period
    #[case(2_000, true)] // well past the period
    fn poll_interval_is_due_once_a_period_has_passed(#[case] elapsed_ms: u64, #[case] due: bool) {
        let mut interval = PollInterval::new(500);
        let start = interval.last;
        assert_eq!(interval.due(start + Duration::from_millis(elapsed_ms)), due);
    }

    #[test]
    fn poll_interval_restarts_after_firing() {
        let mut interval = PollInterval::new(500);
        let start = interval.last;
        assert!(interval.due(start + Duration::from_millis(600)));
        assert!(!interval.due(start + Duration::from_millis(900)));
        assert!(interval.due(start + Duration::from_millis(1_100)));
    }

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
        mgr.cancel_all(1);
        assert!(mgr.permission_timers.is_empty());
        assert!(mgr.text_idle_timers.is_empty());
    }
}
