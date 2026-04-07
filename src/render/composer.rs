// TODO: The engine module (state, character, seat) is being built in parallel.
// Once available, replace the render data types below with direct imports:
//   use crate::engine::state::OfficeState;
//   use crate::engine::character::{Character, BubbleState};
//   use crate::engine::seat::SeatAssignment;

use crate::constants::{SITTING_OFFSET_PX, TILE_SIZE};
use crate::render::buffer::PixelBuffer;
use crate::render::colorize::{adjust_sprite, colorize_sprite};
use crate::render::sprites::{
    character_outline, character_sprite, companion_sprite, fence_sprite, floor_sprite,
    furniture_sprite, status_bubble,
};
use crate::types::{
    AnimType, BubbleKind, CompanionKind, Direction, SpriteData, TileColor, TilePos, TileType,
};

/// A sprite positioned in the scene with z-sort ordering.
#[derive(Debug, Clone)]
pub(crate) struct Drawable {
    /// Rendered sprite data.
    pub sprite: SpriteData,
    /// Screen x position (signed for off-screen).
    pub x: i16,
    /// Screen y position (signed for off-screen).
    pub y: i16,
    /// Z-sort key: higher values render later (in front).
    pub z_y: f32,
    /// Horizontally flip when blitting (LEFT direction).
    pub flipped: bool,
}

/// Input data for scene composition, grouping grid and entity state.
#[derive(Debug, Clone)]
pub struct SceneInput<'a> {
    /// Flat tile map (row-major).
    pub tile_map: &'a [TileType],
    /// Grid column count.
    pub cols: u16,
    /// Grid row count.
    pub rows: u16,
    /// Placed furniture items.
    pub furniture: &'a [FurnitureRender],
    /// Character render data.
    pub characters: &'a [CharacterRender],
    /// Per-tile-position color overrides.
    pub tile_colors: &'a [(TilePos, TileColor)],
    /// Index of selected character (outline alpha 255).
    pub selected: Option<usize>,
}

/// Compose the full scene onto the pixel buffer.
///
/// Renders floor tiles as background, then z-sorts all furniture, characters,
/// bubbles, and overlays.
pub fn compose_scene(buf: &mut PixelBuffer, input: &SceneInput<'_>) {
    buf.clear((0, 0, 0, 255));
    render_tiles(
        buf,
        input.tile_map,
        input.cols,
        input.rows,
        input.tile_colors,
    );

    let mut drawables = collect_drawables(input.furniture, input.characters, input.selected);
    render_sorted(buf, &mut drawables);
}

/// Render floor and wall tiles as the base layer.
///
/// Iterates the tile map and blits floor or wall sprites at grid positions.
/// Walls use auto-tile neighbor detection for edge selection.
pub fn render_tiles(
    buf: &mut PixelBuffer,
    tile_map: &[TileType],
    cols: u16,
    rows: u16,
    tile_colors: &[(TilePos, TileColor)],
) {
    for row in 0..rows {
        for col in 0..cols {
            let idx = row as usize * cols as usize + col as usize;
            if idx >= tile_map.len() {
                continue;
            }
            let tile = tile_map[idx];
            let px = col as i16 * TILE_SIZE as i16;
            let py = row as i16 * TILE_SIZE as i16;

            match tile {
                TileType::Void => {}
                TileType::Fence => {
                    let neighbors = fence_neighbors(tile_map, cols, rows, col, row);
                    let sprite = fence_sprite(neighbors);
                    let sprite = apply_tile_color(sprite, (col, row), tile_colors);
                    buf.blit(&sprite, px, py);
                }
                floor_tile if floor_tile.is_floor() => {
                    let sprite = floor_sprite(floor_tile);
                    let sprite = apply_tile_color(sprite, (col, row), tile_colors);
                    buf.blit(&sprite, px, py);
                }
                _ => {}
            }
        }
    }
}

