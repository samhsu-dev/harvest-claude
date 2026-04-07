# UI Module Design

Terminal user interface: status bar widget and input mapping.

```
ui/
├── mod.rs
├── status_bar.rs   # Bottom status bar widget
└── input.rs        # Mouse/keyboard → Action mapping
```

---

## status_bar.rs

Bottom bar widget. Implements `Widget` for `&StatusBar`.

**Layout**:
- Left: agent count, palette-colored dots
- Center: selected agent name + current tool + status text
- Right: keybindings hint (`q:quit  ←→:select  esc:deselect`)

---

## input.rs

Maps mouse events to semantic `Action` variants. Keyboard events map directly in `event.rs`; mouse requires hit-testing against office state.

| Function | Signature | Description |
|----------|-----------|-------------|
| `tile_from_cell` | `(col, row, offset: (u16, u16)) -> TilePos` | Terminal cell → tile (half-block aware: `pixel_y = row * 2`) |
| `handle_mouse_click` | `(state: &OfficeState, tile: TilePos) -> Option<Action>` | Hit-test characters → `Action::Key` or bubble dismiss |

Mouse click flow: `Action::Mouse(event)` → `App::update()` calls `handle_mouse_click()` → returns optional follow-up `Action` (selection change, bubble dismiss).

Keyboard mapping (in `event.rs`, not here):
- `q` / `Esc` → `Action::Quit`
- `←` / `→` → cycle selection
- Click character → select / dismiss bubble
