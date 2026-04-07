# Claude Pixel TUI — Concepts & Terminology

## 1. Context

**Problem Statement** — AI coding agents run as invisible terminal processes. Developers lack spatial awareness of agent activity, permission states, and tool usage. Existing solutions require a VS Code extension or browser — no native terminal option exists.

**System Role** — Claude Pixel TUI is a standalone Rust terminal application (ratatui + crossterm) that renders running Claude Code agents as animated pixel-art characters in an interactive office, using half-block Unicode characters (`▀`) for pixel-level 24-bit color rendering directly in the terminal.

**Data Flow**
- **Inputs:** JSONL transcript files (`~/.claude/projects/`), file system events (new/modified JSONL), user input (keyboard, mouse), layout file (`~/.pixel-agents/layout.json`)
- **Outputs:** Terminal pixel rendering (half-block characters, 24-bit RGB), persisted layout, persisted config (`~/.pixel-agents/config.json`)
- **Connections:** Claude Code CLI → JSONL files → File Watcher → Parser → Action channel → App update → Renderer → Terminal

**Scope Boundaries**
- **Owned:** Agent auto-detection, office rendering, character animation, layout editing, terminal pixel rendering, seat management, status display, sound notification (terminal bell)
- **Not Owned:** Claude Code CLI behavior, JSONL file format, terminal emulator implementation, agent process management (view-only — no launching or controlling agents)

## 2. Concepts

**Conceptual Diagram**
```
crossterm Event ──→ EventHandler ──→ Action ──→ App::update()
                                       ↑              │
                                       │         App::render()
Scanner (notify) ──→ mpsc::Sender<Action>         │
       │                                    PixelBuffer (RGBA)
       └── ~/.claude/projects/<hash>/              │
             <session>.jsonl                 Half-Block Renderer
                   │                               │
             JsonlReader ──→ Parser           Terminal Output
                   │
             AgentEvent ──→ TimerManager
                            (permission 7s,
                             text-idle 5s)
```

**Core Concepts**

- **Agent**
  - **Definition:** A detected Claude Code session identified by its JSONL transcript file. Agents are discovered by scanning `~/.claude/projects/` for active JSONL files.
  - **Scope:** Includes session ID, JSONL file path, file offset, active tools (`HashMap<String, String>`), status (`AgentStatus`: Active, Idle, Waiting, Permission), `had_tools_in_turn` flag. Excludes terminal process management.
  - **Relationships:** `Agent` → `Character` (visual representation), `Agent` → `JsonlReader` (data source), `Agent` → project directory (discovery scope).

- **Character**
  - **Definition:** An animated pixel-art sprite (8x16 half-block pixels) with a finite state machine (`CharState`: Idle, Walk, Type) that moves through the office grid based on agent activity.
  - **Scope:** Includes palette (`u8`, 0..6), seat assignment (`Option<usize>`), animation state, sub-tile position `(f32, f32)`, direction, wander behavior. Sprites rendered via half-block Unicode characters with 24-bit fg/bg colors.
  - **Relationships:** `OfficeState` owns `Vec<Character>`. Each `Character` references an `Agent` by ID and optionally a `Seat` by index.

- **Office**
  - **Definition:** A tile-based grid (8x8 pixel tiles via half-block rendering) containing floor, walls, furniture, and characters. The spatial container for visualization.
  - **Scope:** Includes tile map (`Vec<Vec<TileType>>`), furniture placement, walkable/blocked tile sets, z-sorted rendering. Grid default 20x11 tiles = 160x88 half-block pixels = 160 cols x 44 rows terminal cells.
  - **Relationships:** `OfficeState` owns layout, characters, furniture, seats. `OfficeLayout` persisted to file.

- **Layout**
  - **Definition:** A serializable office configuration — grid dimensions, tile types, furniture positions, tile colors. Persisted to `~/.pixel-agents/layout.json`. Compatible with the VS Code extension's format.
  - **Scope:** Includes tiles (`Vec<u8>`), furniture (`Vec<PlacedFurniture>`), tile colors (`Option<HashMap<String, TileColor>>`), layout revision. Excludes runtime state (characters, animation).
  - **Relationships:** `OfficeLayout` deserialized from file → `OfficeState::from_layout()` consumes it.

