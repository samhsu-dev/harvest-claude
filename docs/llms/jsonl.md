# JSONL Protocol

> Claude Code transcript record types, event extraction, and reader limits.

## Quick Start

```rust
let mut reader = JsonlReader::new_from_tail(path, REPLAY_TAIL_BYTES)?;
for line in reader.read_new_lines()? {
    for event in extract_events(&line) {   // stateless
        agent.apply(event);
    }
}
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/watcher/jsonl.rs` | `JsonlReader`: byte-level tailing |
| `src/watcher/parser.rs` | `parse_line`, `extract_events`, `tool_detail`, `is_turn_end`, `is_non_exempt_tool`, `truncate_chars` |

## API

| Item | Meaning |
|------|---------|
| `JsonlReader::new_from_tail(path, tail)` | Start `tail` bytes before the end; discard the cut first line |
| `JsonlReader::new_from_start(path)` | Start at byte 0 |
| `read_new_lines()` | Complete lines since the last call, at most `JSONL_READ_CAP` bytes per call |
| `offset()` | Current byte offset |

`AgentEvent`: `ToolStart`, `ToolDone` (with `background_launch` flag), `TurnEnd`, `Text`, `Title`, `Prompt`, `Identity`.

## Records

Location: `~/.claude/projects/<hash>/<session-id>.jsonl`. Append-only, one JSON object per line.

| `type` | Content | Event |
|--------|---------|-------|
| `assistant` | `tool_use` blocks (`id`, `name`, `input`) | `ToolStart` |
| `assistant` | `text` blocks | `Text` |
| `user` | `tool_result` blocks (`tool_use_id`) | `ToolDone` |
| `user` | string or array prompt content | `Prompt` |
| `system` | top-level `subtype: "turn_duration"` | `TurnEnd` |
| `ai-title`, `custom-title` | session title | `Title` |
| `last-prompt` | last user prompt | `Prompt` |
| any record with `cwd` and `gitBranch` | project identity | `Identity` |

- `thinking` blocks are ignored.
- A `tool_result` reading "Async agent launched successfully." sets `background_launch`. The final result of that tool id ends the companion.

## Tool Animation

| Animation | Tools |
|-----------|-------|
| Farm | `Write`, `Edit`, `MultiEdit`, `NotebookEdit`, unknown tools |
| Harvest | `Read`, `Grep`, `Glob`, `WebFetch`, `WebSearch`, `Bash`, `Execute`, `Terminal` |

Exempt tools never start the permission timer: `Task`, `Agent`, `AskUserQuestion`.

## Configuration

| Constant | Value |
|----------|-------|
| `JSONL_READ_CAP` | 65536 bytes per read |
| `JSONL_MAX_LINE_BYTES` | 8 MiB |
| `REPLAY_TAIL_BYTES` | 65536 |
| `JSONL_POLL_MS` | 500 |

## Gotchas

- Buffer raw bytes and decode only complete lines. A multibyte character split across reads stays intact.
- A line above `JSONL_MAX_LINE_BYTES` is skipped without growing the buffer.
- A file shorter than the stored offset resets the reader to byte 0.
- Turn end is the top-level `subtype: "turn_duration"`. Text-only turns lack it, so the 5 s text-idle timer is the fallback.
- User `content` is a string or an array. Check the type before access.
- `extract_events` keeps no state. Per-tool tracking lives in `Agent::apply`.
