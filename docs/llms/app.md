# App & Rendering

> Main loop, event handling, terminal setup, half-block pixel rendering to ratatui Buffer.

## Quick Start

```rust
fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    crossterm::execute!(io::stdout(), EnableMouseCapture)?;
    let mut app = App::new()?;
    app.run(&mut terminal)?;
    ratatui::restore();
    Ok(())
}
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/main.rs` | CLI parsing, terminal setup/teardown, panic handler |
| `src/app.rs` | App struct, game loop, event dispatch |
| `src/renderer.rs` | PixelBuffer compositing, half-block Buffer writes |
| `src/ui/status_bar.rs` | Bottom status bar widget |

## Game Loop

```rust
loop {
    terminal.draw(|frame| app.render(frame))?;
    if crossterm::event::poll(Duration::from_millis(16))? {
        match crossterm::event::read()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            Event::Resize(w, h) => app.handle_resize(w, h),
            _ => {}
        }
    }
    app.update(dt);
    if app.should_quit { break; }
}
```

16ms poll = ~60fps. Rendering is diff-based (ratatui only writes changed cells).

## Half-Block Pixel Rendering

```rust
// Each terminal cell = 2 vertical pixels
fn render_pixel_pair(buf: &mut Buffer, col: u16, row: u16, top: Color, bottom: Color) {
    if let Some(cell) = buf.cell_mut((col, row)) {
        cell.set_symbol("▀");
        cell.set_fg(top);
        cell.set_bg(bottom);
    }
}
```

PixelBuffer (160×88 RGBA) → iterate rows by 2 → write half-block pairs to Buffer.

## Mouse Mapping

```
cell (column, row) → pixel (column, row * 2)
pixel (px, py) → tile (px / TILE_SIZE, py / TILE_SIZE)
```

Mouse precision: column-exact, row per-cell (cannot distinguish upper/lower half-block).

## Gotchas

- Always restore terminal on panic. Use `std::panic::set_hook` to call `ratatui::restore()`.
- `Color::Rgb` requires `COLORTERM=truecolor`. macOS Terminal.app does NOT support it.
- Minimum terminal size: 160 cols × 46 rows. Check on startup and resize events.
