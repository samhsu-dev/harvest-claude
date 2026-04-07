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
| `wall_sprite` | `(neighbors: u8) -> SpriteData` | 8x16 wall, auto-tile bitmask |
| `furniture_sprite` | `(kind: &str) -> SpriteData` | 8x8 furniture item |
| `status_bubble` | `(kind: BubbleKind) -> SpriteData` | 16x6 speech bubble |
| `character_outline` | `(sprite: &SpriteData, alpha: u8) -> SpriteData` | White outline around opaque pixels |

### Wall Auto-Tile Bitmask

Built at render time from `tile_map` neighbors: N=1, E=2, S=4, W=8. 4-bit index into 16 sprite variants. Algorithm:
```
mask = 0
if north is Wall → mask |= 1
if east is Wall  → mask |= 2
if south is Wall → mask |= 4
if west is Wall  → mask |= 8
```

### Outline Algorithm

1. Expand sprite by 2px on all sides (transparent border)
2. For each opaque pixel in original, mark 4 cardinal neighbors as white
3. Clear pixels that overlap original sprite (outline only on exterior)
4. Selected character: alpha 255. Hovered character: alpha 128.

---

## composer.rs

Z-sorted scene composition onto PixelBuffer.

```rust
struct Drawable {
    sprite: SpriteData,
    x: i16,
    y: i16,
    z_y: f32,
    flipped: bool,
}
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `compose_scene` | `(buf: &mut PixelBuffer, state: &OfficeState, selected: Option<usize>)` | Full scene render |
| `render_tiles` | `(buf: &mut PixelBuffer, state: &OfficeState)` | Floor + wall base layer |
| `collect_drawables` | `(state: &OfficeState, selected: Option<usize>) -> Vec<Drawable>` | Gather all entities with z_y |
| `render_sorted` | `(buf: &mut PixelBuffer, drawables: &mut [Drawable])` | Sort by z_y, blit in order |

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

Grayscale input → fixed HSL output. Used for floor and wall tiles.

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

Speech bubble sprites and timer logic.

| Function | Signature | Description |
|----------|-----------|-------------|
| `permission_bubble` | `() -> SpriteData` | Amber "..." bubble |
| `waiting_bubble` | `() -> SpriteData` | Green checkmark bubble |
| `update_bubble` | `(bubble: &mut BubbleState, dt: f32) -> bool` | Tick timer, `true` when expired |

Waiting bubble: 2s display + 0.5s fade-out (`BUBBLE_FADE_DURATION_SEC`).
Permission bubble: persistent until dismissed by click or new user prompt.
