# Parser Design

Pure translation of one transcript line into `AgentEvent`s. Domain contract: `model.md` Cross-Structure Contracts. State handling: `watcher-design.md`.

## Design Overview

- **Classes**: `JsonlRecord`, `RecordType`, `ToolUse`; free functions in `watcher/parser.rs`.
- **Relationships**: `parse_line` creates `JsonlRecord`; `extract_events` reads it and creates `AgentEvent` (`types.rs`).
- **Role**: helper. Stateless; no `Agent`, timer, or I/O access.

## Class / Type Specifications

```rust
pub struct JsonlRecord { pub record_type: RecordType, pub value: serde_json::Value }

pub enum RecordType { Assistant, User, System, AiTitle, CustomTitle, LastPrompt, Unknown }

pub struct ToolUse { pub id: String, pub name: String, pub input: serde_json::Value }
```

`RecordType` is `#[non_exhaustive]`, derives `Debug, Clone, Copy, PartialEq, Eq`. Removed: `Progress` (Claude Code writes no progress records). `"user"` and `"human"` both map to `User`.

## Function Specifications

**parse_line(line: &str) -> Option<JsonlRecord>** — Parses with `serde_json::from_str::<Value>`. Returns `None` for blank or malformed lines. A line that fails to parse is logged at `debug` and skipped.

**extract_events(record: &JsonlRecord) -> Vec<AgentEvent>** — Event order within one Record: `Identity` first, then content events in block order.

| Record | Events |
|--------|--------|
| any with `cwd` | `Identity { cwd, git_branch }` (`gitBranch`, empty when absent) |
| Assistant, block `tool_use` | `ToolStart { tool_id, tool_name, detail }` |
| Assistant, block `text` | `Text { first_line }`, first non-empty line, capped at `SUMMARY_SOURCE_MAX_CHARS` |
| Assistant, block `thinking` | none |
| User, block `tool_result` | `ToolDone { tool_id, is_error, background_launch }` |
| User, prompt text | `Prompt { text }`; skipped when `isMeta` is true or the text starts with `<` |
| System, top-level `subtype == "turn_duration"` | `TurnEnd` |
| `ai-title` (`aiTitle`) | `Title { title, custom: false }` |
| `custom-title` | `Title { title, custom: true }` |
| `last-prompt` (`lastPrompt`) | `Prompt { text }` |

**extract_tool_use(value: &Value) -> Vec<ToolUse>** — `tool_use` blocks of `message.content`.

**extract_tool_results(value: &Value) -> Vec<ToolResult>** — one `ToolResult { id, is_error, background_launch }` per `tool_result` block. `background_launch` is true when the block content (string, or array of text blocks) contains `Async agent launched successfully.`. Replaces `extract_tool_result`, `is_background_agent_result`.

**tool_detail(name: &str, input: &Value) -> String** — Bash: `description` else `command`. Agent and Task: `description` else `prompt`. Read, Edit, Write, NotebookEdit: `file_path`. Glob, Grep: `pattern`. Others: empty. Line breaks collapse to single spaces; capped at `SUMMARY_SOURCE_MAX_CHARS` on a char boundary.

**is_turn_end(value: &Value) -> bool** — Top-level `type == "system"` and top-level `subtype == "turn_duration"`. Fix: the old code read `message.subtype`, which real records never carry.

**is_non_exempt_tool(name: &str) -> bool** — False for `Task`, `Agent`, `AskUserQuestion`.

**truncate_chars(text: &str, max: usize) -> String** — Prefix of at most `max` chars, never inside a multibyte character.

Removed: `BashProgress`, `SubAgent*` handling, the `Agent` parameter of `extract_events`.

## Error Handling

Parsing never fails the caller: unknown shapes yield no events. A record missing `message.content` yields only `Identity`.
