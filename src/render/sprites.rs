use crate::types::{AnimType, BubbleKind, Direction, Pixel, SpriteData, TileType};

// ---------------------------------------------------------------------------
// Character palettes (6 distinct color schemes, index 0..5)
// ---------------------------------------------------------------------------

// Each palette: (skin, hair, shirt, pants, shoes)
const PALETTES: [(Pixel, Pixel, Pixel, Pixel, Pixel); 6] = [
    // 0: Blue shirt, brown hair
    (
        (235, 200, 160, 255),
        (100, 60, 30, 255),
        (60, 100, 180, 255),
        (50, 50, 70, 255),
        (40, 30, 20, 255),
    ),
    // 1: Red shirt, black hair
    (
        (220, 185, 150, 255),
        (30, 25, 20, 255),
        (180, 50, 50, 255),
        (40, 40, 60, 255),
        (30, 25, 20, 255),
    ),
    // 2: Green shirt, blonde hair
    (
        (240, 210, 170, 255),
        (210, 180, 80, 255),
        (60, 150, 80, 255),
        (60, 55, 75, 255),
        (50, 35, 25, 255),
    ),
    // 3: Purple shirt, red hair
    (
        (230, 195, 155, 255),
        (160, 50, 30, 255),
        (120, 60, 160, 255),
        (45, 45, 65, 255),
        (35, 28, 22, 255),
    ),
    // 4: Orange shirt, dark brown hair
    (
        (225, 190, 145, 255),
        (60, 40, 20, 255),
        (200, 120, 40, 255),
        (55, 50, 70, 255),
        (45, 32, 22, 255),
    ),
    // 5: Teal shirt, gray hair
    (
        (238, 205, 165, 255),
        (130, 130, 140, 255),
        (50, 150, 150, 255),
        (48, 48, 68, 255),
        (38, 30, 22, 255),
    ),
];

const T: Pixel = (0, 0, 0, 0); // transparent

/// Generate an 8x16 character sprite for the given palette, direction, animation, and frame.
///
/// Palette index is clamped to 0..5. LEFT direction is handled at render time
/// via horizontal flip of the RIGHT sprite.
pub fn character_sprite(
    palette: u8,
    direction: Direction,
    anim_type: AnimType,
    frame: u8,
) -> SpriteData {
    let idx = (palette % 6) as usize;
    let (skin, hair, shirt, pants, shoes) = PALETTES[idx];

    // Direction determines facing: Down = front, Up = back, Right = side
    // Left is rendered as flipped Right at the blit layer
    let dir = match direction {
        Direction::Left => Direction::Right,
        other => other,
    };

    let pal = CharPalette {
        skin,
        hair,
        shirt,
        pants,
        shoes,
    };
    match dir {
        Direction::Down => build_front(&pal, anim_type, frame),
        Direction::Up => build_back(&pal, anim_type, frame),
        Direction::Right | Direction::Left => build_side(&pal, anim_type, frame),
    }
}

struct CharPalette {
    skin: Pixel,
    hair: Pixel,
    shirt: Pixel,
    pants: Pixel,
    shoes: Pixel,
}

fn build_front(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let s = pal.skin;
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let f = pal.shoes;
    let eye = (40, 40, 50, 255);

    let mut rows: Vec<Vec<Pixel>> = vec![
        // Row 0: top of hair
        vec![T, T, h, h, h, h, T, T],
        // Row 1: hair sides
        vec![T, h, h, h, h, h, h, T],
        // Row 2: face with eyes
        vec![T, h, s, s, s, s, h, T],
        // Row 3: eyes
        vec![T, h, eye, s, s, eye, h, T],
        // Row 4: lower face
        vec![T, T, s, s, s, s, T, T],
        // Row 5: neck
        vec![T, T, T, s, s, T, T, T],
        // Row 6: shoulders
        vec![T, c, c, c, c, c, c, T],
        // Row 7: upper torso
        vec![T, c, c, c, c, c, c, T],
        // Row 8: mid torso
        vec![T, T, c, c, c, c, T, T],
        // Row 9: lower torso
        vec![T, T, c, c, c, c, T, T],
        // Row 10: upper legs
        vec![T, T, p, p, p, p, T, T],
        // Row 11: mid legs
        vec![T, T, p, p, p, p, T, T],
        // Row 12: lower legs
        vec![T, T, p, T, T, p, T, T],
        // Row 13: ankles
        vec![T, T, p, T, T, p, T, T],
        // Row 14: feet
        vec![T, T, f, T, T, f, T, T],
        // Row 15: feet bottom
        vec![T, f, f, T, T, f, f, T],
    ];

    apply_animation(&mut rows, anim_type, frame, pal.pants, pal.shoes);
    rows
}

