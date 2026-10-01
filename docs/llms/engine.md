# Game Engine

> Farm state, character state machine, pathfinding, companions, and the produce warehouse.

## Quick Start

```rust
let mut state = OfficeState::from_layout(layout);
let id = state.add_character(agent_id, palette, hue_shift);
state.update(dt_seconds);            // every tick
let seat = state.find_nearest_free_seat(pos);
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/engine/state.rs` | `OfficeState`: characters, seats, walkable tiles, furniture sprites |
| `src/engine/character.rs` | `Character` state machine, animation, companions, delivery |
| `src/engine/seat.rs` | `derive_seats`, `facing_from_context` |
| `src/engine/pathfind.rs` | `bfs` over walkable tiles (4-connected) |
| `src/engine/warehouse.rs` | `Warehouse`, `load_warehouse`, `save_warehouse`, `produce_for_anim` |

## API

| Item | Meaning |
|------|---------|
| `Character::new` | `(agent_id, pos, palette, hue_shift)` |
| `Character::update` | `(dt, walkable, seats)` advances movement and frames |
| `set_active`, `set_idle`, `set_waiting`, `set_permission` | Status transitions |
| `set_dormant`, `start_dormant_walk`, `wake_from_dormant` | Dormant handling |
| `start_delivery`, `take_delivery` | Carry produce to the barn or warehouse |
| `add_companion`, `remove_companion` | Keyed by tool id; `CompanionKind` Chicken, Cat, Dog |
| `Character::anim_for_tool` | Tool name to `AnimType` |
| `Warehouse` | `add`, `count`, `total`, `tier` |

Animations: `AnimType` Walk, Type, Read, Fish, Farm, Harvest. Produce: `ProduceType` Wheat, Fruit, Fish.

## Configuration

| Constant | Value |
|----------|-------|
| `WALK_FRAME_MS` | 150 |
| `TYPE_FRAME_MS` | 300 |
| `WALK_SPEED` | 3.0 tiles per second |
| `REST_DURATION` | 120 to 240 s |
| `MAX_AGENTS` | 8 |

- Warehouse file: `~/.harvest-claude/warehouse.json`.

## Gotchas

- Seats derive from furniture: `STUMP_*` and `FISHING_SPOT` are seats. The stump variant sets the facing direction.
- `Character::update` counts completed wander paths in a saturating counter. Never use unchecked `u8` addition.
- Bubbles: `BubbleKind::Permission` stays until the tool result arrives. `BubbleKind::Waiting` marks turn end.
- A tool result in Permission state returns the character to Active and clears the bubble.
- A background launch adds a companion. The final result for that tool id removes it.
- `load_warehouse` returns `Result`. A corrupt file is renamed `warehouse.json.corrupt` with a warning.
- `save_warehouse` writes a temp file and renames it. A crash leaves no partial file.
- No matrix spawn or despawn effect exists.
