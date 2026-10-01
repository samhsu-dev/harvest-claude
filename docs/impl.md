# Harvest Claude — Implementation

Rust crate research and API findings for the terminal farm visualizer of Claude Code agents.

---

## Dependencies

```toml
[dependencies]
ratatui = "0.30"
crossterm = "0.29"
notify = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"
clap = { version = "4", features = ["derive"] }
rand = "0.9"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
unicode-width = "0.2"

[dev-dependencies]
tempfile = "3"
rstest = "0.27"
```

---

## Crate Findings

### ratatui ^0.30 — TUI framework

**[ratatui]** `use ratatui::prelude::*` — re-exports Frame, Rect, Buffer, Style, Color, Widget, StatefulWidget.

**[ratatui]** `Color::Rgb(u8, u8, u8)` — 24-bit true color. Only works in terminals with `COLORTERM=truecolor`. macOS Terminal.app does NOT support it; iTerm2, Kitty, WezTerm, Alacritty do.

**[ratatui]** `impl Widget for &T { fn render(self, area: Rect, buf: &mut Buffer) }` — Custom widget trait. Direct Buffer manipulation is faster than Canvas widget for pixel rendering.

**[ratatui]** `buf.cell_mut((x, y)) -> Option<&mut Cell>` — Direct cell access. `cell.set_symbol("▀")` + `cell.set_fg(top_color)` + `cell.set_bg(bottom_color)` for half-block pixel pair.

**[ratatui]** `terminal.draw(|frame| { ... })` — Diff-based rendering. Only changed cells written to terminal. Critical for performance — full redraw at 160x44 cells is 7,040 cells, but diff means only changed cells emit escape sequences.

**[ratatui]** `frame.area()` — Returns `Rect` for available terminal space. Use to detect resize and recompute PixelBuffer dimensions.

### crossterm ^0.29 — Terminal backend

**[crossterm]** `crossterm::event::poll(Duration) -> Result<bool>` — Non-blocking event check. Use `poll(Duration::from_millis(16))` for ~60fps game loop.

**[crossterm]** `crossterm::event::read() -> Result<Event>` — Blocking read. Call after `poll()` returns true.

**[crossterm]** `Event::Mouse(MouseEvent { kind, column, row, modifiers })` — Mouse coordinates are cell-level (u16). For half-block pixels: `pixel_x = column`, `pixel_y = row * 2`. Cannot distinguish upper vs lower half of cell on click.

**[crossterm]** `Event::Key(KeyEvent { code: KeyCode, modifiers, kind, state })` — `KeyCode::Char('q')`, `KeyCode::Esc`, `KeyCode::Enter`. `kind: KeyEventKind::Press` filters out release events.

**[crossterm]** `execute!(stdout, EnterAlternateScreen, EnableMouseCapture)` — Terminal setup. Reverse on exit: `LeaveAlternateScreen, DisableMouseCapture`. Always disable raw mode in cleanup (use guard pattern or `Drop`).

**[crossterm]** `Event::Resize(cols, rows)` — Terminal resize event. Must recompute PixelBuffer dimensions and re-render.

### notify ^8 — File system watcher

**[notify]** `notify::recommended_watcher(tx) -> Result<impl Watcher>` — Creates platform-optimal watcher (FSEvents on macOS, inotify on Linux). Sends events to `mpsc::Sender`.

**[notify]** `watcher.watch(path, RecursiveMode::Recursive)` — Watch directory recursively. Fires `EventKind::Create`, `EventKind::Modify`, `EventKind::Remove`.

**[notify]** macOS uses FSEvents by default (feature `macos_fsevent`). Reliable for file creation/modification. `PollWatcher` available as fallback for network filesystems.

**[notify]** No built-in debouncing. Use manual dedup: track seen paths + mtimes in `known_files: HashMap<PathBuf, SystemTime>`.

