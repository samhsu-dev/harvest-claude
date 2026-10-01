use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};

use crate::constants::{HUE_SHIFT_MIN_DEG, HUE_SHIFT_RANGE_DEG, PALETTE_COUNT, REPLAY_TAIL_BYTES};
use crate::types::AgentEvent;
use crate::watcher::agent::{Agent, ApplyOutcome};
use crate::watcher::jsonl::JsonlReader;
use crate::watcher::parser;

/// Events one agent produced in a poll, each paired with what applying it changed.
pub type AppliedEvents = Vec<(AgentEvent, ApplyOutcome)>;

/// Manages all tracked agents, their JSONL readers, and palette assignments.
#[derive(Debug)]
pub struct AgentRegistry {
    agents: Vec<Agent>,
    readers: HashMap<usize, JsonlReader>,
    next_id: usize,
    palette_usage: [u8; PALETTE_COUNT as usize],
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            readers: HashMap::new(),
            next_id: 1,
            palette_usage: [0; PALETTE_COUNT as usize],
        }
    }

    /// Track a transcript as a new agent and claim the least-used palette for it.
    ///
    /// `hash_dir` names the project until the transcript reports its working directory.
    /// Returns the assigned agent ID.
    ///
    /// # Errors
    ///
    /// Fails when the transcript cannot be opened.
    pub fn add_agent(
        &mut self,
        session_id: String,
        path: PathBuf,
        hash_dir: String,
    ) -> Result<usize> {
        let reader = JsonlReader::new_from_tail(path.clone(), REPLAY_TAIL_BYTES)?;
        let id = self.next_id;
        self.next_id += 1;

        let (palette, hue_shift) = self.claim_palette();
        self.agents.push(Agent::new(
            id, session_id, path, hash_dir, palette, hue_shift,
        ));
        self.readers.insert(id, reader);
        Ok(id)
    }

    /// Apply the transcript tail to an agent that was just added.
    ///
    /// Only the agent's state changes; the caller maps the result onto the scene.
    ///
    /// # Errors
    ///
    /// Fails when the agent is unknown or its transcript cannot be read.
    pub fn replay(&mut self, id: usize) -> Result<()> {
        let reader = self
            .readers
            .get_mut(&id)
            .ok_or_else(|| eyre!("no reader for agent {id}"))?;
        let lines = reader.read_new_lines()?;
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| eyre!("unknown agent {id}"))?;
        for event in events_of(&lines) {
            agent.apply(&event);
        }
        Ok(())
    }

    /// Remove an agent and its reader, and release its palette.
    pub fn remove_agent(&mut self, id: usize) {
        self.readers.remove(&id);
        if let Some(agent) = self.get(id) {
            let slot = &mut self.palette_usage[usize::from(agent.palette)];
            *slot = slot.saturating_sub(1);
        }
        self.agents.retain(|a| a.id != id);
    }

    /// Read new transcript lines of every agent and apply their events.
    ///
    /// Returns, per agent with new events, each event with the outcome of applying it.
    pub fn poll_all(&mut self) -> Vec<(usize, AppliedEvents)> {
        let mut results = Vec::new();
        for agent in &mut self.agents {
            let Some(reader) = self.readers.get_mut(&agent.id) else {
                continue;
            };
            let lines = match reader.read_new_lines() {
                Ok(lines) => lines,
                Err(e) => {
                    tracing::warn!(agent_id = agent.id, "transcript read failed: {e:#}");
                    continue;
                }
            };
            let applied: AppliedEvents = events_of(&lines)
                .into_iter()
                .map(|event| {
                    let outcome = agent.apply(&event);
                    (event, outcome)
                })
                .collect();
            if !applied.is_empty() {
                results.push((agent.id, applied));
            }
        }
        results
    }

    /// Get an immutable reference to an agent by ID.
    pub fn get(&self, id: usize) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Get a mutable reference to an agent by ID.
    pub fn get_mut(&mut self, id: usize) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Return a slice of all agents.
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// Check if a path is already tracked by an agent.
    pub fn has_path(&self, path: &Path) -> bool {
        self.agents.iter().any(|a| a.jsonl_path == path)
    }

    // Least-used palette; a hue shift is added once every palette is in use.
    fn claim_palette(&mut self) -> (u8, Option<i16>) {
        let min_usage = self.palette_usage.iter().copied().min().unwrap_or_default();
        let index = self
            .palette_usage
            .iter()
            .position(|&usage| usage == min_usage)
            .unwrap_or_default();
        self.palette_usage[index] += 1;

        let hue_shift = (min_usage > 0).then(|| {
            HUE_SHIFT_MIN_DEG + (rand::random::<u16>() % HUE_SHIFT_RANGE_DEG as u16) as i16
        });
        (index as u8, hue_shift)
    }
}

