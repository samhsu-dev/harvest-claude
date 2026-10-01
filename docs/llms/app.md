# App & Rendering

> Entry order, event loop, key handling, agent panel, and half-block pixel rendering.

## Quick Start

```rust
// src/lib.rs::run
color_eyre::install()?;
tui::install_panic_hook();
let args = Args::parse();
let mut app = App::new(&args)?;   // before raw mode: errors print normally
let mut terminal = TerminalGuard::new()?;
app.run(&mut terminal)?;
```

`src/main.rs` holds only the logging guard (`logging::init`) and a call to `run`.

## Key Files

| File | Responsibility |
|------|---------------|
| `src/app.rs` | `App`: state, tick, key and mouse handling, agent focus |
| `src/event.rs` | `EventHandler`: crossterm thread, 16 ms tick, `Action` channel |
| `src/tui.rs` | `TerminalGuard`, `restore_terminal`, `install_panic_hook` |
| `src/cli.rs` | `Args` (`--layout`, `--watch-dir`) |
| `src/logging.rs` | `init(&Path) -> Result<WorkerGuard>` file logging |
| `src/ui/status_bar.rs` | `AgentPanel`, `AgentRow`, `ProduceCounts` |
| `src/ui/text.rs` | `truncate_to_width`, `fit_to_width`, `draw_centered` |
| `src/ui/input.rs` | `tile_from_cell`, `hit_test_character` |
| `src/render/*.rs` | `bubble`, `buffer`, `colorize`, `composer`, `sprites` |

## API

| Item | Signature / meaning |
|------|---------------------|
| `EventHandler` | `new`, `next`, `sender` |
| `Action` | `Tick`, `Render`, `Resize`, `Key`, `Mouse`, `Quit` |
| `PixelBuffer` | `new`, `clear`, `blit`, `blit_flipped`, `get`, `set` |
| `compose_scene` | `compose_scene(buf, &SceneInput)` draws tiles, furniture, characters z-sorted |
| `tile_from_cell` | `(col, row, offset_x, offset_y) -> TilePos` |
| `truncate_to_width` | `(text, max_width)` cuts by display columns, never inside a wide character |

## Keys

| Input | Effect |
|-------|--------|
| Left / Right | Select the previous / next non-Dormant agent |
| Enter | Focus the terminal window of the selected agent |
| Esc | Deselect |
| `q`, Ctrl+C | Quit |
| Click on a character | Select it |
| Click on the selected character | Focus its terminal window |

## Configuration

| Flag | Effect |
|------|--------|
| `--layout <path>` | Load this layout. A missing path is an error. |
| `--watch-dir <dir>` | Scan `<dir>` in addition to `~/.claude/projects`. A missing path is an error. |

- Log files: `~/.harvest-claude/logs`, daily rotation, at most 3 files.
- Minimum terminal: `MIN_TERMINAL_COLS=40`, `MIN_TERMINAL_ROWS=8`. Below it the screen shows a plain notice.
- Farm size in cells: `cols*8` columns, `rows*8/2` rows. Default grid 28x16 needs 224x64.
- Panel text widths: `NAME_MAX_WIDTH=12`, `LABEL_MAX_WIDTH=8`.

## Gotchas

- Build `App` before `TerminalGuard`. A startup error then prints on a normal screen.
- `restore_terminal` is idempotent. The panic hook and `Drop` both call it.
- Truncate by display width with `unicode-width`, never by `char` count. CJK characters take 2 columns.
- The panel lists one row per non-Dormant agent plus a produce footer. Dormant agents are skipped by arrows and clicks.
- Focus runs on a spawned thread. The UI never waits for `lsof` or `osascript`.
- `Action` keeps every variant listed in `docs/design.md`.
