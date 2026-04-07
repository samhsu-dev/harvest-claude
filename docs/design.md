# Claude Pixel TUI — Design Overview

## Module Tree

```
src/
├── main.rs                 # Minimal: parse CLI, call lib::run()
├── lib.rs                  # Crate root: mod declarations, pub run()
├── app.rs                  # App orchestrator, main loop, action dispatch
├── action.rs               # Unified Action enum (all events)
├── event.rs                # Crossterm → Action mapping, tick/render intervals
├── tui.rs                  # TerminalGuard: setup, teardown, Drop
├── cli.rs                  # CLI argument parsing (clap derive)
├── constants.rs            # All magic numbers grouped by domain
├── types.rs                # Shared enums, type aliases, data holders
├── engine/                 # → modules/engine-design.md
├── watcher/                # → modules/watcher-design.md
├── render/                 # → modules/render-design.md
├── layout/                 # → modules/layout-design.md
└── ui/                     # → modules/ui-design.md
```

## Architecture Pattern

**Flux / Action-driven** (ratatui community standard):
```
crossterm Event → event.rs → Action → app.update(action) → app.render()
                                 ↑
                  watcher thread → mpsc::Sender<Action>
```

All state mutations flow through `Action`. Terminal events, watcher events, and timer events all produce `Action` variants dispatched to `App::update()`.

## Ownership

- `App` owns `OfficeState`, `AgentRegistry`, `DirectoryScanner`, `TimerManager`
- `OfficeState` owns `Vec<Character>`, `Vec<FurnitureInstance>`, `Vec<Seat>`, tile map
- `AgentRegistry` owns `Vec<Agent>`, `HashMap<usize, JsonlReader>`
- `TerminalGuard` owns `Terminal<CrosstermBackend<Stdout>>`, restores on `Drop`

## Roles

- **Data holders**: `Agent`, `Character`, `Seat`, `FurnitureInstance`, `OfficeLayout`
- **Orchestrator**: `App`
- **Infrastructure**: `TerminalGuard`, `EventHandler`, `DirectoryScanner`
- **Helpers**: `JsonlReader`, `PixelBuffer`, `TimerManager`

---

## main.rs / lib.rs / cli.rs

- `main.rs`: calls `claude_pixel::run()`. No other logic.
- `lib.rs`: declares all modules (see tree above), exports `pub fn run()` — install color_eyre → parse CLI → create TerminalGuard → create App → `app.run()`.
- `cli.rs`: `Args` struct via clap derive — `--watch-dir` (repeatable `Vec<PathBuf>`), `--layout` (optional `PathBuf`).

## tui.rs

Terminal lifecycle. Setup and teardown isolated from app logic. `Drop` guarantees cleanup on panic.

```rust
pub struct TerminalGuard {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub fn new() -> Result<Self>     // raw mode + alternate screen + mouse
}
impl Drop for TerminalGuard {
    fn drop(&mut self)               // disable raw mode + leave alternate screen
}
```

## action.rs

Unified event enum. All state mutations flow through `Action`.

```rust
pub enum Action {
    Tick(f64),                       // dt seconds
    Render,
    Resize(u16, u16),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Quit,

    // Watcher events (from mpsc channel)
    AgentDiscovered { path: PathBuf, project: String, session_id: String },
    AgentGone { path: PathBuf },
    AgentEvent { agent_id: usize, event: AgentEvent },

    // Timer events
    PermissionTimeout { agent_id: usize },
    TextIdleTimeout { agent_id: usize },
    ToolDoneReady { agent_id: usize, tool_id: String },
}
```

## event.rs

Bridges crossterm events to `Action`. Manages tick interval.

```rust
pub struct EventHandler {
    rx: mpsc::Receiver<Action>,
    _tx: mpsc::Sender<Action>,       // clone given to watcher thread
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self   // spawn crossterm poll thread
    pub fn next(&self) -> Result<Action>      // blocking receive
    pub fn sender(&self) -> mpsc::Sender<Action>  // for watcher thread
}
```

## app.rs

Orchestrator. Receives `Action`, updates state, renders.

```rust
pub struct App {
    office: OfficeState,
    agents: AgentRegistry,
    scanner: DirectoryScanner,
    timers: TimerManager,
    selected: Option<usize>,
    running: bool,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(args: Args) -> Result<Self>` | Load layout, init subsystems |
| `run` | `(&mut self, terminal: &mut TerminalGuard) -> Result<()>` | Event loop: recv action → update → render |
| `update` | `(&mut self, action: Action)` | Match action, mutate state |
| `render` | `(&self, frame: &mut Frame)` | Compose scene + status bar |

## constants.rs

All magic numbers as `pub const`. Grouped by domain:

| Group | Key constants |
|-------|--------------|
| Grid | `TILE_SIZE=8`, `DEFAULT_COLS=20`, `DEFAULT_ROWS=11`, `MAX_COLS/ROWS=64` |
| Animation | `WALK_FRAME_MS=150`, `TYPE_FRAME_MS=300`, `WALK_SPEED=3.0` |
| Wander | Pause `2..20s`, moves `3..6`, rest `120..240s` |
| Timers | Permission `7s`, text-idle `5s`, tool-done delay `300ms`, JSONL poll `500ms` |
| Scanner | Stale `600s`, external `120s`, dismissed cooldown `180s`, min file `3KB` |
| Visual | `PALETTE_COUNT=6`, hue shift `45..315°`, matrix `0.3s`, bubble `2s+0.5s fade` |
| Frame | `TICK_RATE_MS=16`, `MAX_DELTA_TIME=0.1` |

## types.rs

Shared enums, type aliases, serializable data holders. No constants.

```rust
pub enum TileType { Void=0, Floor1=1..Floor7=7, Wall=100 }
pub enum Direction { Down, Up, Left, Right }
pub enum CharState { Idle, Walk, Type }
pub enum AnimType { Walk, Type, Read }
pub enum AgentStatus { Active, Idle, Waiting, Permission }
pub enum BubbleKind { Permission, Waiting }

pub type Pixel = (u8, u8, u8, u8);
pub type SpriteData = Vec<Vec<Pixel>>;
pub type TilePos = (u16, u16);

struct PlacedFurniture { uid, furniture_type, col: i16, row: i16, color: Option<TileColor> }
struct TileColor { h: f32, s: f32, b: f32 }
struct OfficeLayout { version, cols, rows, tiles, furniture, tile_colors, layout_revision }
```

---

## Free Functions

| Function | Module | Signature | Description |
|----------|--------|-----------|-------------|
| `project_hash` | `watcher::scanner` | `(path: &str) -> String` | `:`, `\`, `/` → `-` |

## Validation Rules

- **OfficeLayout**: `version == 1`, `tiles.len() == cols * rows`, dims `1..=64`
- **Agent Discovery**: file > 0 bytes, mtime within 10 min, path `~/.claude/projects/<hash>/<uuid>.jsonl`
- **Terminal**: 24-bit color (`COLORTERM=truecolor`), min size `cols*8` x `rows*4+2`
