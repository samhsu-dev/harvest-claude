# Engine Module Design

Game world simulation: character FSM, office state, pathfinding, seats, furniture animation, visual effects.

```
engine/
├── mod.rs
├── character.rs    # Character FSM (Idle/Walk/Type)
├── state.rs        # OfficeState (world, characters, seats, furniture animation)
├── pathfind.rs     # BFS on 4-connected walkable grid
├── seat.rs         # Seat derivation, assignment, facing logic
└── matrix.rs       # Matrix spawn/despawn effect
```

---

## character.rs

```rust
pub struct Character {
    pub agent_id: usize,
    pub state: CharState,
    pub is_active: bool,              // has active tools — drives FSM
    pub pos: (f32, f32),              // sub-tile pixel coords
    pub target: Option<TilePos>,
    pub path: VecDeque<TilePos>,
    pub direction: Direction,         // LEFT = flipped RIGHT sprite at render
    pub palette: u8,
    pub hue_shift: Option<i16>,
    pub seat_id: Option<usize>,
    pub seat_timer: f32,              // rest countdown; -1.0 = skip next rest
    pub anim_frame: u8,
    pub anim_timer: f32,
    pub wander_timer: f32,
    pub wander_count: u8,            // increments per completed path, not per tile
    pub wander_limit: u8,            // re-randomized on every seat arrival
    pub tool_name: Option<String>,
    pub bubble: Option<BubbleState>,
    pub matrix_effect: Option<MatrixEffect>,
}

pub struct BubbleState {
    pub kind: BubbleKind,
    pub timer: f32,                  // countdown for Waiting; unused for Permission
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(agent_id, pos, palette, hue_shift) -> Self` | Initialize at position, Idle state |
| `update` | `(&mut self, dt: f64, walkable: &HashSet<TilePos>, seats: &[Seat])` | FSM tick: position, animation, wander |
| `set_active` | `(&mut self, tool_name: &str, seat_pos: Option<TilePos>, path: VecDeque<TilePos>)` | Walk to seat → Type |
| `set_idle` | `(&mut self)` | Set `is_active = false`, `seat_timer = -1.0` (skip rest) |
| `set_waiting` | `(&mut self)` | Show waiting bubble (auto-fade 2s) |
| `set_permission` | `(&mut self)` | Show permission bubble (persistent) |
| `dismiss_bubble` | `(&mut self)` | Clear bubble |
| `sprite_key` | `(&self) -> (Direction, AnimType, u8)` | Current sprite lookup key |
| `is_reading_tool` | `(tool_name: &str) -> bool` | Read/Grep/Glob/WebFetch/WebSearch |

### Animation Frames

- **Walk**: 4-frame cycle `[0,1,2,1]` at 150ms per frame
- **Type**: 2-frame alternate `[0,1]` at 300ms per frame
- **Read**: 2-frame alternate `[0,1]` at 300ms per frame
- **Idle**: static frame 0, no animation
- **LEFT direction**: RIGHT sprite horizontally flipped at render time

### FSM Transitions

```
Idle  → Walk   set_active() called, pathfind to seat
Idle  → Walk   wander_timer expires, random BFS destination

Walk  → Type   path exhausted AND is_active (at seat, tool running)
Walk  → Idle   path exhausted AND NOT is_active
Walk  → Walk   wander continues (next segment)

Type  → Idle   set_idle() called (turn ends); seat_timer = -1.0 skips rest
Type  → Walk   new tool requires seat change
```

### Movement

- Speed: `WALK_SPEED` tiles/sec → `move_progress += (WALK_SPEED / TILE_SIZE) * dt`
- Path cleared and recalculated when agent becomes active mid-wander

### Wander Behavior

- `wander_count` increments per completed path (not per tile step)
- After `wander_limit` paths: return to seat, rest for `REST_DURATION_MIN..MAX` seconds
- `wander_limit` re-randomized on **every** seat arrival
- `seat_timer = -1.0` sentinel: turn just ended, skip rest on next Walk→Type

---

## state.rs

