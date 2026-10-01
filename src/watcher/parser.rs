use serde_json::Value;

use crate::constants::SUMMARY_SOURCE_MAX_CHARS;
use crate::types::AgentEvent;

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
#[non_exhaustive]
pub enum RecordType {
    /// Assistant message (may contain tool_use blocks).
    Assistant,
    /// User message (may contain tool_result blocks).
    User,
    /// System message (may carry the turn_duration subtype).
    System,
    /// Generated session title.
    AiTitle,
    /// User-chosen session title.
    CustomTitle,
    /// Last user prompt of the session.
    LastPrompt,
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
    /// Tool input object.
    pub input: Value,
}

/// A tool_result block extracted from user content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    /// ID of the tool invocation this result answers.
    pub id: String,
    /// The tool reported an error.
    pub is_error: bool,
    /// The result only reports that a background agent was launched.
    pub background_launch: bool,
}

// Tools that do not trigger the permission timer.
const EXEMPT_TOOLS: &[&str] = &["Task", "Agent", "AskUserQuestion"];

// Text of the tool result that reports a background agent launch.
const ASYNC_LAUNCH_MARKER: &str = "Async agent launched successfully.";

/// Parse a single JSONL line into a record.
///
/// Returns `None` if the line is not valid JSON or lacks a `type` field.
pub fn parse_line(line: &str) -> Option<JsonlRecord> {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => {
            tracing::debug!("skipping malformed transcript line: {e}");
            return None;
        }
    };
    let type_str = value.get("type")?.as_str()?;

    let record_type = match type_str {
        "assistant" => RecordType::Assistant,
        "user" | "human" => RecordType::User,
        "system" => RecordType::System,
        "ai-title" => RecordType::AiTitle,
        "custom-title" => RecordType::CustomTitle,
        "last-prompt" => RecordType::LastPrompt,
        _ => RecordType::Unknown,
    };

    Some(JsonlRecord { record_type, value })
}

/// Extract all agent events from a single JSONL record.
///
/// `Identity` comes first, then content events in block order.
pub fn extract_events(record: &JsonlRecord) -> Vec<AgentEvent> {
    let value = &record.value;
    let mut events = Vec::new();

    if let Some(identity) = identity_event(value) {
        events.push(identity);
    }

    match record.record_type {
        RecordType::Assistant => assistant_events(value, &mut events),
        RecordType::User => user_events(value, &mut events),
        RecordType::System => {
            if is_turn_end(value) {
                events.push(AgentEvent::TurnEnd);
            }
        }
        RecordType::AiTitle => title_event(value, "aiTitle", false, &mut events),
        RecordType::CustomTitle => title_event(value, "customTitle", true, &mut events),
        RecordType::LastPrompt => {
            if let Some(text) = str_field(value, "lastPrompt") {
                events.push(AgentEvent::Prompt {
                    text: one_line(text),
                });
            }
        }
        RecordType::Unknown => {}
    }

    events
}

fn identity_event(value: &Value) -> Option<AgentEvent> {
    let cwd = str_field(value, "cwd")?;
    Some(AgentEvent::Identity {
        cwd: cwd.to_owned(),
        git_branch: str_field(value, "gitBranch").unwrap_or_default().to_owned(),
    })
}

fn title_event(value: &Value, field: &str, custom: bool, events: &mut Vec<AgentEvent>) {
    if let Some(title) = str_field(value, field) {
        events.push(AgentEvent::Title {
            title: one_line(title),
            custom,
        });
    }
}

fn assistant_events(value: &Value, events: &mut Vec<AgentEvent>) {
    for block in content_blocks(value) {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => {
                if let Some(tool) = tool_use_block(block) {
                    let detail = tool_detail(&tool.name, &tool.input);
                    events.push(AgentEvent::ToolStart {
                        tool_id: tool.id,
                        tool_name: tool.name,
                        detail,
                    });
                }
            }
            Some("text") => {
                let text = block.get("text").and_then(Value::as_str);
                if let Some(first_line) = text.and_then(first_non_empty_line) {
                    events.push(AgentEvent::Text { first_line });
                }
            }
            _ => {}
        }
    }
}

