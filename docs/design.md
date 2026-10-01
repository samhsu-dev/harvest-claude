# Harvest Claude — Design

Software structure of the crate. Domain rules: `model.md`. Module details: `modules/*-design.md`.

## Design Overview

Crate `harvest-claude`: one package, library plus thin binary.

```
src/
├── main.rs        # installs tracing, calls lib::run()
├── lib.rs         # mod declarations + pub fn run()
├── logging.rs     # tracing file subscriber with bounded rotation
├── app.rs         # App orchestrator, event loop, side effects
├── action.rs      # Action enum
├── event.rs       # crossterm thread → Action channel
├── tui.rs         # TerminalGuard, panic hook
├── cli.rs         # Args (clap)
├── constants.rs   # tuning values
├── types.rs       # shared enums and data holders
├── engine/        # → modules/engine-design.md
├── watcher/       # → modules/watcher-design.md, modules/parser-design.md
├── render/        # → modules/render-design.md
├── layout/        # → modules/layout-design.md
└── ui/            # → modules/ui-design.md
```

- **Classes**: `App`, `Action`, `EventHandler`, `TerminalGuard`, `Args`, `AgentEvent`, `OfficeLayout`.
- **Relationships**: `App` contains `OfficeState`, `AgentRegistry`, `DirectoryScanner`, `TimerManager`, `Warehouse`. `AgentRegistry` contains `Agent` and `JsonlReader`. `OfficeState` contains `Character`, `Seat`, `FurnitureInstance`. `TerminalGuard` contains the ratatui terminal.
- **Dependency direction**: `app` → {`watcher`, `engine`, `render`, `layout`, `ui`} → `types`, `constants`. `watcher` never imports `engine`, `render`, or `ui`. `engine` never imports `watcher` or `ui`. `render` imports `types` only.
- **Errors**: `color_eyre::Report` with `wrap_err` context. No custom error enum; see Error Handling.
- **Roles**. Data holders: `Agent`, `Character`, `Seat`, `FurnitureInstance`, `OfficeLayout`, `Warehouse`. Orchestrator: `App`. Infrastructure: `TerminalGuard`, `EventHandler`, `DirectoryScanner`. Helpers: `JsonlReader`, `PixelBuffer`, `TimerManager`.

Flow, one thread per source, one consumer:

```
crossterm thread ──► EventHandler ──► Action ──► App::update ──► App::render
                       tick (16 ms) ──┘            ├─ every JSONL_POLL_MS: readers → Agent::apply → effects
                                                   └─ every SCAN_INTERVAL_MS: scanner → adopt / dormant / remove
```

Visibility: every module is `pub` and re-exported from `lib.rs`; integration tests in `tests/` import them by path.

## Class / Type Specifications

### App (`app.rs`, orchestrator)

Fields (private): `office: OfficeState`, `agents: AgentRegistry`, `scanner: DirectoryScanner`, `timers: TimerManager`, `warehouse: Warehouse`, `config_dir: Option<PathBuf>`, `selected: Option<usize>`, `running: bool`, `render_origin: (u16, u16)`, `poll: PollInterval`, `scan: PollInterval`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `(args: Args) -> Result<Self>` | Load layout; build scanner; run initial scan; adopt agents with replay. Errors: unreadable explicit `--watch-dir`. |
| `run` | `(&mut self, guard: &mut TerminalGuard, events: &EventHandler) -> Result<()>` | Receive `Action`, `update`, redraw on `Render`. |
| `update` | `(&mut self, action: Action)` | Dispatch; `Tick` drives polling cadence, timers, engine update. |
| `render` | `(&self, guard: &mut TerminalGuard) -> Result<()>` | Compose scene, labels, status panel; too-small notice. |

Private responsibilities, each one method: adopt session (capacity check, palette, character, replay), wake, mark dormant, remove (palette released by the registry), apply live `AgentEvent` side effects, barn delivery, deliver deposit, select next/previous visible agent, click, focus.

Selection and hit-testing consider only characters whose Agent is not Dormant.

### Action (`action.rs`, data holder)

`#[non_exhaustive] enum Action { Tick(f64), Render, Resize(u16, u16), Key(KeyEvent), Mouse(MouseEvent), Quit }`. Derives `Debug`, `Clone`. Watcher and timer outputs are consumed inside `Tick` handling and have no variants.

### EventHandler (`event.rs`, infrastructure)

Owns the `mpsc` channel and the crossterm poll thread. Sends `Key` (press only), `Mouse`, `Resize`, then `Tick(dt)` and `Render` every `TICK_RATE_MS`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `() -> Self` | Spawn thread. |
| `next` | `(&self) -> Result<Action>` | Blocking receive; error when the thread ended. |
| `sender` | `(&self) -> Sender<Action>` | Clone for tests. |

### TerminalGuard (`tui.rs`, infrastructure)

Owns `Terminal<CrosstermBackend<Stdout>>`. `new` enables raw mode, alternate screen, mouse capture. `Drop` calls `restore_terminal()`.

### Args (`cli.rs`, data holder)

`--watch-dir <PATH>` repeatable: directories scanned in addition to `~/.claude/projects`. `--layout <PATH>` optional layout file. Doc comments name the program "Harvest Claude".

### AgentEvent (`types.rs`, data holder)

Parser output, one variant per model Event (`model.md` contracts). `#[derive(Debug, Clone, PartialEq, Eq)]`.

