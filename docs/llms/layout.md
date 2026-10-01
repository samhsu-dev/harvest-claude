# Layout Format

> `OfficeLayout` schema, furniture catalog, default farm, and load policy.

## Quick Start

```json
{
  "version": 1,
  "cols": 28,
  "rows": 16,
  "layout_revision": 14,
  "tiles": [1, 1, 2, 5, 0],
  "furniture": [{"uid": "f1", "type": "CROP_PLOT", "col": 3, "row": 4}]
}
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/layout/persistence.rs` | `load_or_default`, `validate_layout`, default layout |
| `src/layout/furniture.rs` | Catalog, footprints, seats, categories |
| `src/types.rs` | `TileType`, `OfficeLayout`, `PlacedFurniture` |

## Schema

| Field | Type |
|-------|------|
| `version` | `u32` |
| `cols`, `rows` | Grid size. Default 28x16, maximum 64x64 |
| `tiles` | `Vec<u8>`, `cols*rows` entries, row-major |
| `furniture` | `Vec<PlacedFurniture>` |
| `tile_colors` | Optional per-tile colors |
| `layout_revision` | Optional `u32` |

`PlacedFurniture`: `uid`, `type`, `col: i16`, `row: i16`, optional `color`.

`TileType`: Void 0, Grass 1, GrassDark 2, Dirt 3, DirtDark 4, Water 5, Sand 6, Stone 7, Fence 100.

## Furniture Catalog

All entries have a 1x1 footprint.

| Group | Types |
|-------|-------|
| Crops | `CROP_PLOT`, `CROP_PLOT_ON` |
| Seats | `STUMP_FRONT`, `STUMP_BACK`, `STUMP_LEFT`, `STUMP_RIGHT`, `FISHING_SPOT` |
| Trees | `TREE`, `TREE_FRUIT` |
| Structures | `CABIN_WALL`, `BARN_WALL`, `HOME`, `BARN`, `CHICKEN_COOP`, `COW_PEN`, `WELL`, `MAILBOX`, `MAILBOX_ON`, `SCARECROW` |
| Decor | `LANTERN`, `FLOWER`, `BUSH` |
| Produce piles | `WHEAT_PILE`, `FRUIT_BASKET`, `FISH_PILE` |
| Fences | `FENCE_H`, `FENCE_V` |

## Configuration

- User layout path: `~/.harvest-claude/layout.json`. `--layout <path>` overrides it.
- `BUNDLED_REVISION` = 14.

## Load Policy

| State of the file | Result |
|-------------------|--------|
| Missing | Default layout written and used |
| `layout_revision` below `BUNDLED_REVISION` | Copied to `layout.json.bak-r<rev>`, then replaced by the default |
| Invalid (parse error, size mismatch, out-of-range furniture) | Default used in memory, file untouched |
| Missing explicit `--layout` path | Error |

## Gotchas

- `validate_layout` rejects furniture outside the grid and `tiles` of the wrong length.
- Seats derive from `STUMP_*` and `FISHING_SPOT`. A layout without seats leaves agents idle.
- Furniture categories: `desk` for crops, `seating` for seats, `electronics` for mailboxes, `building` for `HOME` and `BARN`, `decor` for the rest.
- Add a new furniture type in the catalog and in the sprite table together.
