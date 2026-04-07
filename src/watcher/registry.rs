use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use color_eyre::eyre::Result;

use crate::constants::PALETTE_COUNT;
use crate::types::{AgentEvent, AgentStatus};
use crate::watcher::jsonl::JsonlReader;
use crate::watcher::parser;

/// A tracked Claude Code agent session.
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique agent ID (positive for top-level, negative for sub-agents).
    pub id: usize,
    /// Session identifier from the JSONL path.
    pub session_id: String,
    /// Path to the JSONL file being tailed.
    pub jsonl_path: PathBuf,
    /// Human-readable project name.
    pub project_name: String,
    /// Current inferred agent status.
    pub status: AgentStatus,
    /// Currently active tools: tool_id -> tool_name.
    pub active_tools: HashMap<String, String>,
    /// Whether any tools were used in the current turn.
    pub had_tools_in_turn: bool,
    /// Parent agent ID for sub-agents.
    pub parent_id: Option<usize>,
    /// Tool IDs for background (async) agents that survive turn boundaries.
    pub background_tool_ids: HashSet<String>,
    /// Sub-agent tool tracking on parent: tool_id -> tool_name.
    pub active_sub_tool_names: HashMap<String, String>,
}

/// Manages all tracked agents, their JSONL readers, and palette assignments.
#[derive(Debug)]
pub struct AgentRegistry {
    agents: Vec<Agent>,
    readers: HashMap<usize, JsonlReader>,
    next_id: usize,
    palette_usage: [u8; PALETTE_COUNT as usize],
    sub_agent_map: HashMap<String, usize>,
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            readers: HashMap::new(),
            next_id: 1,
            palette_usage: [0; PALETTE_COUNT as usize],
            sub_agent_map: HashMap::new(),
        }
    }

    /// Add a new top-level agent and its JSONL reader.
    ///
    /// Returns the assigned agent ID.
    pub fn add_agent(
        &mut self,
        session_id: String,
        path: PathBuf,
        project_name: String,
    ) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let reader = JsonlReader::new(path.clone())?;

        let agent = Agent {
            id,
            session_id,
            jsonl_path: path,
            project_name,
            status: AgentStatus::Active,
            active_tools: HashMap::new(),
            had_tools_in_turn: false,
            parent_id: None,
            background_tool_ids: HashSet::new(),
            active_sub_tool_names: HashMap::new(),
        };

        self.agents.push(agent);
        self.readers.insert(id, reader);

        // Palette usage incremented (assignment happens externally via assign_palette).
        Ok(id)
    }

    /// Remove an agent and its reader, cleaning up sub-agents.
    pub fn remove_agent(&mut self, id: usize) {
        self.readers.remove(&id);

        // Remove sub-agents that belong to this agent.
        let sub_keys: Vec<String> = self
            .sub_agent_map
            .iter()
            .filter(|(_, sub_id)| {
                let sub_id = **sub_id;
                self.agents
                    .iter()
                    .any(|a| a.id == sub_id && a.parent_id == Some(id))
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in &sub_keys {
            if let Some(sub_id) = self.sub_agent_map.remove(key) {
                self.readers.remove(&sub_id);
                self.agents.retain(|a| a.id != sub_id);
            }
        }

        self.agents.retain(|a| a.id != id);
    }

    /// Poll all agents for new JSONL lines and parse them into events.
    pub fn poll_all(&mut self) -> Vec<(usize, Vec<AgentEvent>)> {
        let mut results = Vec::new();

        let agent_ids: Vec<usize> = self.agents.iter().map(|a| a.id).collect();

        for agent_id in agent_ids {
            let lines = match self.readers.get_mut(&agent_id) {
                Some(reader) => match reader.read_new_lines() {
                    Ok(lines) => lines,
                    Err(_) => continue,
                },
                None => continue,
            };

            if lines.is_empty() {
                continue;
            }

            let mut events = Vec::new();
            for line in &lines {
                let Some(record) = parser::parse_line(line) else {
                    continue;
                };

                // Need an immutable reference to the agent for parsing.
                let agent_idx = match self.agents.iter().position(|a| a.id == agent_id) {
                    Some(idx) => idx,
                    None => continue,
                };

                let line_events = parser::extract_events(&record, &self.agents[agent_idx]);
                events.extend(line_events);
            }

            if !events.is_empty() {
                results.push((agent_id, events));
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

    /// Check if a path is already registered as any agent (top-level or sub-agent).
    pub fn has_path(&self, path: &std::path::Path) -> bool {
        self.agents.iter().any(|a| a.jsonl_path == path)
    }

    /// Check if an agent ID belongs to a sub-agent (has a parent).
    pub fn is_sub_agent(&self, id: usize) -> bool {
        self.agents
            .iter()
            .any(|a| a.id == id && a.parent_id.is_some())
    }

    /// Assign a palette index using least-used strategy.
    ///
    /// Returns `(palette_index, optional_hue_shift)`. A hue shift is provided
    /// when all palettes have been used at least once.
    pub fn assign_palette(&mut self) -> (u8, Option<i16>) {
        let min_usage = self.palette_usage.iter().copied().min().unwrap_or_default();

        let palette_idx = self
            .palette_usage
            .iter()
            .position(|&u| u == min_usage)
            .unwrap_or_default() as u8;

        self.palette_usage[palette_idx as usize] += 1;

        let hue_shift = if min_usage > 0 {
            // All palettes used at least once — apply hue shift.
            use crate::constants::{HUE_SHIFT_MIN_DEG, HUE_SHIFT_RANGE_DEG};
            let shift =
                HUE_SHIFT_MIN_DEG + (rand::random::<u16>() % HUE_SHIFT_RANGE_DEG as u16) as i16;
            Some(shift)
        } else {
            None
        };

        (palette_idx, hue_shift)
    }

    /// Add a sub-agent spawned by a parent tool invocation.
    ///
    /// The sub-agent key is `"parentId:parentToolId"`. Returns the new sub-agent ID.
    pub fn add_sub_agent(
        &mut self,
        parent_tool_id: &str,
        parent_id: usize,
        path: PathBuf,
    ) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let reader = JsonlReader::new_from_start(path.clone())?;

        let parent_project = self
            .get(parent_id)
            .map(|a| a.project_name.clone())
            .unwrap_or_default();

        let parent_session = self
            .get(parent_id)
            .map(|a| a.session_id.clone())
            .unwrap_or_default();

        let agent = Agent {
            id,
            session_id: parent_session,
            jsonl_path: path,
            project_name: parent_project,
            status: AgentStatus::Active,
            active_tools: HashMap::new(),
            had_tools_in_turn: false,
            parent_id: Some(parent_id),
            background_tool_ids: HashSet::new(),
            active_sub_tool_names: HashMap::new(),
        };

        self.agents.push(agent);
        self.readers.insert(id, reader);

        let key = format!("{parent_id}:{parent_tool_id}");
        self.sub_agent_map.insert(key, id);

        Ok(id)
    }

    /// Remove a sub-agent by its parent tool ID.
    pub fn remove_sub_agent(&mut self, parent_tool_id: &str) {
        // Find the key that ends with the parent_tool_id.
        let key = self
            .sub_agent_map
            .keys()
            .find(|k| k.ends_with(&format!(":{parent_tool_id}")))
            .cloned();

        let Some(key) = key else { return };
        let Some(sub_id) = self.sub_agent_map.remove(&key) else {
            return;
        };
        self.readers.remove(&sub_id);
        self.agents.retain(|a| a.id != sub_id);
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_jsonl(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("test.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"{\"type\":\"system\",\"message\":{\"subtype\":\"init\"}}\n")
            .unwrap();
        path
    }

    #[test]
    fn add_and_remove_agent() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_jsonl(dir.path());

        let mut reg = AgentRegistry::new();
        let id = reg
            .add_agent("sess1".into(), path, "project".into())
            .unwrap();
        assert_eq!(reg.agents().len(), 1);
        assert!(reg.get(id).is_some());

        reg.remove_agent(id);
        assert!(reg.agents().is_empty());
    }

    #[test]
    fn palette_assignment_least_used() {
        let mut reg = AgentRegistry::new();
        let (p1, shift1) = reg.assign_palette();
        assert_eq!(p1, 0);
        assert!(shift1.is_none());

        // Fill all palettes.
        for _ in 1..PALETTE_COUNT {
            reg.assign_palette();
        }

        // Next assignment should have a hue shift.
        let (_, shift) = reg.assign_palette();
        assert!(shift.is_some());
    }
}