```rust
pub enum AgentEvent {
    ToolStart { tool_id: String, tool_name: String, detail: String },
    ToolDone { tool_id: String, is_error: bool, background_launch: bool },
    TurnEnd,
    Text { first_line: String },
    Title { title: String, custom: bool },
    Prompt { text: String },
    Identity { cwd: String, git_branch: String },
}
```

### Shared enums and aliases (`types.rs`)

| Type | Variants / shape |
|------|------------------|
| `TileType` | `Void=0, Grass, GrassDark, Dirt, DirtDark, Water, Sand, Stone, Fence=100` |
| `Direction` | `Down, Up, Left, Right` |
| `CharState` | `Idle, Walk, Type` |
| `AnimType` | `Walk, Type, Read, Fish, Farm, Harvest` |
| `AgentStatus` | `Active, Idle, Waiting, Permission, Dormant` |
| `ProduceType` | `Wheat, Fruit, Fish` |
| `BubbleKind` | `Permission, Waiting` |
| `CompanionKind` | `Chicken, Cat, Dog` |
| `Pixel, SpriteData, TilePos` | `(u8,u8,u8,u8)`, `Vec<Vec<Pixel>>`, `(u16,u16)` |
| `PlacedFurniture`, `TileColor`, `OfficeLayout` | serde data holders; layout fields in `modules/layout-design.md` |

### Value tiers (`constants.rs`)

All values are compile-time constants (tier 1). Grouped by concern.

| Group | Constants |
|-------|-----------|
| Grid | `TILE_SIZE=8`, `DEFAULT_COLS=28`, `DEFAULT_ROWS=16`, `MAX_COLS=MAX_ROWS=64`, `MIN_TERMINAL_COLS=40`, `MIN_TERMINAL_ROWS=8` |
| Animation | `WALK_FRAME_MS`, `TYPE_FRAME_MS`, `WALK_SPEED`, `SITTING_OFFSET_PX`, furniture and bubble durations |
| Wander | pause, moves, rest ranges |
| Polling | `JSONL_POLL_MS=500`, `SCAN_INTERVAL_MS=3000`, `JSONL_READ_CAP=64 KiB`, `JSONL_MAX_LINE_BYTES=8 MiB`, `REPLAY_TAIL_BYTES=64 KiB` |
| Heuristics | `PERMISSION_TIMER_MS=7000`, `TEXT_IDLE_TIMER_MS=5000` |
| Lifecycle | `EXTERNAL_THRESHOLD_SECS=120`, `DORMANT_THRESHOLD_SECS=600`, `REMOVED_THRESHOLD_SECS=7200`, `GLOBAL_MIN_FILE_SIZE=3072`, `MAX_AGENTS=8` |
| Text | `SUMMARY_SOURCE_MAX_CHARS=200`, `LABEL_MAX_WIDTH=8`, `NAME_MAX_WIDTH=12` |
| Visual | `PALETTE_COUNT=6`, hue shift range, `WAITING_BUBBLE_SECS`, `BUBBLE_FADE_SECS` |
| Frame | `TICK_RATE_MS=16`, `MAX_DELTA_TIME=0.1` |

Removed: `MATRIX_DURATION_SECS`, `TOOL_DONE_DELAY_MS`, `DISMISSED_COOLDOWN_SECS`, `CLEAR_IDLE_THRESHOLD_MS`, `STALE_THRESHOLD_SECS` (duplicates `DORMANT_THRESHOLD_SECS`).

## Function Specifications

**run() -> Result<()>** (`lib.rs`) — Responsibility: process entry. Behavior: install `color_eyre`; chain `tui::install_panic_hook`; parse `Args`; build `App`; create `TerminalGuard`; run loop. `App::new` runs before the terminal enters raw mode so startup errors print on a normal screen. Errors: any startup or render failure, with context.

**install_panic_hook()** (`tui.rs`) — Behavior: wraps the current panic hook so `restore_terminal()` runs before the report prints. Called once after `color_eyre::install`.

**restore_terminal() -> Result<()>** (`tui.rs`) — Behavior: disable raw mode, leave alternate screen, disable mouse capture. Idempotent. Called by `Drop` and the panic hook.

**logging::init(dir: &Path) -> Result<WorkerGuard>** (`logging.rs`) — Behavior: non-blocking daily-rolling file appender writing `<dir>/logs/harvest-claude.<date>.log`, at most 3 files kept, filter from `RUST_LOG` defaulting to `info`. `main` holds the guard for the process lifetime. Failure to create the directory returns an error that `main` reports and continues without logging.

## Error Handling

No custom error enum: the crate is an application, failures end in a message or a log line.

| Condition | Handling |
|-----------|----------|
| Explicit `--watch-dir` missing or not a directory | `App::new` returns error naming the path; process exits before raw mode |
| Default projects directory missing | `info` log; scanner idle |
| Layout file unreadable, invalid, or outdated | `warn` log; bundled default in memory; file preserved (`layout-design.md`) |
| Warehouse file corrupt | `warn` log; file renamed `warehouse.json.corrupt`; empty warehouse |
| Warehouse save fails | `warn` log; retry on next deposit |
| Transcript read error on one Agent | `warn` log once per Agent; other Agents continue |
| Malformed or oversized JSONL line | skipped; `debug` log |
| Terminal below `MIN_TERMINAL_*` | plain notice, no scene |
