# Watcher Module Design

Transcript discovery, tailing, per-agent state, and heuristic timers. Record parsing: `parser-design.md`.

## Design Overview

```
watcher/
├── mod.rs
├── scanner.rs    # DirectoryScanner: discovery, dormant, removal
├── jsonl.rs      # JsonlReader: byte-safe tailing
├── agent.rs      # Agent, ToolActivity, ProjectIdentity, Agent::apply, summary
├── registry.rs   # AgentRegistry: agents, readers, palettes
├── parser.rs     # → parser-design.md
├── timer.rs      # TimerManager, PollInterval
└── focus.rs      # terminal window focus (macOS)
```

- **Relationships**: `AgentRegistry` contains `Agent` and `JsonlReader`. `Agent` contains `ToolActivity` and `ProjectIdentity`. `registry` uses `parser`. `scanner` uses no other watcher module.
- **Visibility**: all modules `pub`; crate `harvest-claude`.
- **Roles**. Data holders: `Agent`, `ToolActivity`, `ProjectIdentity`, `ScanEvent`. Infrastructure: `DirectoryScanner`, `JsonlReader`. Helpers: `TimerManager`, `PollInterval`.

## Class / Type Specifications

### DirectoryScanner (`scanner.rs`)

Fields (private): `watcher: RecommendedWatcher`, `events_rx`, `known_files: HashMap<PathBuf, SystemTime>`, `dormant_files: HashMap<PathBuf, SystemTime>`, `watch_dirs: Vec<PathBuf>`.

```rust
pub enum ScanEvent {
    NewSession { path: PathBuf, project_name: String, session_id: String },
    SessionDormant { path: PathBuf },
    SessionGone { path: PathBuf },
}
```

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `(dirs: Vec<PathBuf>) -> Result<Self>` | Watch each existing directory recursively. A missing directory is skipped with a log line; the caller validates explicit ones. |
| `initial_scan` | `(&mut self) -> Result<Vec<PathBuf>>` | Transcripts with mtime within `DORMANT_THRESHOLD_SECS` and size ≥ `GLOBAL_MIN_FILE_SIZE`, newest first. Marks them known. |
| `poll` | `(&mut self) -> Vec<ScanEvent>` | Drain notify events. New file: emit `NewSession` when mtime within `EXTERNAL_THRESHOLD_SECS` and size > 0. Dormant file written: move to known and emit `NewSession`. Missing file: emit `SessionGone`. Known file older than `DORMANT_THRESHOLD_SECS`: `SessionDormant`. Dormant older than `REMOVED_THRESHOLD_SECS`: `SessionGone`. |
| `forget` | `(&mut self, path: &Path)` | Drop a path from known so its next write emits `NewSession` again. Used when adoption is refused at the limit. |

Function `session_info(path: &Path) -> Option<(String, String)>`: session id = file stem; project name = parent directory name (the project hash directory). Single implementation, used by scanner and `App`. Paths containing a `subagents` component are rejected by `is_jsonl_path`.

Removed: `dismiss`, `dismiss_clear`, `check_clear`, `pending_clear_files`, `project_hash`, the `sessions` path requirement.

### ToolActivity, ProjectIdentity, Agent (`agent.rs`, data holders)

```rust
pub struct ToolActivity { pub id: String, pub name: String, pub detail: String }

pub struct ProjectIdentity { pub directory: String, pub branch: String }

pub struct Agent {
    pub id: usize, pub session_id: String, pub jsonl_path: PathBuf,
    pub identity: ProjectIdentity,        // hash directory name until the first cwd
    pub status: AgentStatus,
    pub palette: u8, pub hue_shift: Option<i16>,
    pub tools: Vec<ToolActivity>,         // running, oldest first
    pub had_tools_in_turn: bool,
    pub background_tool_ids: HashSet<String>,
    pub cwd: Option<String>,
    custom_title: Option<String>, ai_title: Option<String>,
    last_text: String, last_prompt: String,
}

pub struct ApplyOutcome { pub turn_had_tools: bool, pub background_finished: bool }
```

