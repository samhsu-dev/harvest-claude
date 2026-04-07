use serde_json::Value;

use crate::types::AgentEvent;
use crate::watcher::registry::Agent;

/// Parsed JSONL record with its type and raw JSON value.
#[derive(Debug, Clone)]
pub struct JsonlRecord {
    /// Classified record type.
    pub record_type: RecordType,
    /// Raw JSON value.
    pub value: Value,
}

/// Classification of a JSONL record by its `type` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// Assistant message (may contain tool_use blocks).
    Assistant,
    /// User message (may contain tool_result blocks).
    User,
    /// System message (may contain turn_duration subtype).
    System,
    /// Progress update (bash/mcp execution output).
    Progress,
    /// Unrecognized record type.
    Unknown,
}

/// A tool_use block extracted from assistant content.
#[derive(Debug, Clone)]
pub struct ToolUse {
    /// Tool invocation ID.
    pub id: String,
    /// Tool name.
    pub name: String,
}

// Tools that do not trigger the permission timer.
const EXEMPT_TOOLS: &[&str] = &["Task", "Agent", "AskUserQuestion"];

/// Parse a single JSONL line into a record.
///
/// Returns `None` if the line is not valid JSON or lacks a `type` field.
pub fn parse_line(line: &str) -> Option<JsonlRecord> {
    let value: Value = serde_json::from_str(line).ok()?;
    let type_str = value.get("type")?.as_str()?;

    let record_type = match type_str {
        "assistant" => RecordType::Assistant,
        "user" | "human" => RecordType::User,
        "system" => RecordType::System,
        "progress" => RecordType::Progress,
        _ => RecordType::Unknown,
    };

    Some(JsonlRecord { record_type, value })
}

/// Extract all agent events from a single JSONL record.
pub fn extract_events(record: &JsonlRecord, agent: &Agent) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    match record.record_type {
        RecordType::Assistant => {
            let tools = extract_tool_use(&record.value);
            if tools.is_empty() {
                // Text-only assistant message — emit TextOnly only if no tools in this turn.
                if !agent.had_tools_in_turn {
                    events.push(AgentEvent::TextOnly);
                }
            } else {
                for tool in tools {
                    events.push(AgentEvent::ToolStart {
                        tool_id: tool.id,
                        tool_name: tool.name,
                    });
                }
            }
        }
        RecordType::User => {
            let tool_ids = extract_tool_result(&record.value);
            for tool_id in &tool_ids {
                // Check for background agent detection.
                if is_background_agent_result(&record.value, tool_id) {
                    events.push(AgentEvent::BackgroundAgentDetected {
                        tool_id: tool_id.clone(),
                    });
                }
                events.push(AgentEvent::ToolDone {
                    tool_id: tool_id.clone(),
                });
            }
        }
        RecordType::System => {
            if is_turn_end(&record.value) {
                events.push(AgentEvent::TurnEnd);
            }
        }
        RecordType::Progress => {
            // Extract tool_id from progress records to restart permission timer.
            if let Some(tool_id) = record.value.get("tool_use_id").and_then(Value::as_str) {
                events.push(AgentEvent::BashProgress {
                    tool_id: tool_id.to_owned(),
                });
            }
        }
        RecordType::Unknown => {}
    }

    events
}

/// Extract tool_use blocks from an assistant message's content array.
pub fn extract_tool_use(value: &Value) -> Vec<ToolUse> {
    let mut tools = Vec::new();

    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return tools,
    };

    let items = match content.as_array() {
        Some(arr) => arr,
        None => return tools,
    };

    for item in items {
        if item.get("type").and_then(Value::as_str) == Some("tool_use") {
            let id = item.get("id").and_then(Value::as_str).unwrap_or_default();
            let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
            if !id.is_empty() && !name.is_empty() {
                tools.push(ToolUse {
                    id: id.to_owned(),
                    name: name.to_owned(),
                });
            }
        }
    }

    tools
}

/// Extract completed tool IDs from a user message's content array.
pub fn extract_tool_result(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();

    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return ids,
    };

    let items = match content.as_array() {
        Some(arr) => arr,
        None => return ids,
    };

    for item in items {
        let is_tool_result = item.get("type").and_then(Value::as_str) == Some("tool_result");
        if !is_tool_result {
            continue;
        }
        if let Some(id) = item
            .get("tool_use_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            ids.push(id.to_owned());
        }
    }

    ids
}

/// Returns true if the record is a system message with subtype `turn_duration`.
pub fn is_turn_end(value: &Value) -> bool {
    value
        .get("message")
        .and_then(|m| m.get("subtype"))
        .and_then(Value::as_str)
        == Some("turn_duration")
}

/// Returns true if the tool name is non-exempt (triggers permission timer).
pub fn is_non_exempt_tool(name: &str) -> bool {
    !EXEMPT_TOOLS.contains(&name)
}

// Check if a tool_result contains "Async agent launched successfully."
fn is_background_agent_result(value: &Value, tool_id: &str) -> bool {
    let content = match value.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return false,
    };

    let items = match content.as_array() {
        Some(arr) => arr,
        None => return false,
    };

    const ASYNC_MARKER: &str = "Async agent launched successfully.";

    for item in items {
        let is_match = item.get("type").and_then(Value::as_str) == Some("tool_result")
            && item.get("tool_use_id").and_then(Value::as_str) == Some(tool_id);
        if !is_match {
            continue;
        }

        // Content as a plain string.
        if item
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|t| t.contains(ASYNC_MARKER))
        {
            return true;
        }

        // Content as an array of text blocks.
        let Some(arr) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for block in arr {
            if block
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains(ASYNC_MARKER))
            {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assistant_line() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello"}]}}"#;
        let record = parse_line(line).unwrap();
        assert_eq!(record.record_type, RecordType::Assistant);
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_line("not json").is_none());
        assert!(parse_line("{}").is_none()); // no type field
    }

    #[test]
    fn extract_tool_use_blocks() {
        let val: Value = serde_json::from_str(
            r#"{"message":{"content":[{"type":"tool_use","id":"t1","name":"Bash"},{"type":"text","text":"hi"}]}}"#,
        ).unwrap();
        let tools = extract_tool_use(&val);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "t1");
        assert_eq!(tools[0].name, "Bash");
    }

    #[test]
    fn extract_tool_result_ids() {
        let val: Value = serde_json::from_str(
            r#"{"message":{"content":[{"type":"tool_result","tool_use_id":"t1"},{"type":"tool_result","tool_use_id":"t2"}]}}"#,
        ).unwrap();
        let ids = extract_tool_result(&val);
        assert_eq!(ids, vec!["t1", "t2"]);
    }

    #[test]
    fn turn_end_detection() {
        let val: Value =
            serde_json::from_str(r#"{"message":{"subtype":"turn_duration"}}"#).unwrap();
        assert!(is_turn_end(&val));

        let val2: Value = serde_json::from_str(r#"{"message":{"subtype":"other"}}"#).unwrap();
        assert!(!is_turn_end(&val2));
    }

    #[test]
    fn exempt_tool_check() {
        assert!(!is_non_exempt_tool("Task"));
        assert!(!is_non_exempt_tool("Agent"));
        assert!(!is_non_exempt_tool("AskUserQuestion"));
        assert!(is_non_exempt_tool("Bash"));
        assert!(is_non_exempt_tool("Read"));
    }
}