fn build_back(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let s = pal.skin;
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let f = pal.shoes;

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, T, h, h, h, h, T, T],
        vec![T, h, h, h, h, h, h, T],
        vec![T, h, h, h, h, h, h, T],
        vec![T, h, h, h, h, h, h, T],
        vec![T, T, s, s, s, s, T, T],
        vec![T, T, T, s, s, T, T, T],
        vec![T, c, c, c, c, c, c, T],
        vec![T, c, c, c, c, c, c, T],
        vec![T, T, c, c, c, c, T, T],
        vec![T, T, c, c, c, c, T, T],
        vec![T, T, p, p, p, p, T, T],
        vec![T, T, p, p, p, p, T, T],
        vec![T, T, p, T, T, p, T, T],
        vec![T, T, p, T, T, p, T, T],
        vec![T, T, f, T, T, f, T, T],
        vec![T, f, f, T, T, f, f, T],
    ];

    apply_animation(&mut rows, anim_type, frame, pal.pants, pal.shoes);
    rows
}

fn build_side(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let s = pal.skin;
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let f = pal.shoes;
    let eye = (40, 40, 50, 255);

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, T, h, h, h, h, T, T],
        vec![T, h, h, h, h, h, T, T],
        vec![T, h, s, s, s, h, T, T],
        vec![T, h, s, eye, s, s, T, T],
        vec![T, T, s, s, s, T, T, T],
        vec![T, T, T, s, s, T, T, T],
        vec![T, T, c, c, c, c, T, T],
        vec![T, c, c, c, c, c, T, T],
        vec![T, T, c, c, c, T, T, T],
        vec![T, T, c, c, c, T, T, T],
        vec![T, T, p, p, p, T, T, T],
        vec![T, T, p, p, p, T, T, T],
        vec![T, T, T, p, T, T, T, T],
        vec![T, T, T, p, T, T, T, T],
        vec![T, T, T, f, T, T, T, T],
        vec![T, T, f, f, T, T, T, T],
    ];

    apply_animation(&mut rows, anim_type, frame, pal.pants, pal.shoes);
    rows
}

fn apply_animation(
    rows: &mut [Vec<Pixel>],
    anim_type: AnimType,
    frame: u8,
    pants: Pixel,
    shoes: Pixel,
) {
    match anim_type {
        AnimType::Walk => {
            // 4-frame walk cycle: alternate leg positions
            let phase = frame % 4;
            match phase {
                0 => {} // neutral
                1 => {
                    // Left leg forward
                    rows[12] = vec![T, T, pants, T, T, pants, T, T];
                    rows[13] = vec![T, pants, T, T, T, pants, T, T];
                    rows[14] = vec![T, shoes, T, T, T, shoes, T, T];
                    rows[15] = vec![shoes, shoes, T, T, T, shoes, shoes, T];
                }
                2 => {} // neutral (passing)
                _ => {
                    // Right leg forward
                    rows[12] = vec![T, T, pants, T, T, pants, T, T];
                    rows[13] = vec![T, T, pants, T, T, T, pants, T];
                    rows[14] = vec![T, T, shoes, T, T, T, shoes, T];
                    rows[15] = vec![T, shoes, shoes, T, T, shoes, shoes, T];
                }
            }
        }
        AnimType::Type => {
            // 2-frame typing: arms move
            if frame % 2 == 1 {
                rows[7] = vec![
                    T, rows[7][1], rows[7][2], rows[7][3], rows[7][4], rows[7][5], rows[7][6], T,
                ];
                // Slight arm shift for typing motion
                if rows[8].len() >= 7 {
                    rows[8][1] = rows[6][1];
                    rows[8][6] = rows[6][6];
                }
            }
        }
        AnimType::Read => {
            // 2-frame reading: subtle head bob
            if frame % 2 == 1 {
                // Shift head down slightly by making row 0 transparent
                rows[0] = vec![T; 8];
            }
        }
    }
}

