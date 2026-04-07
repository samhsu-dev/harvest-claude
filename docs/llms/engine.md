# Game Engine

> OfficeState, character FSM, z-sorted compositing to PixelBuffer, BFS pathfinding.

## Quick Start

```rust
let layout = load_layout("~/.pixel-agents/layout.json")?;
let mut state = OfficeState::from_layout(layout);
state.add_character(agent_id, palette);
// Each frame:
state.update(dt);
state.render(&mut pixel_buffer);
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/engine/state.rs` | OfficeState: layout, characters, furniture, seats |
| `src/engine/character.rs` | Character FSM (IDLE → WALK → TYPE), animation |
| `src/engine/pathfind.rs` | BFS shortest path on walkable tiles |
| `src/engine/sprite.rs` | SpriteData type, blit with alpha compositing |

## OfficeState

```rust
struct OfficeState {
    layout: OfficeLayout,
    tile_map: Vec<Vec<TileType>>,
    characters: Vec<Character>,
    furniture: Vec<FurnitureInstance>,
    seats: Vec<Seat>,
    blocked: HashSet<(u16, u16)>,
    walkable: Vec<(u16, u16)>,
}
```

## Character FSM

| State | Trigger | Behavior |
|-------|---------|----------|
| IDLE | No active tool | Wander randomly (BFS). After 3-6 moves, rest at seat 60-120s. |
| WALK | Path exists | Lerp between tiles at 3 tiles/sec. 2 walk frames, 200ms each. |
| TYPE | Tool active, at seat | Typing animation (Write/Edit/Bash) or reading (Read/Grep/Glob). 400ms/frame. |

Directions: Down, Up, Right. Left = horizontally flipped Right sprite.

## Z-Sorting

All entities collected into a `Vec<Drawable>`, sorted by bottom-Y pixel coordinate:
- Characters: `y + TILE_SIZE`.
- Furniture: `(row + footprint_rows) * TILE_SIZE`.
- Rendered back-to-front onto PixelBuffer via alpha-composited blit.

## Sprite Compositing

```rust
fn blit(buf: &mut PixelBuffer, sprite: &SpriteData, x: i16, y: i16) {
    for (sy, row) in sprite.pixels.iter().enumerate() {
        for (sx, &(r, g, b, a)) in row.iter().enumerate() {
            if a < 2 { continue; } // transparent
            let px = x + sx as i16;
            let py = y + sy as i16;
            if px >= 0 && py >= 0 && (px as u16) < buf.width && (py as u16) < buf.height {
                buf.set(px as u16, py as u16, (r, g, b, 255));
            }
        }
    }
}
```

## Gotchas

- Delta time capped at 100ms to prevent teleporting after terminal unfocus.
- Tile coordinates: `(col, row)` where col = x, row = y. Pixel coords: `(col * TILE_SIZE, row * TILE_SIZE)`.
- 8×8 tiles (half of webview's 16×16). Character sprites 8×16 (half of webview's 16×32).
- Wander uses BFS, not A*. Office grids are small enough that BFS is optimal.
