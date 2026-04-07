# Layout Format

> OfficeLayout schema, tile types, persistence, compatibility with webview version.

## Schema

```rust
struct OfficeLayout {
    version: u8,           // Must be 1
    cols: u16,             // Grid width in tiles
    rows: u16,             // Grid height in tiles
    tiles: Vec<TileType>,  // Flat row-major (len = cols × rows)
    furniture: Vec<PlacedFurniture>,
}

struct PlacedFurniture {
    uid: String,
    furniture_type: String,  // Asset ID
    col: i16,                // Can be negative (wall items)
    row: i16,
}

enum TileType {
    Void = 0,     // Transparent, non-walkable
    Floor1 = 1,   // Walkable floor patterns 1-7
    Floor2 = 2,
    Floor3 = 3,
    Floor4 = 4,
    Floor5 = 5,
    Floor6 = 6,
    Floor7 = 7,
    Wall = 100,   // Non-walkable wall
}
```

## Persistence

- File: `~/.pixel-agents/layout.json`.
- Same format as webview version. JSON with `version`, `cols`, `rows`, `tiles`, `furniture` fields.
- On first run: create default layout (20×11, basic office).
- Atomic write: write to `.tmp`, rename over original.

## Webview Compatibility

The TUI version uses 8×8 tiles; the webview uses 16×16. Layouts are stored at tile-level (not pixel-level), so the same `layout.json` works for both. The TUI just renders each tile at half the pixel resolution.

Furniture positions are tile-based, so they transfer directly. Visual fidelity differs but spatial layout is identical.

## Default Layout

```
20 cols × 11 rows
Row 0:    Wall × 20
Row 1-2:  Floor + desks + chairs (workspace area)
Row 3-9:  Floor + misc furniture
Row 10:   Floor (walkway)
```

## Gotchas

- `tiles` is a flat array, row-major. Index = `row * cols + col`.
- Furniture `col`/`row` can be negative (wall-mounted items above the grid).
- Grid max: 64×64. Default: 20×11.
