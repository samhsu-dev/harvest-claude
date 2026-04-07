# JSONL Protocol

> Claude Code transcript record types, parsing rules, agent status derivation.

## File Location

`~/.claude/projects/<hash>/<session-id>.jsonl`

- `<hash>` = workspace path with `:`, `\`, `/` replaced by `-`.
- `<session-id>` = UUID.
- One file per Claude Code session. Append-only.

## Record Types

Each line is a self-contained JSON object with a `type` field.

### assistant

```json
{"type": "assistant", "message": {"content": [
  {"type": "tool_use", "id": "toolu_xxx", "name": "Read", "input": {...}},
  {"type": "text", "text": "..."}
]}}
```

- `tool_use` blocks → agent tool start. Extract `id` and `name`.
- `thinking` blocks → ignored.

### user

```json
{"type": "user", "message": {"content": [
  {"type": "tool_result", "tool_use_id": "toolu_xxx", "content": "..."}
]}}
```

- `tool_result` → agent tool done. Extract `tool_use_id`.
- `content` can be string (text prompt) or array (tool results). Handle both.

### system

```json
{"type": "system", "subtype": "turn_duration", "duration_ms": 1234}
```

- `subtype == "turn_duration"` → reliable turn-end signal (~98% of tool-using turns).
- Clear all active tools on this record. Character → IDLE.
- Never emitted for text-only turns. Use text-idle timer (5s) as fallback.

### progress

```json
{"type": "progress", "data": {"type": "agent_progress", ...}}
```

- `data.type == "agent_progress"` → sub-agent tool activity.
- `data.type == "bash_progress"` → long-running Bash output (confirms tool is executing).

## Status Derivation

```
No active tools + no recent data (>5s)     → Idle
Active tool_use without tool_result         → Active (typing/reading)
Active tool_use for >7s without progress    → Permission (waiting for approval)
system turn_duration received               → Waiting (turn complete, green checkmark)
```

## Tool Categorization

| Animation | Tool Names |
|-----------|-----------|
| Typing | Write, Edit, Bash, Task, NotebookEdit |
| Reading | Read, Grep, Glob, WebFetch, WebSearch |

Unknown tools default to typing animation.

## Gotchas

- Partial lines at EOF: JSONL writes are not atomic. Buffer incomplete lines.
- `/clear` creates a NEW JSONL file. Old file stops growing. Detect via `notify::EventKind::Create`.
- `turn_duration` never emitted for text-only turns. Start a 5s idle timer when no tools are active and new user text arrives.
- User prompt `content` can be string or array. Check type before accessing.
