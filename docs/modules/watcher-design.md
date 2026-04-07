# Watcher Module Design

Agent detection pipeline: directory scanning, JSONL tailing, record parsing, heuristic timers.

```
watcher/
├── mod.rs
├── scanner.rs     # Directory scanning, session discovery, /clear detection
├── registry.rs    # AgentRegistry, palette assignment, sub-agents
├── jsonl.rs       # JSONL file tailing, partial line buffer
├── parser.rs      # Record parsing, tool extraction, event emission
└── timer.rs       # Heuristic timers (permission, text-idle)
```

---

## scanner.rs

```rust
pub struct DirectoryScanner {
    watch_dirs: Vec<PathBuf>,
    watcher: RecommendedWatcher,
    events_rx: Receiver<notify::Result<Event>>,
    known_files: HashMap<PathBuf, SystemTime>,
    dismissed_files: HashMap<PathBuf, Instant>,  // path → dismissed_at (3-min cooldown)
    clear_dismissed: HashSet<PathBuf>,            // permanently dismissed /clear files
    active_threshold: Duration,                   // 10 min for stale detection
    external_threshold: Duration,                 // 2 min for external adoption
    min_file_size: u64,                           // 3KB for global scan
    pending_clear_files: HashSet<PathBuf>,        // two-tick adoption: first scan skips
}

pub enum ScanEvent {
    NewSession { path: PathBuf, project_name: String, session_id: String },
    SessionGone { path: PathBuf },
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(dirs: Vec<PathBuf>) -> Result<Self>` | Set up notify watcher on dirs |
| `initial_scan` | `(&mut self) -> Result<Vec<PathBuf>>` | One-time scan for active JSONL |
| `poll` | `(&mut self) -> Vec<ScanEvent>` | Drain events, check staleness, two-tick adopt |
| `dismiss` | `(&mut self, path: &Path)` | Cooldown dismiss (3 min) |
| `dismiss_clear` | `(&mut self, path: &Path)` | Permanent dismiss (/clear file) |
| `check_clear` | `(&self, path: &Path) -> Result<bool>` | Scan first 8KB for `/clear</command-name>` |

### /clear Detection

Conditions: agent idle > 2s AND terminal focused AND has processed lines.
Scanner checks new JSONL files in project dir for `/clear</command-name>` in first 8KB.
Two-tick external adoption: first scan → `pending_clear_files`, second scan → adopt if unclaimed.

### External Session Thresholds

- External agents: mtime within `external_threshold` (2 min) + file size > 0
- Global scan (Watch All Sessions): mtime within 10 min + file size > 3KB
- Dismissed file cooldown: 3 minutes before re-adoption

---

## registry.rs