**[notify]** `watcher` must be stored (not dropped). Dropping the watcher stops watching. Keep in `DirectoryScanner` struct.

### serde + serde_json ^1 — JSON parsing

**[serde_json]** `serde_json::from_str::<Value>(line)` — Parse one JSONL line. Use `Value` for flexible field access (JSONL record schema varies by type).

**[serde_json]** `value["type"].as_str()` — Dispatch on record type (`"assistant"`, `"user"`, `"system"`, `"ai-title"`, `"custom-title"`, `"last-prompt"`). No `"progress"` records exist. Returns `Option<&str>`.

**[serde_json]** `value.pointer("/message/content")` — JSON Pointer for nested access. Useful for extracting tool_use blocks from deeply nested structures.

**[serde_json]** Partial lines at EOF produce errors. Buffer raw bytes (`Vec<u8>`), split on `b'\n'`, and decode only complete lines with `String::from_utf8_lossy`; a read that ends inside a multibyte character must not be decoded.

**[serde]** `#[serde(skip_serializing_if = "Option::is_none")]` — Omit optional fields in layout JSON. Keeps output compatible with VS Code extension.

### dirs ^6 — Home directory

**[dirs]** `dirs::home_dir() -> Option<PathBuf>` — Returns `~` on macOS/Linux. For `~/.claude/`: `dirs::home_dir().unwrap().join(".claude")`.

### clap ^4 — CLI arguments

**[clap]** `#[derive(Parser)] struct Args { ... }` — Derive macro generates CLI parser. `Args::parse()` at startup.

**[clap]** `#[command(version, about)]` — Auto-generates `--version` and `--help` from Cargo.toml metadata.

**[clap]** `#[arg(long)] watch_dir: Vec<PathBuf>` — Repeatable flag: `--watch-dir /path1 --watch-dir /path2`.

### rand ^0.9 — Random numbers

**[rand]** `rand::rng()` — Thread-local RNG (renamed from `thread_rng()` in 0.8).

**[rand]** `rng.random_range(min..max)` — Uniform range (renamed from `gen_range()` in 0.8). Use for wander pause duration, direction selection, palette assignment.

### color-eyre ^0.6 — Error handling

**[color-eyre]** `color_eyre::install()?` — Call once at startup. Installs colored backtrace + panic hook. The panic hook must restore terminal state before printing backtrace.