/// Gather all entities into Drawable structs with z_y sort keys.
///
/// Z-sort rules:
/// - Furniture: z_y = (row + footprint_h) * TILE_SIZE
/// - Character: z_y = pixel_y + TILE_SIZE/2 + 0.5
/// - Character (Type state): y shifted down by SITTING_OFFSET_PX
/// - Bubble: z_y = character.z_y + 0.1
pub(crate) fn collect_drawables(
    furniture: &[FurnitureRender],
    characters: &[CharacterRender],
    selected: Option<usize>,
) -> Vec<Drawable> {
    let mut drawables = Vec::with_capacity(furniture.len() + characters.len() * 2);

    // Furniture drawables
    for item in furniture {
        let sprite = furniture_sprite(&item.kind);
        let sprite = if let Some(ref color) = item.color {
            adjust_sprite(&sprite, color)
        } else {
            sprite
        };

        // Seats render behind characters at the same row
        let z_y = if item.is_seat {
            item.row as f32 * TILE_SIZE as f32
        } else {
            (item.row as f32 + 1.0) * TILE_SIZE as f32
        };

        drawables.push(Drawable {
            sprite,
            x: item.col * TILE_SIZE as i16,
            y: item.row * TILE_SIZE as i16,
            z_y,
            flipped: false,
        });
    }

    // Character drawables
    for (idx, ch) in characters.iter().enumerate() {
        let (sprite_dir, flipped) = match ch.direction {
            Direction::Left => (Direction::Right, true),
            other => (other, false),
        };

        let sprite = character_sprite(ch.palette, sprite_dir, ch.anim_type, ch.frame);

        // Type state: shift down by sitting offset
        let y_offset = if ch.anim_type == AnimType::Type {
            SITTING_OFFSET_PX as i16
        } else {
            0
        };

        let px = ch.pixel_x;
        let py = ch.pixel_y + y_offset;
        let z_y = ch.pixel_y as f32 + TILE_SIZE as f32 / 2.0 + 0.5;

        // Selection outline (rendered just behind the character)
        if selected == Some(idx) {
            let outline = character_outline(&sprite, 255);
            drawables.push(Drawable {
                sprite: outline,
                x: px - 2, // outline expanded by 2px
                y: py - 2,
                z_y: z_y - 0.01,
                flipped,
            });
        }

        drawables.push(Drawable {
            sprite,
            x: px,
            y: py,
            z_y,
            flipped,
        });

        // Companion animal drawables (beside character)
        for comp in &ch.companions {
            let comp_sprite = companion_sprite(comp.kind, comp.frame);
            let comp_x = px + comp.offset_x as i16;
            let comp_y = py + 8 + comp.offset_y as i16; // below character's head
            drawables.push(Drawable {
                sprite: comp_sprite,
                x: comp_x,
                y: comp_y,
                z_y: z_y + 0.05,
                flipped: false,
            });
        }

        // Bubble drawable (above character head)
        if let Some(ref bubble) = ch.bubble {
            let bubble_sprite = status_bubble(bubble.kind);
            let bubble_x = px - 4; // center 16-wide bubble over 8-wide character
            let bubble_y = py - 8;
            drawables.push(Drawable {
                sprite: bubble_sprite,
                x: bubble_x,
                y: bubble_y,
                z_y: z_y + 0.1,
                flipped: false,
            });
        }
    }

    drawables
}