```rust
pub struct OfficeState {
    pub layout: OfficeLayout,
    pub tile_map: Vec<Vec<TileType>>,
    pub characters: Vec<Character>,
    pub furniture: Vec<FurnitureInstance>,
    pub seats: Vec<Seat>,
    pub blocked: HashSet<TilePos>,
    pub walkable: HashSet<TilePos>,
    pub furniture_anim_timer: f32,
    pub desk_z_by_tile: HashMap<TilePos, f32>,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `from_layout` | `(layout: OfficeLayout) -> Self` | Compute tile_map, seats, blocked, walkable, desk_z_by_tile |
| `add_character` | `(&mut self, agent_id, palette, hue_shift) -> usize` | Spawn at free seat or random walkable |
| `remove_character` | `(&mut self, agent_id: usize)` | Free seat, remove from vec |
| `update` | `(&mut self, dt: f64)` | Tick characters, matrix effects, furniture animation |
| `find_free_seat` | `(&self) -> Option<usize>` | Prefer seats facing electronics (3 tiles deep), then any |
| `find_nearest_free_seat` | `(&self, near: TilePos) -> Option<usize>` | Manhattan distance, for sub-agents |
| `rebuild_furniture_sprites` | `(&mut self)` | Swap electronics ON/OFF based on facing agents |
| `character_by_agent` | `(&self, id) -> Option<&Character>` | Lookup by agent ID |
| `character_by_agent_mut` | `(&mut self, id) -> Option<&mut Character>` | Mutable lookup |
| `character_at_tile` | `(&self, pos: TilePos) -> Option<usize>` | Hit-test: CHARACTER_HIT bounds |

### Furniture Animation

- `furniture_anim_timer` advances each frame
- Frame index: `floor(timer / FURNITURE_ANIM_INTERVAL_SEC) % frame_count`
- New frame triggers `rebuild_furniture_sprites()`: electronics facing active agent → ON sprite
- Detection: scan 3 tiles in facing direction + 1 tile to each side (`AUTO_ON_FACING_DEPTH`)

---

## pathfind.rs

```rust
pub fn bfs(
    walkable: &HashSet<TilePos>,
    from: TilePos,
    to: TilePos,
    own_seat: Option<TilePos>,
) -> Option<Vec<TilePos>>
```

BFS on 4-connected grid. `own_seat` temporarily unblocked (character's seat may be in `blocked`). Path excludes `from`, includes `to`. Returns `None` if unreachable. Empty vec if `from == to`.

---

## seat.rs

```rust
pub struct Seat {
    pub col: u16,
    pub row: u16,
    pub facing: Direction,
    pub occupied_by: Option<usize>,
    pub furniture_uid: String,
}
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `derive_seats` | `(furniture: &[FurnitureInstance], tile_map: &[Vec<TileType>]) -> Vec<Seat>` | Extract seats from chairs, multi-tile = uid, uid:1, uid:2 |
| `facing_from_context` | `(pos: TilePos, furniture: &[FurnitureInstance]) -> Direction` | Chair orientation > adjacent desk > Down |

Multi-tile chairs: one seat per non-background footprint tile. UIDs: first = `uid`, rest = `uid:N`.

---

## matrix.rs

Green rain spawn/despawn effect (0.3s). Per-column stagger 0-30% of duration.

```rust
pub struct MatrixEffect {
    pub spawning: bool,
    pub elapsed: f32,
    pub columns: Vec<MatrixColumn>,
}

pub struct MatrixColumn {
    pub x: u16,
    pub offset: f32,
    pub chars: Vec<(u8, u8, u8)>,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new_spawn` | `(width, height) -> Self` | Reveal effect |
| `new_despawn` | `(width, height) -> Self` | Consume effect |
| `update` | `(&mut self, dt: f32) -> bool` | Advance, `true` when complete |
| `apply` | `(&self, buf: &mut PixelBuffer, x, y)` | Composite onto buffer |

Sweep length: sprite height + 6 trail rows. Head: bright green `#ccffcc`. Trail: 3 brightness levels at 0.33/0.66 thresholds. Hash-based 30fps flicker, ~70% visibility.
