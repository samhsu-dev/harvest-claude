# Render Module Design

Pixel rendering pipeline: framebuffer, sprite generation, z-sorted composition, colorization, bubbles.

```
render/
├── mod.rs
├── buffer.rs      # PixelBuffer (RGBA framebuffer)
├── sprites.rs     # Procedural sprite generation
├── composer.rs    # Z-sorted scene composition
├── colorize.rs    # HSL colorization (two modes)
└── bubble.rs      # Speech bubble sprites
```

---

## buffer.rs

In-memory RGBA framebuffer. All sprites composited here before terminal output.

```rust
pub struct PixelBuffer {
    width: u16,
    height: u16,
    pixels: Vec<Pixel>,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(width, height) -> Self` | Allocate zeroed buffer |
| `clear` | `(&mut self, color: Pixel)` | Fill all pixels |
| `blit` | `(&mut self, sprite: &SpriteData, x: i16, y: i16)` | Alpha-composite (signed coords for off-screen) |
| `blit_flipped` | `(&mut self, sprite: &SpriteData, x: i16, y: i16)` | Horizontal flip blit (for LEFT direction) |
| `get` | `(&self, x, y) -> Pixel` | Read pixel |
| `set` | `(&mut self, x, y, color: Pixel)` | Write pixel |
| `width` / `height` | `(&self) -> u16` | Dimensions |

Implements `Widget` for `&PixelBuffer`: each terminal cell = two vertical pixels via `▀` with fg (top) / bg (bottom) colors.

---

## sprites.rs

Procedural sprite generation. All sprites are hardcoded pixel arrays — no external PNG dependencies.

| Function | Signature | Description |
|----------|-----------|-------------|
| `character_sprite` | `(palette, direction, anim, frame) -> SpriteData` | 8x16 character |
| `floor_sprite` | `(tile: TileType) -> SpriteData` | 8x8 floor tile |
| `fence_sprite` | `(neighbors: u8) -> SpriteData` | Fence, auto-tile bitmask |
| `furniture_sprite` | `(kind: &str) -> SpriteData` | 8x8 furniture item |
| `produce_sprite` | `(kind: &str, tier: u8) -> SpriteData` | Warehouse pile, tier 1–3 |
| `companion_sprite` | `(kind: CompanionKind, frame: u8) -> SpriteData` | Chicken, cat, dog; 2 frames |
| `status_bubble` | `(kind: BubbleKind) -> SpriteData` | Speech bubble; also `permission_bubble`, `waiting_bubble` |
| `character_outline` | `(sprite: &SpriteData, alpha: u8) -> SpriteData` | White outline around opaque pixels |

### Fence Auto-Tile Bitmask

Built at render time from tile map neighbors: N=1, E=2, S=4, W=8. 4-bit index into 16 variants. The tile type is `Fence`; no wall tile exists. Algorithm:
```
mask = 0
if north is Fence → mask |= 1
if east is Fence → mask |= 2
if south is Fence → mask |= 4
if west is Fence → mask |= 8
```

### Outline Algorithm

1. Expand sprite by 2px on all sides (transparent border)
2. For each opaque pixel in original, mark 4 cardinal neighbors as white
3. Clear pixels that overlap original sprite (outline only on exterior)
4. Selected character: alpha 255. Hovered character: alpha 128.

---

## composer.rs

Z-sorted scene composition onto PixelBuffer.

Callers convert engine state into plain render structs; `render` never imports `engine`.

| Type | Content |
|------|---------|
| `SceneInput<'a>` | `tile_map: &[TileType]` (flat, row-major), `cols`, `rows`, `furniture: &[FurnitureRender]`, `characters: &[CharacterRender]`, `tile_colors: &[(TilePos, TileColor)]`, `selected: Option<usize>` |
| `FurnitureRender` | `kind`, `col`, `row`, `color`, `is_seat`, `tier` (produce pile size) |
| `CharacterRender` | palette, direction, anim type, frame, pixel position, hue shift, bubble (`BubbleRender`), companions (`CompanionRender`), flipped |
| `BubbleRender` | `kind`, `timer` |
| `CompanionRender` | `kind`, `offset_x`, `offset_y`, `frame` |