fn events_of(lines: &[String]) -> Vec<AgentEvent> {
    lines
        .iter()
        .filter_map(|line| parser::parse_line(line))
        .flat_map(|record| parser::extract_events(&record))
        .collect()
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::types::AgentStatus;

    fn create_test_jsonl(dir: &Path) -> PathBuf {
        let path = dir.join("test.jsonl");
        std::fs::write(&path, "{\"type\":\"system\",\"subtype\":\"init\"}\n").unwrap();
        path
    }

    fn append(path: &Path, line: &str) {
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        writeln!(file, "{line}").unwrap();
    }

    #[test]
    fn add_and_remove_agent() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut reg = AgentRegistry::new();
        let id = reg
            .add_agent("sess1".into(), path.clone(), "project".into())
            .unwrap();
        assert_eq!(reg.agents().len(), 1);
        assert!(reg.has_path(&path));
        assert_eq!(reg.get(id).unwrap().display_name(), "project");

        reg.remove_agent(id);
        assert!(reg.agents().is_empty());
        assert!(!reg.has_path(&path));
    }

    #[test]
    fn replay_applies_the_tail_so_later_polls_report_only_new_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());
        append(
            &path,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#,
        );
        let mut reg = AgentRegistry::new();
        let id = reg.add_agent("s".into(), path.clone(), "p".into()).unwrap();

        reg.replay(id).unwrap();

        assert_eq!(reg.get(id).unwrap().status, AgentStatus::Active);
        assert!(reg.poll_all().is_empty());
        append(
            &path,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#,
        );
        assert_eq!(reg.poll_all().len(), 1);
    }

    #[test]
    fn replay_of_an_unknown_agent_is_an_error() {
        let mut reg = AgentRegistry::new();
        assert!(reg.replay(7).is_err());
    }

    #[test]
    fn palettes_are_least_used_and_hue_shifted_when_exhausted() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());
        let mut reg = AgentRegistry::new();

        let mut palettes = Vec::new();
        for i in 0..=PALETTE_COUNT {
            let id = reg
                .add_agent(format!("s{i}"), path.clone(), "p".into())
                .unwrap();
            palettes.push(reg.get(id).unwrap().clone());
        }

        let distinct: std::collections::HashSet<u8> = palettes[..PALETTE_COUNT as usize]
            .iter()
            .map(|a| a.palette)
            .collect();
        assert_eq!(distinct.len(), PALETTE_COUNT as usize);
        assert!(
            palettes[..PALETTE_COUNT as usize]
                .iter()
                .all(|a| a.hue_shift.is_none())
        );
        assert!(palettes[PALETTE_COUNT as usize].hue_shift.is_some());
    }

    #[test]
    fn removing_an_agent_releases_its_palette() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());
        let mut reg = AgentRegistry::new();

        let first = reg.add_agent("a".into(), path.clone(), "p".into()).unwrap();
        let palette = reg.get(first).unwrap().palette;
        reg.remove_agent(first);

        let second = reg.add_agent("b".into(), path, "p".into()).unwrap();
        assert_eq!(reg.get(second).unwrap().palette, palette);
    }

    #[test]
    fn poll_all_applies_events_and_returns_outcomes() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());
        let mut reg = AgentRegistry::new();
        let id = reg.add_agent("s".into(), path.clone(), "p".into()).unwrap();

        append(
            &path,
            r#"{"type":"assistant","cwd":"/work/app","gitBranch":"main","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"description":"Run build"}}]}}"#,
        );
        append(&path, r#"{"type":"system","subtype":"turn_duration"}"#);

        let results = reg.poll_all();
        assert_eq!(results.len(), 1);
        let (agent_id, applied) = &results[0];
        assert_eq!(*agent_id, id);
        assert_eq!(applied.len(), 3, "identity, tool start, turn end");
        assert!(applied[2].1.turn_had_tools);

        let agent = reg.get(id).unwrap();
        assert_eq!(agent.status, AgentStatus::Waiting);
        assert_eq!(agent.display_name(), "app (main)");
        assert!(reg.poll_all().is_empty());
    }
}