Derives: `Debug, Clone`; `ProjectIdentity` adds `PartialEq, Eq, Default`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `(id, session_id, path, hash_dir: String, palette, hue_shift) -> Self` | Status `Idle`, identity from hash directory. |
| `apply` | `(&mut self, event: &AgentEvent) -> ApplyOutcome` | State update only, no timers or animation. Implements the `model.md` State Model rows for events (`ToolStart`, `ToolDone`, `TurnEnd`) and stores text, title, prompt, identity. Pure with respect to everything outside the Agent, so replay and live use it identically. |
| `summary` | `(&self) -> String` | Activity Summary by `model.md` precedence; each source collapsed to one line and capped at `SUMMARY_SOURCE_MAX_CHARS`. Width truncation is a UI concern. |
| `display_name` | `(&self) -> String` | `<directory> (<branch>)` or `<directory>`. |
| `is_running_non_exempt` | `(&self) -> bool` | Any running tool that starts a permission timer. |

`ToolDone` for a background launch keeps the tool and adds its id to `background_tool_ids`. `ToolDone` for an id in `background_tool_ids` removes both and sets `background_finished`. Unknown tool ids are ignored.

### AgentRegistry (`registry.rs`, orchestrator of tracked state)

Fields (private): `agents: Vec<Agent>`, `readers: HashMap<usize, JsonlReader>`, `next_id`, `palette_usage: [u8; PALETTE_COUNT]`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `add_agent` | `(&mut self, session_id: String, path: PathBuf, hash_dir: String) -> Result<usize>` | Claim least-used palette; open reader at `REPLAY_TAIL_BYTES` before end of file. Errors: file open failure. |
| `replay` | `(&mut self, id) -> Result<()>` | Read the tail, apply every event to the Agent without side effects. |
| `remove_agent` | `(&mut self, id)` | Drop reader, release palette. |
| `poll_all` | `(&mut self) -> Vec<(usize, Vec<(AgentEvent, ApplyOutcome)>)>` | New lines → events. Applies each event to its Agent and returns it with its outcome, so `App` runs side effects per event; a read error is logged and the Agent skipped. |
| `get`, `get_mut`, `agents`, `has_path` | lookup | `agents()` returns `&[Agent]`. |

Removed: sub-agent API (`add_sub_agent`, `remove_sub_agent`, `is_sub_agent`, `parent_id`, `sub_agent_map`, `active_sub_tool_names`), public `assign_palette`.

### JsonlReader (`jsonl.rs`, helper)

Fields (private): `path`, `offset: u64`, `pending: Vec<u8>`, `discarding: bool`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new_from_tail` | `(path: PathBuf, tail: u64) -> Result<Self>` | Start at `len - tail` (or 0). When offset > 0 the first partial line is discarded. |
| `new_from_start` | `(path: PathBuf) -> Result<Self>` | Offset 0. |
| `read_new_lines` | `(&mut self) -> Result<Vec<String>>` | Reopen, reset on truncation, read ≤ `JSONL_READ_CAP` bytes into `pending`, split on `\n`, decode only complete lines (UTF-8 never split). A partial line above `JSONL_MAX_LINE_BYTES` is dropped until its newline. |
| `offset` | `(&self) -> u64` | Current offset. |

Removed: `new` (seek to end) — a new Agent always replays a tail.

### TimerManager and PollInterval (`timer.rs`, helpers)

```rust
pub struct TimerManager { permission: HashMap<usize, Instant>, text_idle: HashMap<usize, Instant> }
pub enum TimerEvent { PermissionTimeout { agent_id: usize }, TextIdleTimeout { agent_id: usize } }
pub struct PollInterval { period: Duration, last: Instant }
```

| Method | Behavior |
|--------|----------|
| `start_permission(id)` / `cancel_permission(id)` | Start or restart; cancel. |
| `start_text_idle(id)` / `cancel_text_idle(id)` | Same for text-idle. |
| `cancel_all(id)` | Both timers. |
| `check_expired(&mut self) -> Vec<TimerEvent>` | Return and remove expired. |
| `PollInterval::new(ms)`, `due(&mut self, now) -> bool` | True once per period, then resets. |

Removed: `restart_permission`, `delay_tool_done`, `TimerEvent::ToolDoneReady`.

### focus (`focus.rs`, infrastructure)

**focus_agent_window(cwd: String)** — spawns a thread, never blocks the UI. Finds the terminal TTY of a process whose working directory equals `cwd` (`lsof`), then focuses the matching Terminal.app or iTerm2 tab. Every string placed in AppleScript is escaped for `\` and `"`. No-op with a log line when `cwd` is unknown or the platform is not macOS. Removed: hash-directory decoding heuristics.