**[color-eyre]** `.wrap_err("context")` — Add context to errors (replaces anyhow's `.context()`). Use at module boundaries.

**[color-eyre]** Custom panic hook pattern for TUI: install `color_eyre`, then override panic hook to call `disable_raw_mode()` + `LeaveAlternateScreen` before printing backtrace. Prevents terminal corruption on panic.

### tracing ^0.1 + tracing-subscriber ^0.3 — Logging

**[tracing]** `info!()`, `debug!()`, `warn!()`, `error!()` — Structured logging macros. In TUI app, log to file (not stdout): `tracing_subscriber::fmt().with_writer(file).init()`.

**[tracing]** Log directory: `~/.harvest-claude/logs/`. Created on startup.

Transcript record shapes: `impl-transcript.md`.

### unicode-width ^0.2 — Display width

**[unicode-width]** `use unicode_width::{UnicodeWidthChar, UnicodeWidthStr}`; `str.width() -> usize`, `char.width() -> Option<usize>` (`None` for control characters). CJK characters are width 2, so byte or char counts misplace truncation.

### tracing-appender ^0.2 — File logging

**[tracing-appender]** `RollingFileAppender::builder().rotation(Rotation::DAILY).filename_prefix("harvest-claude").filename_suffix("log").max_log_files(3).build(dir)? -> Result<RollingFileAppender, InitError>` — files are named `harvest-claude.<YYYY-MM-DD>.log`; older files beyond the limit are deleted when a new file opens.

**[tracing-appender]** `tracing_appender::non_blocking(appender) -> (NonBlocking, WorkerGuard)` — dropping the guard stops the writer thread and loses buffered lines; `main` holds it to exit.

**[tracing-subscriber]** `fmt().with_writer(non_blocking).with_ansi(false).with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))` — needs feature `env-filter`.

### ratatui additions

**[ratatui]** `ratatui::restore()` — restores terminal (raw mode off, alternate screen left); callable repeatedly. Chain it in the panic hook: `let prev = std::panic::take_hook(); set_hook(Box::new(move |i| { let _ = restore_terminal(); prev(i) }))`.

**[ratatui]** `buf.set_stringn(x, y, string, max_width, style) -> (u16, u16)` — writes at most `max_width` columns, width-aware; use for floating labels instead of byte slicing.

### crossterm addition

**[crossterm]** Raw mode delivers Ctrl+C as `KeyEvent { code: Char('c'), modifiers: KeyModifiers::CONTROL }`, not as SIGINT.

### rstest ^0.27 — Table tests

**[rstest]** `#[rstest] #[case(input, expected)] fn name(#[case] input: .., #[case] expected: ..)` — one test per case; used for the real-record-shape parser tests.

---

## Architecture Decisions

### Sync, No Tokio

Main thread: `EventHandler::next()` blocking receive → `App::update(action)` → `App::render()`.
Background: crossterm poll thread + watcher thread → `mpsc::Sender<Action>` → main thread `mpsc::Receiver`.

No async runtime needed. File reading is synchronous (seek + read_line). Rendering is synchronous (ratatui). Event polling is synchronous (crossterm). Adding tokio would increase binary size ~2MB and complexity without benefit.

### JSONL Polling vs fs.watch

VS Code extension uses 500ms polling as primary strategy (fs.watch is unreliable cross-platform). TUI reads every tracked transcript every `JSONL_POLL_MS` (500 ms) and uses `notify` only for discovery of new files. The scanner drains the `notify` channel every frame, which costs only a channel read. This gives near-instant detection on most platforms with reliable fallback.

### Half-Block Rendering vs Canvas

VS Code extension uses HTML Canvas with `drawImage()`. TUI uses PixelBuffer → half-block Unicode characters. Key difference: TUI resolution is halved vertically (each cell = 2 pixels tall). TILE_SIZE stays 8, so character sprites are 8x16 pixels = 8 cols x 8 terminal rows. This matches the VS Code version's 16x16 tile / 2 = 8x8 terminal cells per tile equivalent.

### Sprite Generation: Procedural vs PNG

VS Code extension loads PNG sprite sheets. TUI generates sprites procedurally (hardcoded pixel arrays). Trade-off: no external asset dependency, but harder to modify visuals. Sprites are defined as `const` or `static` arrays for zero-cost embedding.

### Timer Architecture

VS Code extension uses `setTimeout`/`setInterval`. Rust TUI uses `Instant`-based timers checked on each tick of the main loop. `TimerManager` stores `HashMap<usize, Instant>` per timer type. On each frame, `check_expired()` iterates all timers and returns events for any past deadline. O(n) per frame where n = active agents (acceptable for <100 agents).

### Terminal Cleanup Safety

Pattern: a guard struct whose `Drop` calls the idempotent `restore_terminal()`, plus a panic hook that calls it before the report prints. The `App` is built before the terminal enters raw mode, so startup errors print on a normal screen.

```rust
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}
```

### Layout Compatibility

Layout JSON format matches VS Code extension: `{ version, cols, rows, tiles, furniture, tile_colors, layout_revision }`. TUI reads `~/.harvest-claude/layout.json`. The TUI never edits the layout; it writes only the bundled default when the file is missing and a backup before replacing an outdated revision.

---

## Verification

- Check this `impl.md` for existing findings before writing integration code.
- Record all new findings immediately. Not in conversation only.
- Verify via docs.rs, crates.io, or minimal test. No guessing.