- **Half-Block Pixel**
  - **Definition:** The rendering primitive. Each terminal character cell displays two vertical pixels using the Unicode upper half-block character (`▀`) with foreground color (top pixel) and background color (bottom pixel). Achieves 24-bit per-pixel color in any true-color terminal.
  - **Scope:** Rendering only. One character cell = 1 column x 2 rows of pixels. Terminal must support 24-bit color (iTerm2, Kitty, WezTerm, Alacritty, Windows Terminal).
  - **Relationships:** `PixelBuffer` → half-block conversion → ratatui `Buffer` cells.

- **Scanner**
  - **Definition:** Background thread (`notify::RecommendedWatcher`) that discovers active Claude Code sessions by monitoring `~/.claude/projects/` for JSONL files. Sends events via `mpsc::channel` to the main thread.
  - **Scope:** Watches specified directories + global `~/.claude/projects/*/`. Filters by file age (`SystemTime`) and size. Creates/removes agents on file appearance/staleness.
  - **Relationships:** `DirectoryScanner` → `notify::Watcher` (file events), `DirectoryScanner` → `AgentRegistry` (creates agents).

- **Tool Activity**
  - **Definition:** A tracked tool invocation parsed from JSONL transcripts. Drives character animation state (Idle → Walk → Type transition, typing vs reading animation).
  - **Scope:** Includes tool ID, tool name, start/done detection, permission detection. Tool categorization: typing (Write/Edit/Bash/Task) vs reading (Read/Grep/Glob/WebFetch/WebSearch).
  - **Relationships:** `Agent` tracks `active_tools`. Tool events trigger `Character` state transitions.

- **Timer Manager**
  - **Definition:** Heuristic timers that infer agent states when definitive signals are delayed. Two timers: permission (7s after non-exempt tool with no result) and text-idle (5s after text-only assistant response with no tools).
  - **Scope:** Per-agent timer state. Permission timer starts on `tool_use` of non-exempt tool, cancelled by `tool_result` or `turn_duration`. Text-idle timer starts on text-only assistant message, cancelled by any subsequent record.
  - **Relationships:** Parser functions emit `AgentEvent` → `App::update()` starts/cancels timers. Timer expiry → `Action::PermissionTimeout` or `Action::TextIdleTimeout`.

- **Palette & Colorization**
  - **Definition:** Character visual identity. 6 base palettes (skin/hair/shirt color sets). First 6 agents get unique palettes. Beyond 6: least-used palette with random hue shift (45-315 degrees).
  - **Scope:** Palette index (`u8`, 0..6) + optional hue shift (`Option<i16>`). Applied at sprite generation time via HSL rotation.
  - **Relationships:** `AgentRegistry` assigns palette on agent creation. `Character` stores palette + hue shift. Sprite functions accept palette as parameter.

- **Sub-Agent**
  - **Definition:** A background agent launched by a parent agent (via Task/Agent tool). Tracked via `progress` JSONL records with `parentToolUseID`. Spawns a separate character at the nearest free seat to the parent.
  - **Scope:** Negative agent IDs (convention). Tool activity tracked independently. Lifecycle tied to parent's background tool completion. Permission bubbles propagate to parent.
  - **Relationships:** Parent `Agent` → sub-agent mapping via `sub_agents: HashMap<String, usize>`. Sub-agent `Character` spawns near parent's seat.

- **Speech Bubble**
  - **Definition:** Visual overlay above a character indicating permission-wait (amber "...") or completion-wait (green checkmark). Permission bubbles persist until dismissed. Waiting bubbles auto-fade after 2 seconds.
  - **Scope:** Rendered as small sprite above character. Click to dismiss permission bubble. Waiting bubble has countdown timer.
  - **Relationships:** `Character` holds `Option<BubbleState>`. Timer expiry or user click clears bubble.

