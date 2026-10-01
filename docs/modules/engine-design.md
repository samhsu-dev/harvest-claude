# Engine Module Design

Farm world simulation: character state machine, world state, pathfinding, seats, produce warehouse. Domain rules: `model.md`.

## Design Overview

```
engine/
├── mod.rs
├── character.rs    # Character, BubbleState, Companion
├── state.rs        # OfficeState
├── pathfind.rs     # BFS on the 4-connected walkable grid
├── seat.rs         # Seat derivation and facing
└── warehouse.rs    # Warehouse, load/save, produce mapping
```

- **Classes**: `Character`, `BubbleState`, `Companion`, `OfficeState`, `Seat`, `Warehouse`.
- **Relationships**: `OfficeState` contains `Character`, `Seat`, `FurnitureInstance`. `Character` contains `BubbleState` and `Companion`. `engine` imports `types` and `constants` only, plus `layout::furniture` for `FurnitureInstance`.
- **Roles**. Data holders: `Seat`, `BubbleState`, `Companion`, `Warehouse`. Orchestrator: `OfficeState`. Helpers: `bfs`, `derive_seats`, `Character` (state machine owner).
- **Removed**: `matrix.rs`, `MatrixEffect`, `MatrixColumn`, field `Character::matrix_effect` (spawn and despawn effect was never triggered).

## Class / Type Specifications

### Character (`character.rs`)

Plain-data struct with public fields, each independently valid. Derives `Debug, Clone`.

| Field | Meaning |
|-------|---------|
| `agent_id` | Owning Agent id |
| `state: CharState` | `Idle`, `Walk`, `Type` |
| `is_active` | A tool is running; drives Walk→Type |
| `is_dormant` | Hidden inside the building |
| `pos`, `target`, `path`, `direction` | Sub-tile pixel position, movement target, remaining tiles, facing |
| `palette`, `hue_shift` | Sprite colouring |
| `seat_id`, `seat_timer` | Assigned seat; rest countdown, `-1.0` skips next rest |
| `anim_frame`, `anim_timer` | Animation cursor |
| `wander_timer`, `wander_count`, `wander_limit` | Wander schedule; count increments per completed path, saturating |
| `tool_name` | Name of the running tool |
| `bubble: Option<BubbleState>` | Permission (persistent) or Waiting (auto-fade) |
| `companions: Vec<Companion>` | One per running background tool |
| `work_anim: Option<AnimType>` | Animation from seat furniture: Fish, Farm, Harvest |
| `pending_dormant` | Walk home, then hide |
| `pending_delivery: Option<ProduceType>` | Walk to barn, then deposit |

`BubbleState { kind: BubbleKind, timer: f32 }`. `Companion { tool_id, kind: CompanionKind, offset, anim_frame, anim_timer }`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `(agent_id, pos, palette, hue_shift) -> Self` | Idle at the tile |
| `update` | `(&mut self, dt: f64, walkable: &HashSet<TilePos>, seats: &[Seat])` | Movement, animation, wander, bubble timers |
| `set_active` | `(&mut self, tool_name: &str, seat_pos: Option<TilePos>, path: VecDeque<TilePos>)` | Walk to seat, then Type |
| `set_idle` | `(&mut self)` | Clear activity; skip next rest |
| `set_waiting` / `set_permission` / `dismiss_bubble` | `(&mut self)` | Bubble control |
| `set_dormant`, `start_dormant_walk`, `wake_from_dormant` | | Dormant life cycle |
| `start_delivery` | `(&mut self, produce, path) -> bool` | Begin barn walk; false when a delivery is already pending |
| `take_delivery` | `(&mut self) -> Option<ProduceType>` | Consume the pending delivery on arrival |
| `add_companion` / `remove_companion` | `(tool_id, kind)` / `(&str)` | Companion list; add ignores a known tool id |
| `sprite_key` | `(&self) -> (Direction, AnimType, u8)` | Sprite lookup key |
| `anim_for_tool` / `is_reading_tool` | `(&str)` | Read, Grep, Glob, WebFetch, WebSearch → `Read` |
| `current_tile` | `(&self) -> TilePos` | Tile containing `pos` |

