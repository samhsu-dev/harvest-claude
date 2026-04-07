# Sprites & Assets

> Embedded sprite data, character palettes, animation frames, furniture tiles.

## Sprite Format

```rust
type SpriteData = Vec<Vec<(u8, u8, u8, u8)>>; // rows × cols of RGBA pixels
// (0, 0, 0, 0) = transparent
```

All sprites embedded at compile time via `const` arrays or `include_bytes!` + decode.

## Character Sprites

8×16 pixels (half of webview's 16×32). 6 palettes.

```
Frame layout per character:
  Walk1  Walk2  Type1  Type2  Read1  Read2
  8×16   8×16   8×16   8×16   8×16   8×16

Direction rows:
  Row 0: Down
  Row 1: Up
  Row 2: Right (Left = flipped at runtime)
```

Total per palette: 6 frames × 3 directions = 18 sprites of 8×16.

## Palettes

6 base colors (matching webview `CHARACTER_PALETTES`):

| Index | Skin | Hair | Shirt |
|-------|------|------|-------|
| 0 | light | brown | blue |
| 1 | light | black | red |
| 2 | medium | brown | green |
| 3 | dark | black | purple |
| 4 | light | blonde | orange |
| 5 | medium | red | teal |

Beyond 6 agents: reuse palette with hue-shifted colors.

## Furniture Sprites

8×8 pixels per tile (half of webview's 16×16). Simplified from webview originals.

| Category | Items | Notes |
|----------|-------|-------|
| Desks | desk_front, desk_side | Surface for typing animation |
| Chairs | chair_front, chair_back | Seat derivation source |
| Electronics | monitor, laptop | Auto-state (on when agent active) |
| Plants | plant, cactus | Decorative |
| Storage | bookshelf | Decorative |

## Floor Tiles

8×8 solid color blocks. 7 patterns simplified to flat colors:

| Index | Default Color |
|-------|--------------|
| 0 | `#5a4a3a` (dark wood) |
| 1 | `#6b5b4b` (light wood) |
| 2 | `#4a5a6a` (blue tile) |
| 3 | `#5a6a5a` (green tile) |
| 4 | `#6a6a6a` (gray) |
| 5 | `#5a5a4a` (beige) |
| 6 | `#3a3a4a` (dark blue) |

## Wall Tiles

8×16 per tile (8 wide × 16 tall for 3D effect). Solid color with darker bottom edge.

## Status Bubbles

Text-based (not pixel sprites):
- Permission: `[...]` in amber
- Waiting: `[✓]` in green
- Active tool: `[toolname]` in white

## Gotchas

- Sprites designed for 8×8 tile grid. Webview uses 16×16. Layout loading must halve coordinates.
- Left-facing sprites = horizontal flip of right-facing. Flip at render time, not stored separately.
- Alpha threshold: `a < 2` treated as transparent (matches webview's `PNG_ALPHA_THRESHOLD`).