/// Generate an 8x8 floor tile sprite for the given tile type.
///
/// Different grayscale patterns per floor variant.
pub fn floor_sprite(tile: TileType) -> SpriteData {
    let base: u8 = match tile {
        TileType::Floor1 => 180,
        TileType::Floor2 => 170,
        TileType::Floor3 => 160,
        TileType::Floor4 => 190,
        TileType::Floor5 => 175,
        TileType::Floor6 => 165,
        TileType::Floor7 => 185,
        TileType::Void | TileType::Wall => 100,
    };

    let light = base.saturating_add(10);
    let dark = base.saturating_sub(10);

    let a: Pixel = (base, base, base, 255);
    let b: Pixel = (light, light, light, 255);
    let c: Pixel = (dark, dark, dark, 255);

    // Subtle checkerboard-like pattern
    match tile {
        TileType::Floor1 | TileType::Floor4 | TileType::Floor7 => vec![
            vec![a, a, b, b, a, a, b, b],
            vec![a, a, b, b, a, a, b, b],
            vec![b, b, a, a, b, b, a, a],
            vec![b, b, a, a, b, b, a, a],
            vec![a, a, b, b, a, a, b, b],
            vec![a, a, b, b, a, a, b, b],
            vec![b, b, a, a, b, b, a, a],
            vec![b, b, a, a, b, b, a, a],
        ],
        TileType::Floor2 | TileType::Floor5 => vec![
            vec![a, b, a, b, a, b, a, b],
            vec![b, a, b, a, b, a, b, a],
            vec![a, b, a, b, a, b, a, b],
            vec![b, a, b, a, b, a, b, a],
            vec![a, b, a, b, a, b, a, b],
            vec![b, a, b, a, b, a, b, a],
            vec![a, b, a, b, a, b, a, b],
            vec![b, a, b, a, b, a, b, a],
        ],
        TileType::Floor3 | TileType::Floor6 => vec![
            vec![a, a, a, a, c, a, a, a],
            vec![a, a, a, a, a, a, a, c],
            vec![a, a, c, a, a, a, a, a],
            vec![a, a, a, a, a, c, a, a],
            vec![c, a, a, a, a, a, a, a],
            vec![a, a, a, c, a, a, a, a],
            vec![a, a, a, a, a, a, c, a],
            vec![a, c, a, a, a, a, a, a],
        ],
        TileType::Void | TileType::Wall => vec![vec![a; 8]; 8],
    }
}

/// Generate an 8x16 wall sprite with auto-tiling based on neighbor bitmask.
///
/// Neighbors: N=1, E=2, S=4, W=8. Adjusts top/bottom/left/right edges.
pub fn wall_sprite(neighbors: u8) -> SpriteData {
    let base: Pixel = (80, 80, 90, 255);
    let light: Pixel = (100, 100, 110, 255);
    let dark: Pixel = (55, 55, 65, 255);
    let edge: Pixel = (40, 40, 50, 255);

    let has_n = neighbors & 1 != 0;
    let has_e = neighbors & 2 != 0;
    let has_s = neighbors & 4 != 0;
    let has_w = neighbors & 8 != 0;

    let mut sprite = vec![vec![base; 8]; 16];

    // Top edge highlight (no north neighbor)
    if !has_n {
        sprite[0] = vec![light; 8];
        sprite[1] = vec![light; 8];
    }

    // Bottom edge shadow (no south neighbor)
    if !has_s {
        sprite[14] = vec![dark; 8];
        sprite[15] = vec![edge; 8];
    }

    // Left edge
    if !has_w {
        for row in &mut sprite {
            row[0] = edge;
        }
    }

    // Right edge
    if !has_e {
        for row in &mut sprite {
            row[7] = edge;
        }
    }

    // Brick pattern in mid section
    for (y, row) in sprite.iter_mut().enumerate().take(14).skip(3) {
        if y % 4 == 3 {
            *row = vec![dark; 8];
        } else if y % 4 == 1 {
            row[3] = dark;
            row[4] = dark;
        }
    }

    sprite
}

/// Generate a furniture sprite for the given kind.
///
/// Supported: "DESK_FRONT", "WOODEN_CHAIR_FRONT", "MONITOR", "LAMP".
/// Unknown kinds return a default placeholder sprite.
pub fn furniture_sprite(kind: &str) -> SpriteData {
    match kind {
        "DESK_FRONT" => desk_front_sprite(),
        "WOODEN_CHAIR_FRONT" => chair_front_sprite(),
        "MONITOR" => monitor_sprite(),
        "LAMP" => lamp_sprite(),
        _ => default_furniture_sprite(),
    }
}