- **Matrix Effect**
  - **Definition:** Visual spawn/despawn animation. Green cascading "rain" columns that reveal (spawn) or consume (despawn) a character over 0.3 seconds.
  - **Scope:** 16 vertical pixel columns per character tile, staggered timing per column (0-30% offset). Per-pixel rendering with alpha decay along trail.
  - **Relationships:** Triggered on `add_character` (spawn) or `remove_character` (despawn). Blocks character rendering during effect.

## 3. Contracts & Flow

**Data Contracts**

- **With Claude Code CLI:** JSONL records in `~/.claude/projects/<hash>/<session>.jsonl`. Record types: `assistant` (tool_use, text), `user` (tool_result), `system` (turn_duration), `progress` (sub-agent activity, bash/mcp progress). Project hash = workspace path with `:`, `\`, `/` → `-`.
- **With File System:** Layout at `~/.pixel-agents/layout.json` (version 1, atomic writes). Config at `~/.pixel-agents/config.json`. Watched directories configurable via CLI args.
- **With Terminal:** crossterm raw mode + alternate screen + mouse capture. Output: ANSI escape sequences with 24-bit color. Input: key events, mouse events (cell-level coordinates).

**Internal Processing Flow**

1. **Startup** — Parse CLI args → load layout → `OfficeState::from_layout()` → Scanner initial scan → `Action::AgentDiscovered` per active file → characters spawned with matrix effect.
2. **Event loop** — `EventHandler::next()` blocks → returns `Action` → `App::update(action)` mutates state → `App::render()` draws frame. All state changes flow through `Action` variants.
3. **File watching** — `notify` detects JSONL modifications → `AgentRegistry::poll_all()` → `JsonlReader` reads new lines → parser extracts `AgentEvent` → dispatched as `Action::AgentEvent` → timer manager adjusted.
4. **Tool detection** — `AgentEvent::ToolStart` with non-exempt tool → permission timer starts (7s) → character walks to seat → Type animation. `AgentEvent::ToolDone` → cancel permission timer → 300ms delay before UI update.
5. **Heuristic inference** — `AgentEvent::TextOnly` + `had_tools_in_turn == false` → text-idle timer (5s). Timer expiry → `Action::TextIdleTimeout`. Permission timer expiry → `Action::PermissionTimeout` + speech bubble.
6. **Turn completion** — `AgentEvent::TurnEnd` → foreground tools cleared → character returns to Idle → wander behavior resumes.
7. **Rendering** — `Action::Render` triggers frame → update character FSMs → update effects → z-sort entities → composite to `PixelBuffer` → half-block characters → ratatui diff-render.

## 4. Scenarios

- **Typical:** Developer runs `claude-pixel`. Scanner finds 3 active Claude sessions. Three characters spawn with matrix effects, walk to assigned seats, begin typing animations. One finishes a turn — character stands, wanders. Developer presses `q` to quit.

- **Sub-agent:** Agent launches a background Task tool. `progress` record detected → sub-agent character spawns at nearest free seat to parent with matrix effect. Sub-agent finishes → despawn effect. Parent's background tool clears.

- **Permission wait:** Agent invokes a non-exempt tool (Bash). 7 seconds pass with no tool_result. Permission timer fires → character shows amber "..." bubble. Developer clicks character → bubble dismissed. (When hooks are available, permission detection is instant.)

- **Session discovery:** New Claude session starts after app launch. Scanner detects new JSONL file via `notify` watcher → new agent created → palette assigned (least-used) → character spawns with matrix effect. JSONL file goes stale (no writes for 10 minutes) → agent removed → character despawns.

- **Interaction:** Developer clicks a character (mouse cell → tile mapping). Character highlighted with outline. Status bar shows agent details (session ID, current tool, project path). Arrow keys or click another character to change selection. Press Esc to deselect.

- **Partial JSONL:** JSONL file written mid-line (partial JSON). `JsonlReader` buffers incomplete line → completes on next 500ms poll. Read capped at 64KB per poll to prevent blocking.
