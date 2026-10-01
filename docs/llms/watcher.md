# Agent Detection

> Directory scanning, transcript tailing, per-agent state, timers, and window focus.

## Quick Start

```rust
let mut scanner = DirectoryScanner::new(dirs)?;   // Vec<PathBuf>
for path in scanner.initial_scan() {
    let id = registry.add_agent(/* session, path, project */)?;
    registry.replay(id)?;                          // apply transcript tail
}
for (id, applied) in registry.poll_all() { /* update characters */ }
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/watcher/scanner.rs` | `DirectoryScanner`, `session_info(path)`, `notify` events |
| `src/watcher/registry.rs` | `AgentRegistry`: agents, readers, palette claims |
| `src/watcher/agent.rs` | `Agent`: state, summary, identity |
| `src/watcher/jsonl.rs` | `JsonlReader` tailing |
| `src/watcher/parser.rs` | Record parsing and event extraction |
| `src/watcher/timer.rs` | `TimerManager`, `PollInterval` |
| `src/watcher/focus.rs` | `focus_agent_window(cwd: String)` |

## API

| Type | Methods |
|------|---------|
| `DirectoryScanner` | `new(Vec<PathBuf>)`, `initial_scan`, `poll`, `forget` |
| `AgentRegistry` | `new`, `add_agent`, `replay`, `remove_agent`, `poll_all`, `get`, `get_mut`, `agents`, `has_path` |
| `Agent` | `new`, `apply -> ApplyOutcome`, `summary`, `display_name`, `is_running_non_exempt` |
| `TimerManager` | `start_permission`, `cancel_permission`, `start_text_idle`, `cancel_text_idle`, `cancel_all`, `check_expired` |
| `PollInterval` | `new`, `due` |

## Configuration

| Constant | Value |
|----------|-------|
| `PERMISSION_TIMER_MS` | 7000 |
| text-idle timer | 5000 ms |
| `JSONL_POLL_MS` | 500 |
| `REPLAY_TAIL_BYTES` | 65536 |
| Dormant after silence | 600 s |
| Removed after silence | 7200 s |
| `MAX_AGENTS` | 8 |

## Gotchas

- Discovery root is `~/.claude/projects`. `--watch-dir` adds directories.
- `session_info` accepts `<hash>/<uuid>.jsonl`, with or without a `sessions` directory. Paths under `subagents/` are rejected.
- The scanner drains `notify` events every tick. Readers poll on `PollInterval`.
- A refused adoption at the agent limit calls `forget`. The next write retries it.
- A write to a Dormant agent wakes it, regardless of the limit.
- Replay applies the transcript tail. An adopted agent that runs a tool shows Active and its summary, with a permission timer but no bubble.
- The agent display name is `<dir> (<branch>)` from the `Identity` event.
- Summary precedence: running tool `<name>: <detail>`, assistant text, title, last prompt.
- Window focus uses `Identity.cwd`: `lsof -d cwd -Fpn` finds the pid, then escaped AppleScript raises the Terminal or iTerm tab.
- No dismiss tracking and no `/clear` tracking exist.
