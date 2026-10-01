# Sprites & Assets

> Embedded character, furniture, companion, and produce sprites with palettes.

## Quick Start

```rust
let sprite = character_sprite(palette, Direction::Down, AnimType::Walk, frame);
buf.blit(&sprite, x, y);
let pet = companion_sprite(CompanionKind::Cat, frame);
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src/render/sprites.rs` | Character, companion, furniture, and produce sprite builders |
| `src/render/colorize.rs` | Hue shift for palettes beyond the sixth agent |
| `src/render/bubble.rs` | Permission and waiting bubbles |
| `src/render/buffer.rs` | `PixelBuffer` |

## API

| Item | Meaning |
|------|---------|
| `SpriteData` | `Vec<Vec<Pixel>>`; transparent pixel is `T` |
| Character sprite | 8x8 pixels per `(direction, anim, frame)` |
| `companion_sprite(kind, frame)` | 8x8, two frames: normal and bob |
| `BubbleKind` | `Permission`, `Waiting` |
| `Direction` | `Down`, `Up`, `Left`, `Right` |

## Configuration

| Constant | Value |
|----------|-------|
| `TILE_SIZE` | 8 pixels |
| Palette count | 6: Farmer, Rancher, Gardener, Berry picker, Harvest worker, Fisher |
| `WALK_FRAME_MS` | 150 |
| `TYPE_FRAME_MS` | 300 |

Colors come from the Dawnbringer 16 palette.

## Gotchas

- Palette index wraps at 6 (`palette % 6`). The registry assigns the least-used palette.
- Once all 6 palettes are in use, a random hue shift is added to the next agent.
- `Left` mirrors the `Right` sprite. Only `Down`, `Up`, and `Right` are drawn.
- Each character sprite is 8 pixels wide and 8 tall. Each terminal cell shows two vertical pixels.
- Sprites are compile-time data. No asset files load at runtime.
- Produce piles grow by tier: `Warehouse::tier(produce)` selects the sprite variant.