/// Sort drawables by z_y and blit in order (back to front).
pub(crate) fn render_sorted(buf: &mut PixelBuffer, drawables: &mut [Drawable]) {
    drawables.sort_by(|a, b| {
        a.z_y
            .partial_cmp(&b.z_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for drawable in drawables.iter() {
        if drawable.flipped {
            buf.blit_flipped(&drawable.sprite, drawable.x, drawable.y);
        } else {
            buf.blit(&drawable.sprite, drawable.x, drawable.y);
        }
    }
}

// ---------------------------------------------------------------------------
// Render data types (decoupled from engine state)
// ---------------------------------------------------------------------------
// These serve as the render module's input contract. The engine layer converts
// its Character / OfficeState into these structs before calling compose_scene.

/// Furniture render data extracted from engine state.
#[derive(Debug, Clone)]
pub struct FurnitureRender {
    /// Furniture type key (e.g. "DESK_FRONT", "MONITOR").
    pub kind: String,
    /// Grid column.
    pub col: i16,
    /// Grid row.
    pub row: i16,
    /// Optional HSL color adjustment.
    pub color: Option<TileColor>,
    /// Seat furniture renders behind characters at the same row.
    pub is_seat: bool,
}

/// Bubble render data.
#[derive(Debug, Clone)]
pub struct BubbleRender {
    /// Bubble kind (Permission or Waiting).
    pub kind: BubbleKind,
    /// Remaining display time in seconds.
    pub timer: f32,
}

/// Companion animal render data.
#[derive(Debug, Clone)]
pub struct CompanionRender {
    /// Animal kind.
    pub kind: CompanionKind,
    /// Pixel offset from parent character.
    pub offset_x: f32,
    /// Pixel offset from parent character.
    pub offset_y: f32,
    /// Animation frame (0 or 1).
    pub frame: u8,
}

/// Character render data extracted from engine state.
#[derive(Debug, Clone)]
pub struct CharacterRender {
    /// Palette index (0..5).
    pub palette: u8,
    /// Facing direction.
    pub direction: Direction,
    /// Current animation type.
    pub anim_type: AnimType,
    /// Current animation frame.
    pub frame: u8,
    /// Pixel x position on the buffer.
    pub pixel_x: i16,
    /// Pixel y position on the buffer.
    pub pixel_y: i16,
    /// Active speech bubble, if any.
    pub bubble: Option<BubbleRender>,
    /// Companion animals following this character.
    pub companions: Vec<CompanionRender>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn fence_neighbors(tile_map: &[TileType], cols: u16, rows: u16, col: u16, row: u16) -> u8 {
    let mut mask = 0u8;
    let get = |c: u16, r: u16| -> Option<TileType> {
        if c < cols && r < rows {
            tile_map
                .get(r as usize * cols as usize + c as usize)
                .copied()
        } else {
            None
        }
    };

    if row > 0 && get(col, row - 1) == Some(TileType::Fence) {
        mask |= 1;
    }
    if get(col + 1, row) == Some(TileType::Fence) {
        mask |= 2;
    }
    if get(col, row + 1) == Some(TileType::Fence) {
        mask |= 4;
    }
    if col > 0 && get(col - 1, row) == Some(TileType::Fence) {
        mask |= 8;
    }

    mask
}

fn apply_tile_color(
    sprite: SpriteData,
    pos: TilePos,
    tile_colors: &[(TilePos, TileColor)],
) -> SpriteData {
    tile_colors
        .iter()
        .find(|(p, _)| *p == pos)
        .map(|(_, color)| colorize_sprite(&sprite, color))
        .unwrap_or(sprite)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawable_z_sort_order() {
        let mut drawables = vec![
            Drawable {
                sprite: vec![vec![(255, 0, 0, 255)]],
                x: 0,
                y: 0,
                z_y: 10.0,
                flipped: false,
            },
            Drawable {
                sprite: vec![vec![(0, 255, 0, 255)]],
                x: 0,
                y: 0,
                z_y: 5.0,
                flipped: false,
            },
            Drawable {
                sprite: vec![vec![(0, 0, 255, 255)]],
                x: 0,
                y: 0,
                z_y: 15.0,
                flipped: false,
            },
        ];

        drawables.sort_by(|a, b| a.z_y.partial_cmp(&b.z_y).unwrap());
        assert_eq!(drawables[0].z_y, 5.0);
        assert_eq!(drawables[1].z_y, 10.0);
        assert_eq!(drawables[2].z_y, 15.0);
    }

    #[test]
    fn fence_neighbors_isolated() {
        let map = vec![TileType::Fence];
        let mask = fence_neighbors(&map, 1, 1, 0, 0);
        assert_eq!(mask, 0);
    }

    #[test]
    fn fence_neighbors_surrounded() {
        let map = vec![TileType::Fence; 9];
        let mask = fence_neighbors(&map, 3, 3, 1, 1);
        assert_eq!(mask, 0b1111);
    }

    #[test]
    fn compose_scene_runs_without_panic() {
        let mut buf = PixelBuffer::new(32, 32);
        let tiles = vec![TileType::Grass; 16];
        let input = SceneInput {
            tile_map: &tiles,
            cols: 4,
            rows: 4,
            furniture: &[],
            characters: &[],
            tile_colors: &[],
            selected: None,
        };
        compose_scene(&mut buf, &input);
    }

    #[test]
    fn collect_drawables_with_selected_adds_outline() {
        let chars = vec![CharacterRender {
            palette: 0,
            direction: Direction::Down,
            anim_type: AnimType::Walk,
            frame: 0,
            pixel_x: 8,
            pixel_y: 8,
            bubble: None,
            companions: vec![],
        }];

        let drawables = collect_drawables(&[], &chars, Some(0));
        assert_eq!(drawables.len(), 2); // character + outline
    }

    #[test]
    fn collect_drawables_with_bubble() {
        let chars = vec![CharacterRender {
            palette: 0,
            direction: Direction::Down,
            anim_type: AnimType::Walk,
            frame: 0,
            pixel_x: 8,
            pixel_y: 8,
            bubble: Some(BubbleRender {
                kind: BubbleKind::Permission,
                timer: 1.0,
            }),
            companions: vec![],
        }];

        let drawables = collect_drawables(&[], &chars, None);
        assert_eq!(drawables.len(), 2); // character + bubble
    }

    #[test]
    fn left_direction_produces_flipped_drawable() {
        let chars = vec![CharacterRender {
            palette: 0,
            direction: Direction::Left,
            anim_type: AnimType::Walk,
            frame: 0,
            pixel_x: 8,
            pixel_y: 8,
            bubble: None,
            companions: vec![],
        }];

        let drawables = collect_drawables(&[], &chars, None);
        assert!(drawables[0].flipped);
    }
}
