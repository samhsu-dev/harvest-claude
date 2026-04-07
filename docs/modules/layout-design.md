# Layout Module Design

Layout persistence, serialization, version migration, and furniture catalog.

```
layout/
├── mod.rs
├── persistence.rs  # Layout/config file I/O (~/.pixel-agents/)
├── serializer.rs   # OfficeLayout ↔ OfficeState conversion
└── furniture.rs    # Furniture catalog, footprint, placement
```

---

## persistence.rs

| Function | Signature | Description |
|----------|-----------|-------------|
| `load_layout` | `(path: &Path) -> Result<OfficeLayout>` | Read + deserialize + migrate |
| `save_layout` | `(path: &Path, layout: &OfficeLayout) -> Result<()>` | Atomic write (write tmp, rename) |
| `default_layout` | `() -> OfficeLayout` | Bundled default office |
| `pixel_agents_dir` | `() -> Result<PathBuf>` | `~/.pixel-agents/`, create if missing |
| `load_or_default` | `() -> Result<OfficeLayout>` | Try file → fallback to default |

### Layout Version Migration

On load, apply migrations in order:

1. **VOID value**: old value 8 → new value 0. Detected by `!layout_revision AND tiles.contains(8)`.
2. **Furniture types**: legacy names mapped (`desk` → `DESK_FRONT`, `chair` → `WOODEN_CHAIR_FRONT`, etc.). Unmapped types pass through.
3. **Tile colors**: old patterns (Floor1=beige, Floor2=brown, Floor3=purple, Floor4=tan) → generate `TileColor` per tile. No color for non-floor tiles.
4. **Layout revision**: if bundled revision > file revision → reset to bundled default.

---

## serializer.rs

Transforms `OfficeLayout` (serialized) into runtime game state structures.

| Function | Signature | Description |
|----------|-----------|-------------|
| `build_tile_map` | `(layout: &OfficeLayout) -> Vec<Vec<TileType>>` | Flat tiles → 2D grid |
| `build_blocked` | `(furniture: &[FurnitureInstance]) -> HashSet<TilePos>` | Furniture footprint tiles |
| `build_walkable` | `(tile_map, blocked) -> HashSet<TilePos>` | Floor tiles minus blocked |
| `build_furniture` | `(layout: &OfficeLayout) -> Vec<FurnitureInstance>` | PlacedFurniture → FurnitureInstance with sprites |
| `build_desk_z_map` | `(furniture: &[FurnitureInstance]) -> HashMap<TilePos, f32>` | Pre-compute desk z_y per tile for surface items |

### Desk Z Pre-Computation

Iterate all desk-category furniture. For each tile in footprint, store `z_y = (row + footprint_h) * TILE_SIZE`. Surface items use `max(sprite_bottom, desk_z + 0.5)` for correct layering.

---

## furniture.rs

```rust
pub struct FurnitureInstance {
    pub uid: String,
    pub furniture_type: String,
    pub col: i16,
    pub row: i16,
    pub sprite: SpriteData,
    pub footprint: Vec<TilePos>,
    pub z_y: f32,
    pub is_seat: bool,
    pub facing: Option<Direction>,
    pub category: Option<String>,     // "electronics", "desk", etc.
    pub mirrored: bool,               // LEFT = horizontally flipped RIGHT
}
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `furniture_footprint` | `(kind: &str) -> Vec<(i16, i16)>` | Relative tile offsets |
| `is_surface_item` | `(kind: &str) -> bool` | Items placed on desks (monitor, lamp) |
| `is_electronics` | `(kind: &str) -> bool` | Items that switch ON when agent faces them |