Frames: Walk `[0,1,2,1]` at `WALK_FRAME_MS`; Type, Read, Farm, Harvest, Fish two frames at `TYPE_FRAME_MS`; Idle frame 0. Left facing is the Right sprite mirrored at render time.

### OfficeState (`state.rs`, orchestrator)

Fields: `layout`, `tile_map: Vec<Vec<TileType>>`, `characters`, `furniture`, `seats`, `blocked`, `walkable`, `furniture_anim_timer`, `desk_z_by_tile`.

| Method | Signature | Behavior |
|--------|-----------|----------|
| `from_layout` | `(layout) -> Self` | Derive tile map, seats, blocked, walkable, desk z |
| `add_character` | `(&mut self, agent_id, palette, hue_shift) -> usize` | Spawn at a free seat, else a random walkable tile |
| `remove_character` | `(&mut self, agent_id)` | Free the seat, remove |
| `update` | `(&mut self, dt: f64)` | Tick characters and furniture animation |
| `find_free_seat` / `find_nearest_free_seat` | `() / (near)` | Seat choice; nearest by Manhattan distance |
| `find_near_furniture` | `(&self, from, kind: &str) -> Option<TilePos>` | Nearest furniture of a kind, for barn and home |
| `rebuild_furniture_sprites` | `(&mut self)` | Electronics ON when an active character faces them within `AUTO_ON_FACING_DEPTH` |
| `character_by_agent`, `character_by_agent_mut`, `character_at_tile` | lookups | Hit test uses character bounds |

`character_at_tile` is a pure geometric test. Excluding Dormant characters from selection is the caller's rule (`design.md`, App).

### Seat (`seat.rs`, data holder)

`Seat { col, row, facing, occupied_by: Option<usize>, furniture_uid }`, method `tile_pos() -> TilePos`. Multi-tile chairs yield one seat per footprint tile, uids `uid`, `uid:1`, `uid:2`.

### Warehouse (`warehouse.rs`, data holder)

`Warehouse { counts: HashMap<ProduceType, u32> }`, derives `Debug, Clone, Default, Serialize, Deserialize`.

| Method | Behavior |
|--------|----------|
| `add(produce)` | Increment, saturating |
| `count(produce)`, `total()` | Read |
| `tier(produce) -> u8` | 0 empty, 1 for 1–3, 2 for 4–9, 3 for 10+ |

## Function Specifications

**bfs(walkable, from, to, own_seat) -> Option<Vec<TilePos>>** (`pathfind.rs`) — Shortest 4-connected path excluding `from`, including `to`. `own_seat` is treated as walkable. `None` when unreachable; empty when `from == to`.

**derive_seats(furniture, tile_map) -> Vec<Seat>** (`seat.rs`) — One seat per chair footprint tile.

**facing_from_context(pos, furniture) -> Direction** (`seat.rs`) — Chair orientation, else adjacent desk, else Down.

**load_warehouse(config_dir: &Path) -> Result<Warehouse>** — Missing file gives an empty warehouse. A file that cannot be parsed is renamed to `warehouse.json.corrupt`, a `warn` is logged, and an empty warehouse is returned. A read error returns `Err` with the path as context. Behavior change from the old signature: a corrupt file is no longer silently replaced on the next save.

**save_warehouse(config_dir: &Path, warehouse: &Warehouse) -> Result<()>** — Writes `warehouse.json.tmp` then renames over `warehouse.json`, so a crash never leaves a truncated file. Creates the directory when missing.

**produce_for_anim(anim: AnimType) -> Option<ProduceType>** — Farm → Wheat, Harvest → Fruit, Fish → Fish, others none.

## Error Handling

Only warehouse I/O returns `Result`. Everything else is infallible; an unreachable path leaves the character where it is.