fn desk_front_sprite() -> SpriteData {
    let wood: Pixel = (140, 100, 60, 255);
    let dark: Pixel = (100, 70, 40, 255);
    let top: Pixel = (160, 120, 75, 255);

    vec![
        vec![top, top, top, top, top, top, top, top],
        vec![wood, wood, wood, wood, wood, wood, wood, wood],
        vec![wood, dark, dark, dark, dark, dark, dark, wood],
        vec![wood, dark, dark, dark, dark, dark, dark, wood],
        vec![wood, dark, dark, dark, dark, dark, dark, wood],
        vec![wood, T, T, T, T, T, T, wood],
        vec![wood, T, T, T, T, T, T, wood],
        vec![dark, T, T, T, T, T, T, dark],
    ]
}

fn chair_front_sprite() -> SpriteData {
    let wood: Pixel = (120, 80, 45, 255);
    let seat: Pixel = (150, 105, 60, 255);
    let dark: Pixel = (90, 60, 35, 255);

    vec![
        vec![T, wood, T, T, T, T, wood, T],
        vec![T, wood, T, T, T, T, wood, T],
        vec![T, wood, seat, seat, seat, seat, wood, T],
        vec![T, T, seat, seat, seat, seat, T, T],
        vec![T, T, T, dark, dark, T, T, T],
        vec![T, T, T, dark, dark, T, T, T],
        vec![T, T, dark, T, T, dark, T, T],
        vec![T, dark, T, T, T, T, dark, T],
    ]
}

fn monitor_sprite() -> SpriteData {
    let frame: Pixel = (50, 50, 55, 255);
    let screen: Pixel = (60, 120, 180, 255);
    let stand: Pixel = (70, 70, 75, 255);

    vec![
        vec![frame, frame, frame, frame, frame, frame, frame, frame],
        vec![frame, screen, screen, screen, screen, screen, screen, frame],
        vec![frame, screen, screen, screen, screen, screen, screen, frame],
        vec![frame, screen, screen, screen, screen, screen, screen, frame],
        vec![frame, screen, screen, screen, screen, screen, screen, frame],
        vec![frame, frame, frame, frame, frame, frame, frame, frame],
        vec![T, T, T, stand, stand, T, T, T],
        vec![T, T, stand, stand, stand, stand, T, T],
    ]
}

fn lamp_sprite() -> SpriteData {
    let shade: Pixel = (200, 180, 120, 255);
    let glow: Pixel = (255, 240, 180, 200);
    let pole: Pixel = (100, 100, 105, 255);
    let base: Pixel = (80, 80, 85, 255);

    vec![
        vec![T, T, shade, shade, shade, shade, T, T],
        vec![T, shade, shade, glow, glow, shade, shade, T],
        vec![T, T, shade, shade, shade, shade, T, T],
        vec![T, T, T, pole, pole, T, T, T],
        vec![T, T, T, pole, pole, T, T, T],
        vec![T, T, T, pole, pole, T, T, T],
        vec![T, T, T, pole, pole, T, T, T],
        vec![T, T, base, base, base, base, T, T],
    ]
}

fn default_furniture_sprite() -> SpriteData {
    let c: Pixel = (120, 120, 130, 255);
    let d: Pixel = (90, 90, 100, 255);

    vec![
        vec![T, T, c, c, c, c, T, T],
        vec![T, c, c, c, c, c, c, T],
        vec![c, c, d, c, c, d, c, c],
        vec![c, c, c, c, c, c, c, c],
        vec![c, c, c, c, c, c, c, c],
        vec![T, c, c, c, c, c, c, T],
        vec![T, T, d, T, T, d, T, T],
        vec![T, T, d, T, T, d, T, T],
    ]
}

/// Generate a speech bubble sprite.
///
/// Returns a 16x6 bubble: amber "..." for Permission, green checkmark for Waiting.
pub fn status_bubble(kind: BubbleKind) -> SpriteData {
    match kind {
        BubbleKind::Permission => permission_bubble(),
        BubbleKind::Waiting => waiting_bubble(),
    }
}