fn user_events(value: &Value, events: &mut Vec<AgentEvent>) {
    for result in extract_tool_results(value) {
        events.push(AgentEvent::ToolDone {
            tool_id: result.id,
            is_error: result.is_error,
            background_launch: result.background_launch,
        });
    }
    if let Some(text) = prompt_text(value) {
        events.push(AgentEvent::Prompt { text });
    }
}

/// Text of a typed user prompt; none for meta records and injected `<...>` messages.
fn prompt_text(value: &Value) -> Option<String> {
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let content = value.get("message")?.get("content")?;
    let raw = match content {
        Value::String(text) => text.as_str(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .find_map(|b| b.get("text").and_then(Value::as_str))?,
        _ => return None,
    };
    let text = one_line(raw);
    (!text.is_empty() && !raw.trim_start().starts_with('<')).then_some(text)
}

fn content_blocks(value: &Value) -> &[Value] {
    value
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn tool_use_block(block: &Value) -> Option<ToolUse> {
    let id = str_field(block, "id")?;
    let name = str_field(block, "name")?;
    Some(ToolUse {
        id: id.to_owned(),
        name: name.to_owned(),
        input: block.get("input").cloned().unwrap_or(Value::Null),
    })
}

/// Extract tool_use blocks from an assistant message's content array.
pub fn extract_tool_use(value: &Value) -> Vec<ToolUse> {
    content_blocks(value)
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
        .filter_map(tool_use_block)
        .collect()
}

/// Extract tool_result blocks from a user message's content array.
pub fn extract_tool_results(value: &Value) -> Vec<ToolResult> {
    content_blocks(value)
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
        .filter_map(|block| {
            Some(ToolResult {
                id: str_field(block, "tool_use_id")?.to_owned(),
                is_error: block.get("is_error").and_then(Value::as_bool) == Some(true),
                background_launch: is_background_launch(block),
            })
        })
        .collect()
}

// A result's content is a plain string or an array of text blocks.
fn is_background_launch(block: &Value) -> bool {
    match block.get("content") {
        Some(Value::String(text)) => text.contains(ASYNC_LAUNCH_MARKER),
        Some(Value::Array(parts)) => parts.iter().any(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains(ASYNC_LAUNCH_MARKER))
        }),
        _ => false,
    }
}

/// One-line description of a tool input, used in the activity summary.
pub fn tool_detail(name: &str, input: &Value) -> String {
    let keys: &[&str] = match name {
        "Bash" => &["description", "command"],
        "Agent" | "Task" => &["description", "prompt"],
        "Read" | "Edit" | "Write" | "NotebookEdit" => &["file_path"],
        "Glob" | "Grep" => &["pattern"],
        _ => &[],
    };
    keys.iter()
        .find_map(|key| str_field(input, key))
        .map(one_line)
        .unwrap_or_default()
}

/// Returns true if the record is a system message with subtype `turn_duration`.
pub fn is_turn_end(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("system")
        && value.get("subtype").and_then(Value::as_str) == Some("turn_duration")
}

/// Returns true if the tool name is non-exempt (triggers permission timer).
pub fn is_non_exempt_tool(name: &str) -> bool {
    !EXEMPT_TOOLS.contains(&name)
}

/// Prefix of at most `max` chars, never inside a multibyte character.
pub fn truncate_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

// Collapse line breaks and runs of whitespace, then cap the length.
fn one_line(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&joined, SUMMARY_SOURCE_MAX_CHARS)
}

fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(one_line)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn events_of(line: &str) -> Vec<AgentEvent> {
        extract_events(&parse_line(line).expect("valid record"))
    }

    #[test]
    fn parse_assistant_line() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello"}]}}"#;
        assert_eq!(parse_line(line).unwrap().record_type, RecordType::Assistant);
    }

    #[test]
    fn parse_invalid_json_returns_none() {
        assert!(parse_line("not json").is_none());
        assert!(parse_line("{}").is_none()); // no type field
    }

    #[rstest]
    #[case("ai-title", RecordType::AiTitle)]
    #[case("custom-title", RecordType::CustomTitle)]
    #[case("last-prompt", RecordType::LastPrompt)]
    #[case("human", RecordType::User)]
    #[case("progress", RecordType::Unknown)]
    fn record_type_classification(#[case] type_name: &str, #[case] expected: RecordType) {
        let line = format!(r#"{{"type":"{type_name}"}}"#);
        assert_eq!(parse_line(&line).unwrap().record_type, expected);
    }

    #[test]
    fn extract_tool_use_blocks() {
        let val = json!({"message":{"content":[
            {"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}},
            {"type":"text","text":"hi"}]}});
        let tools = extract_tool_use(&val);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "t1");
        assert_eq!(tools[0].name, "Bash");
        assert_eq!(tools[0].input["command"], "ls");
    }

    #[test]
    fn extract_tool_result_blocks() {
        let val = json!({"message":{"content":[
            {"type":"tool_result","tool_use_id":"t1"},
            {"type":"tool_result","tool_use_id":"t2","is_error":true}]}});
        let results = extract_tool_results(&val);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "t1");
        assert!(!results[0].is_error);
        assert!(results[1].is_error);
    }

    #[rstest]
    #[case(json!("Async agent launched successfully.\nagentId: a1"), true)]
    #[case(json!([{"type":"text","text":"Async agent launched successfully."}]), true)]
    #[case(json!("done"), false)]
    #[case(json!([{"type":"text","text":"done"}]), false)]
    fn background_launch_detection(#[case] content: Value, #[case] expected: bool) {
        let val = json!({"message":{"content":[
            {"type":"tool_result","tool_use_id":"t1","content":content}]}});
        assert_eq!(extract_tool_results(&val)[0].background_launch, expected);
    }

    #[test]
    fn turn_end_detection_reads_the_top_level_subtype() {
        // Real record shape: `subtype` is top-level, `message` is absent.
        let real = json!({"type":"system","subtype":"turn_duration","durationMs":1523,
            "messageCount":4,"cwd":"/work/app","gitBranch":"main"});
        assert!(is_turn_end(&real));

        let other = json!({"type":"system","subtype":"stop_hook_summary"});
        assert!(!is_turn_end(&other));

        let nested = json!({"message":{"subtype":"turn_duration"}});
        assert!(!is_turn_end(&nested));
    }

    #[test]
    fn exempt_tool_check() {
        assert!(!is_non_exempt_tool("Task"));
        assert!(!is_non_exempt_tool("Agent"));
        assert!(!is_non_exempt_tool("AskUserQuestion"));
        assert!(is_non_exempt_tool("Bash"));
        assert!(is_non_exempt_tool("Read"));
    }

    #[rstest]
    #[case("Bash", json!({"description":"List files","command":"ls -la"}), "List files")]
    #[case("Bash", json!({"command":"ls\n  -la"}), "ls -la")]
    #[case("Agent", json!({"description":"Find callers","prompt":"long"}), "Find callers")]
    #[case("Task", json!({"prompt":"do it"}), "do it")]
    #[case("Read", json!({"file_path":"/src/main.rs"}), "/src/main.rs")]
    #[case("Edit", json!({"file_path":"/a.rs","old_string":"x"}), "/a.rs")]
    #[case("Grep", json!({"pattern":"fn main"}), "fn main")]
    #[case("WebFetch", json!({"url":"https://example.com"}), "")]
    #[case("Bash", json!(null), "")]
    fn tool_detail_rules(#[case] name: &str, #[case] input: Value, #[case] expected: &str) {
        assert_eq!(tool_detail(name, &input), expected);
    }

    #[test]
    fn tool_detail_is_capped_on_a_char_boundary() {
        let long = "字".repeat(SUMMARY_SOURCE_MAX_CHARS + 50);
        let detail = tool_detail("Bash", &json!({"command": long}));
        assert_eq!(detail.chars().count(), SUMMARY_SOURCE_MAX_CHARS);
    }

    #[test]
    fn assistant_record_yields_identity_then_blocks_in_order() {
        let line = r#"{"type":"assistant","cwd":"/work/app","gitBranch":"main","message":{"content":[
            {"type":"thinking","thinking":"hmm"},
            {"type":"text","text":"\nChecking the build.\nMore."},
            {"type":"tool_use","id":"t1","name":"Bash","input":{"description":"Run build"}}]}}"#;
        assert_eq!(
            events_of(line),
            vec![
                AgentEvent::Identity {
                    cwd: "/work/app".into(),
                    git_branch: "main".into()
                },
                AgentEvent::Text {
                    first_line: "Checking the build.".into()
                },
                AgentEvent::ToolStart {
                    tool_id: "t1".into(),
                    tool_name: "Bash".into(),
                    detail: "Run build".into()
                },
            ]
        );
    }

    #[test]
    fn thinking_only_record_yields_no_events() {
        let line =
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hmm"}]}}"#;
        assert!(events_of(line).is_empty());
    }

    #[test]
    fn user_tool_result_yields_tool_done() {
        let line = r#"{"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#;
        assert_eq!(
            events_of(line),
            vec![AgentEvent::ToolDone {
                tool_id: "t1".into(),
                is_error: false,
                background_launch: false
            }]
        );
    }

    #[rstest]
    #[case(
        r#"{"type":"user","message":{"content":"Fix the login bug"}}"#,
        Some("Fix the login bug")
    )]
    #[case(
        r#"{"type":"user","message":{"content":[{"type":"text","text":"Fix\nthe bug"}]}}"#,
        Some("Fix the bug")
    )]
    #[case(
        r#"{"type":"user","isMeta":true,"message":{"content":"Caveat: local command"}}"#,
        None
    )]
    #[case(
        r#"{"type":"user","message":{"content":"<command-name>/clear</command-name>"}}"#,
        None
    )]
    fn user_prompt_rules(#[case] line: &str, #[case] expected: Option<&str>) {
        let prompt = events_of(line).into_iter().find_map(|e| match e {
            AgentEvent::Prompt { text } => Some(text),
            _ => None,
        });
        assert_eq!(prompt.as_deref(), expected);
    }

    #[rstest]
    #[case(r#"{"type":"ai-title","aiTitle":"Fix login"}"#, Some(("Fix login", false)))]
    #[case(r#"{"type":"custom-title","customTitle":"My title"}"#, Some(("My title", true)))]
    #[case(r#"{"type":"ai-title"}"#, None)]
    fn title_records(#[case] line: &str, #[case] expected: Option<(&str, bool)>) {
        let expected = expected.map(|(t, c)| AgentEvent::Title {
            title: t.into(),
            custom: c,
        });
        assert_eq!(events_of(line).into_iter().next(), expected);
    }

    #[rstest]
    #[case(r#"{"type":"last-prompt","lastPrompt":"Add tests"}"#, 1)]
    #[case(r#"{"type":"last-prompt"}"#, 0)]
    fn last_prompt_is_optional(#[case] line: &str, #[case] count: usize) {
        assert_eq!(events_of(line).len(), count);
    }

    #[test]
    fn turn_end_record_yields_turn_end() {
        let line = r#"{"type":"system","subtype":"turn_duration","durationMs":10}"#;
        assert_eq!(events_of(line), vec![AgentEvent::TurnEnd]);
    }

    #[test]
    fn record_without_content_yields_only_identity() {
        let line = r#"{"type":"assistant","cwd":"/w","gitBranch":"dev"}"#;
        assert_eq!(
            events_of(line),
            vec![AgentEvent::Identity {
                cwd: "/w".into(),
                git_branch: "dev".into()
            }]
        );
    }

    #[test]
    fn truncate_chars_never_splits_a_character() {
        assert_eq!(truncate_chars("你好世界", 2), "你好");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }
}