```rust
pub struct AgentRegistry {
    agents: Vec<Agent>,
    readers: HashMap<usize, JsonlReader>,
    next_id: usize,
    palette_usage: [u8; PALETTE_COUNT as usize],
    sub_agent_map: HashMap<String, usize>,   // "parentId:parentToolId" → sub_agent_id
}

pub struct Agent {
    pub id: usize,
    pub session_id: String,
    pub jsonl_path: PathBuf,
    pub project_name: String,
    pub status: AgentStatus,
    pub active_tools: HashMap<String, String>,      // tool_id → tool_name
    pub had_tools_in_turn: bool,
    pub parent_id: Option<usize>,
    pub background_tool_ids: HashSet<String>,        // Task/Agent tools surviving turn_duration
    pub active_sub_tool_names: HashMap<String, String>, // sub-agent tool tracking on parent
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `add_agent` | `(&mut self, session_id, path, project) -> usize` | Create agent + reader, assign palette |
| `remove_agent` | `(&mut self, id)` | Drop reader, free palette, clean sub-agents |
| `poll_all` | `(&mut self) -> Vec<(usize, Vec<AgentEvent>)>` | Read new JSONL lines, parse events |
| `get` / `get_mut` | `(&self/&mut self, id) -> Option<&/&mut Agent>` | Lookup |
| `agents` | `(&self) -> &[Agent]` | All agents |
| `assign_palette` | `(&mut self) -> (u8, Option<i16>)` | Least-used palette + optional hue shift |
| `add_sub_agent` | `(&mut self, parent_tool_id, parent_id, path) -> usize` | Negative ID sub-agent |
| `remove_sub_agent` | `(&mut self, parent_tool_id: &str)` | Remove by parent tool ID |

### Background Agent Detection

Parse `tool_result` for "Async agent launched successfully." text → add tool_id to `background_tool_ids`.
Background tools survive `turn_duration` — not cleared with foreground tools.

---

## jsonl.rs

```rust
pub struct JsonlReader {
    path: PathBuf,
    offset: u64,
    line_buffer: String,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(path: PathBuf) -> Result<Self>` | Open file, seek to end |
| `new_from_start` | `(path: PathBuf) -> Result<Self>` | Open file, read from beginning |
| `read_new_lines` | `(&mut self) -> Result<Vec<String>>` | Read up to 64KB, split lines, buffer incomplete |
| `offset` | `(&self) -> u64` | Current file offset |

Partial line handling: lines without `\n` kept in `line_buffer`, prepended to next read.

---

## parser.rs

Stateless functions. Parse JSONL records into structured events.

```rust
pub struct JsonlRecord { pub record_type: RecordType, pub value: Value }
pub enum RecordType { Assistant, User, System, Progress, Unknown }

pub enum AgentEvent {
    ToolStart { tool_id: String, tool_name: String },
    ToolDone { tool_id: String },
    TurnEnd,
    TextOnly,                           // text-only assistant, no tool_use
    SubAgentToolStart { parent_tool_id: String, tool_id: String, tool_name: String },
    SubAgentToolDone { parent_tool_id: String, tool_id: String },
    SubAgentSpawn { parent_tool_id: String },
    BashProgress { tool_id: String },   // restarts permission timer
    BackgroundAgentDetected { tool_id: String },
}
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `parse_line` | `(line: &str) -> Option<JsonlRecord>` | Parse via serde_json::Value |
| `extract_events` | `(record: &JsonlRecord, agent: &Agent) -> Vec<AgentEvent>` | All events from one record |
| `extract_tool_use` | `(value: &Value) -> Vec<ToolUse>` | tool_use blocks from assistant content |
| `extract_tool_result` | `(value: &Value) -> Vec<String>` | Completed tool IDs from user content |
| `is_turn_end` | `(value: &Value) -> bool` | system subtype == turn_duration |
| `is_non_exempt_tool` | `(name: &str) -> bool` | Exempt: Task, Agent, AskUserQuestion |

### Event Processing Rules

- **ToolStart** with non-exempt tool → start permission timer (7s)
- **ToolDone** → cancel permission timer, 300ms delay before UI update (flicker prevention)
- **TextOnly** (assistant with no tool_use, `had_tools_in_turn == false`) → start text-idle timer (5s)
- **TurnEnd** → clear foreground tools (preserve `background_tool_ids`), reset `had_tools_in_turn`
- **BashProgress / McpProgress** → restart permission timer (tool executing, not stuck)
- **Any new JSONL data** → cancel text-idle timer (before line processing)

---

## timer.rs

Per-agent heuristic timers. Checked each frame via `Instant` comparison.

```rust
pub struct TimerManager {
    permission_timers: HashMap<usize, Instant>,
    text_idle_timers: HashMap<usize, Instant>,
    tool_done_delays: HashMap<(usize, String), Instant>,  // (agent_id, tool_id) → delay
}

pub enum TimerEvent {
    PermissionTimeout { agent_id: usize },
    TextIdleTimeout { agent_id: usize },
    ToolDoneReady { agent_id: usize, tool_id: String },
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `() -> Self` | Empty timer set |
| `start_permission` | `(&mut self, agent_id)` | Start/restart 7s timer |
| `restart_permission` | `(&mut self, agent_id)` | Restart if exists (for bash/mcp progress) |
| `cancel_permission` | `(&mut self, agent_id)` | Cancel permission timer |
| `start_text_idle` | `(&mut self, agent_id)` | Start/restart 5s timer |
| `cancel_text_idle` | `(&mut self, agent_id)` | Cancel text-idle timer |
| `cancel_all` | `(&mut self, agent_id)` | Cancel both timers |
| `delay_tool_done` | `(&mut self, agent_id, tool_id)` | Start 300ms done delay |
| `check_expired` | `(&mut self) -> Vec<TimerEvent>` | Return all expired, remove them |