/// Amber "..." permission bubble (16x6).
pub fn permission_bubble() -> SpriteData {
    let bg: Pixel = (255, 255, 255, 230);
    let border: Pixel = (180, 180, 180, 230);
    let dot: Pixel = (200, 150, 50, 255);
    let tail: Pixel = (255, 255, 255, 230);

    vec![
        vec![
            T, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, T,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, dot, bg, bg, dot, bg, bg, dot, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            T, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, T,
        ],
        vec![T, T, T, T, tail, tail, T, T, T, T, T, T, T, T, T, T],
    ]
}

/// Green checkmark waiting bubble (16x6).
pub fn waiting_bubble() -> SpriteData {
    let bg: Pixel = (255, 255, 255, 230);
    let border: Pixel = (180, 180, 180, 230);
    let check: Pixel = (50, 180, 80, 255);
    let tail: Pixel = (255, 255, 255, 230);

    vec![
        vec![
            T, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, T,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, check, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, check, bg, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, check, bg, check, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            T, border, border, border, border, border, check, border, border, border, border,
            border, border, border, border, T,
        ],
        vec![T, T, T, T, tail, tail, T, T, T, T, T, T, T, T, T, T],
    ]
}

/// Create a white outline around opaque pixels in the sprite.
///
/// Expands the sprite by 2px on all sides. Cardinal neighbors of opaque pixels
/// become white with the given alpha. Overlapping original pixels are cleared
/// so the outline appears only on the exterior.
pub fn character_outline(sprite: &SpriteData, alpha: u8) -> SpriteData {
    if sprite.is_empty() {
        return Vec::new();
    }

    let orig_h = sprite.len();
    let orig_w = sprite[0].len();
    let pad = 2;
    let new_h = orig_h + pad * 2;
    let new_w = orig_w + pad * 2;

    let white: Pixel = (255, 255, 255, alpha);

    // Start with a fully transparent expanded buffer
    let mut outline = vec![vec![(0u8, 0u8, 0u8, 0u8); new_w]; new_h];

    // Mark cardinal neighbors of each opaque pixel
    let offsets: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    for (sy, row) in sprite.iter().enumerate() {
        for (sx, &pixel) in row.iter().enumerate() {
            if pixel.3 == 0 {
                continue;
            }
            for &(dx, dy) in &offsets {
                let nx = (sx as i32 + pad as i32 + dx) as usize;
                let ny = (sy as i32 + pad as i32 + dy) as usize;
                if nx < new_w && ny < new_h {
                    outline[ny][nx] = white;
                }
            }
        }
    }

    // Clear pixels that overlap with the original sprite
    for (sy, row) in sprite.iter().enumerate() {
        for (sx, &pixel) in row.iter().enumerate() {
            if pixel.3 > 0 {
                outline[sy + pad][sx + pad] = T;
            }
        }
    }

    outline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_sprite_dimensions() {
        let sprite = character_sprite(0, Direction::Down, AnimType::Walk, 0);
        assert_eq!(sprite.len(), 16);
        assert!(sprite.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn floor_sprite_dimensions() {
        let sprite = floor_sprite(TileType::Floor1);
        assert_eq!(sprite.len(), 8);
        assert!(sprite.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn wall_sprite_dimensions() {
        let sprite = wall_sprite(0b0000);
        assert_eq!(sprite.len(), 16);
        assert!(sprite.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn furniture_sprite_unknown_returns_default() {
        let sprite = furniture_sprite("UNKNOWN_TYPE");
        assert_eq!(sprite.len(), 8);
        assert!(sprite.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn permission_bubble_dimensions() {
        let sprite = permission_bubble();
        assert_eq!(sprite.len(), 6);
        assert!(sprite.iter().all(|row| row.len() == 16));
    }

    #[test]
    fn character_outline_expands_by_2() {
        let sprite = vec![vec![(255, 0, 0, 255); 4]; 4];
        let outlined = character_outline(&sprite, 255);
        assert_eq!(outlined.len(), 8); // 4 + 2*2
        assert!(outlined.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn all_palettes_produce_valid_sprites() {
        for palette in 0..6 {
            let sprite = character_sprite(palette, Direction::Down, AnimType::Walk, 0);
            assert_eq!(sprite.len(), 16);
        }
    }

    #[test]
    fn walk_animation_has_four_frames() {
        for frame in 0..4 {
            let sprite = character_sprite(0, Direction::Down, AnimType::Walk, frame);
            assert_eq!(sprite.len(), 16);
        }
    }
}