| Function | Signature | Description |
|----------|-----------|-------------|
| `compose_scene` | `(buf: &mut PixelBuffer, input: &SceneInput<'_>)` | Clear, tiles, then z-sorted drawables |
| `render_tiles` | `(buf, tile_map, cols, rows, tile_colors)` | Floor and fence base layer |
| `collect_drawables` | `(furniture, characters, selected) -> Vec<Drawable>` | Private; gather entities with `z_y` |
| `render_sorted` | `(buf, drawables: &mut [Drawable])` | Private; sort by `z_y`, blit in order |

### Z-Sort Rules

- Furniture: `z_y = (row + footprint_h) * TILE_SIZE`
- Surface items on desk: `z_y = max(sprite_bottom, desk_z_by_tile[pos] + 0.5)`
- Character: `z_y = pos.1 + TILE_SIZE / 2 + 0.5`
- Character (Type state): sprite shifted down `SITTING_OFFSET_PX` pixels
- Chair (back-facing): `z_y = (row + footprint_h) * TILE_SIZE + 1` (in front of seated character)
- Bubble: `z_y = character.z_y + 0.1`

### Character Rendering

- LEFT direction: use RIGHT sprite with `flipped = true` → `blit_flipped()`
- Type state: y position += `SITTING_OFFSET_PX` (visual sit-down offset)
- Selection outline: selected = 255 alpha, hovered = 128 alpha

---

## colorize.rs

Two colorization modes for tiles, furniture, and character hue shifts.

### Colorize Mode (Photoshop-style)

Grayscale input → fixed HSL output. Used for floor and fence tiles.

1. Perceived luminance: `L = (0.299*R + 0.587*G + 0.114*B) / 255`
2. Contrast: `L = 0.5 + (L - 0.5) * ((100 + c) / 100)`
3. Brightness: `L = L + b / 200`
4. Apply target hue and saturation from `TileColor`

### Adjust Mode (HSL shift)

Shifts original pixel HSL values. Used for furniture and character hue shifts.

- Hue: rotate ±180° with wrap-around modulo 360
- Saturation: shift ±100 (clamped 0.0..1.0)
- Brightness/Contrast: same formulas as Colorize

| Function | Signature | Description |
|----------|-----------|-------------|
| `rgb_to_hsl` | `(r, g, b) -> (f32, f32, f32)` | RGB → HSL |
| `hsl_to_rgb` | `(h, s, l) -> (u8, u8, u8)` | HSL → RGB |
| `colorize_sprite` | `(sprite: &SpriteData, color: &TileColor) -> SpriteData` | Colorize mode |
| `adjust_sprite` | `(sprite: &SpriteData, color: &TileColor) -> SpriteData` | Adjust mode |
| `adjust_hue` | `(pixel: Pixel, degrees: i16) -> Pixel` | Rotate hue only |

Alpha preserved from original pixel. Semi-transparent pixels (`alpha < 255`) retain alpha.

---

## bubble.rs

Speech bubble timer logic. `BubbleState` here is the render-side copy used by `update_bubble`; the engine owns the character's bubble. Its sprite functions duplicate `sprites::permission_bubble` and `sprites::waiting_bubble`; the engine and composer use the `sprites` versions.

| Function | Signature | Description |
|----------|-----------|-------------|
| `update_bubble` | `(bubble: &mut BubbleState, dt: f32) -> bool` | Tick timer, `true` when expired |

Waiting bubble: `WAITING_BUBBLE_SECS` display then `BUBBLE_FADE_SECS` fade-out.
Permission bubble: persistent until the pending tool completes, the user clicks it, or a new turn starts.
