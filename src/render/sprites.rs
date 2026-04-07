use crate::types::{AnimType, BubbleKind, CompanionKind, Direction, Pixel, SpriteData, TileType};

// ---------------------------------------------------------------------------
// Dawnbringer 16-color palette (ThKaspar micro-tileset)
// ---------------------------------------------------------------------------

const GRASS_GREEN: Pixel = (109, 170, 44, 255);
const DARK_GREEN: Pixel = (52, 101, 36, 255);
const MID_GREEN: Pixel = (75, 136, 40, 255);
const DIRT_BROWN: Pixel = (133, 76, 48, 255);
const DARK_BROWN: Pixel = (68, 36, 52, 255);
const WOOD: Pixel = (210, 125, 44, 255);
const WHEAT: Pixel = (218, 212, 94, 255);
const WATER_BLUE: Pixel = (109, 194, 202, 255);
const WATER_DARK: Pixel = (89, 125, 206, 255);
const LIGHT: Pixel = (222, 238, 214, 255);
const STONE_GREY: Pixel = (78, 74, 78, 255);
const DARK: Pixel = (20, 12, 28, 255);
const WARM_RED: Pixel = (172, 50, 50, 255);
const SKIN_MED: Pixel = (210, 170, 130, 255);

// ---------------------------------------------------------------------------
// Character palettes (6 farm character variants)
// ---------------------------------------------------------------------------

// Each palette: (skin, hair, shirt, pants, shoes)
const PALETTES: [(Pixel, Pixel, Pixel, Pixel, Pixel); 6] = [
    // 0: Farmer — tan skin, brown hair, blue overalls
    (SKIN_MED, DIRT_BROWN, WATER_BLUE, WATER_DARK, DARK_BROWN),
    // 1: Rancher — lighter skin, black hair, red flannel
    (LIGHT, DARK, WARM_RED, DARK_BROWN, DARK),
    // 2: Gardener — tan skin, blonde/green bandana, green shirt
    (SKIN_MED, WHEAT, MID_GREEN, DARK_GREEN, DARK_BROWN),
    // 3: Berry picker — skin, purple hair ribbon, purple outfit
    (
        LIGHT,
        (140, 60, 140, 255),
        (120, 60, 160, 255),
        DARK_BROWN,
        DARK,
    ),
    // 4: Harvest worker — skin, dark hair, orange vest
    (SKIN_MED, DARK_BROWN, WOOD, DIRT_BROWN, DARK),
    // 5: Fisher — skin, grey hat, teal shirt
    (LIGHT, STONE_GREY, (50, 150, 150, 255), DARK_BROWN, DARK),
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
    let eye = DARK;

    let mut rows: Vec<Vec<Pixel>> = vec![
        // Row 0: hat brim / top of hair (wider for straw hat feel)
        vec![T, h, h, h, h, h, h, T],
        // Row 1: hair full
        vec![h, h, h, h, h, h, h, h],
        // Row 2: face with hair sides
        vec![T, h, s, s, s, s, h, T],
        // Row 3: eyes
        vec![T, h, eye, s, s, eye, h, T],
        // Row 4: lower face
        vec![T, T, s, s, s, s, T, T],
        // Row 5: neck
        vec![T, T, T, s, s, T, T, T],
        // Row 6: shoulders (overalls/work shirt)
        vec![T, c, c, c, c, c, c, T],
        // Row 7: upper torso
        vec![c, c, c, c, c, c, c, c],
        // Row 8: mid torso
        vec![T, c, c, c, c, c, c, T],
        // Row 9: lower torso / belt
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
        vec![T, h, h, h, h, h, h, T],
        vec![h, h, h, h, h, h, h, h],
        vec![T, h, h, h, h, h, h, T],
        vec![T, h, h, h, h, h, h, T],
        vec![T, T, s, s, s, s, T, T],
        vec![T, T, T, s, s, T, T, T],
        vec![T, c, c, c, c, c, c, T],
        vec![c, c, c, c, c, c, c, c],
        vec![T, c, c, c, c, c, c, T],
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
    let eye = DARK;

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, h, h, h, h, h, T, T],
        vec![h, h, h, h, h, h, T, T],
        vec![T, h, s, s, s, h, T, T],
        vec![T, h, s, eye, s, s, T, T],
        vec![T, T, s, s, s, T, T, T],
        vec![T, T, T, s, s, T, T, T],
        vec![T, T, c, c, c, c, T, T],
        vec![T, c, c, c, c, c, c, T],
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
                rows[0] = vec![T; 8];
            }
        }
    }
}

/// Generate an 8x8 floor tile sprite for the given tile type.
///
/// Uses Dawnbringer 16 palette for farm terrain.
pub fn floor_sprite(tile: TileType) -> SpriteData {
    match tile {
        TileType::Grass => {
            // Lush green with subtle variation
            let a = GRASS_GREEN;
            let b = MID_GREEN;
            vec![
                vec![a, a, b, a, a, b, a, a],
                vec![a, a, a, a, b, a, a, a],
                vec![b, a, a, a, a, a, b, a],
                vec![a, a, a, b, a, a, a, a],
                vec![a, b, a, a, a, a, a, b],
                vec![a, a, a, a, b, a, a, a],
                vec![a, a, b, a, a, a, a, a],
                vec![a, a, a, a, a, b, a, a],
            ]
        }
        TileType::GrassDark => {
            let a = MID_GREEN;
            let b = DARK_GREEN;
            vec![
                vec![a, a, b, a, a, a, b, a],
                vec![a, a, a, a, b, a, a, a],
                vec![b, a, a, a, a, a, a, b],
                vec![a, a, b, a, a, b, a, a],
                vec![a, a, a, a, a, a, b, a],
                vec![a, b, a, a, b, a, a, a],
                vec![a, a, a, b, a, a, a, a],
                vec![b, a, a, a, a, a, a, b],
            ]
        }
        TileType::Dirt => {
            // Tilled soil with horizontal furrow lines
            let a = DIRT_BROWN;
            let b = DARK_BROWN;
            vec![
                vec![a, a, a, a, a, a, a, a],
                vec![b, b, b, b, b, b, b, b],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![b, b, b, b, b, b, b, b],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![b, b, b, b, b, b, b, b],
            ]
        }
        TileType::DirtDark => {
            let a = DARK_BROWN;
            let b = DIRT_BROWN;
            vec![
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, b, a, a, b, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![a, b, a, a, b, a, a, b],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, b, a, a, b, a],
                vec![a, a, a, a, a, a, a, a],
                vec![a, b, a, a, a, b, a, a],
            ]
        }
        TileType::Water => {
            // Animated-looking wave pattern
            let a = WATER_BLUE;
            let b = WATER_DARK;
            vec![
                vec![a, a, b, a, a, a, b, a],
                vec![a, b, a, a, a, b, a, a],
                vec![b, a, a, a, b, a, a, a],
                vec![a, a, a, b, a, a, a, b],
                vec![a, a, b, a, a, a, b, a],
                vec![a, b, a, a, a, b, a, a],
                vec![b, a, a, a, b, a, a, a],
                vec![a, a, a, b, a, a, a, b],
            ]
        }
        TileType::Sand => {
            let a = WHEAT;
            let b = SKIN_MED;
            vec![
                vec![a, a, a, b, a, a, a, a],
                vec![a, a, a, a, a, a, b, a],
                vec![a, b, a, a, a, a, a, a],
                vec![a, a, a, a, b, a, a, a],
                vec![a, a, a, a, a, a, a, b],
                vec![a, a, b, a, a, a, a, a],
                vec![a, a, a, a, a, b, a, a],
                vec![b, a, a, a, a, a, a, a],
            ]
        }
        TileType::Stone => {
            let a = STONE_GREY;
            let b = DARK;
            let c = LIGHT;
            vec![
                vec![a, a, a, b, a, a, a, a],
                vec![a, c, a, a, a, c, a, a],
                vec![a, a, a, a, a, a, a, b],
                vec![b, a, a, a, b, a, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, b, a, a, a, b, a],
                vec![a, a, a, a, c, a, a, a],
                vec![a, a, a, a, a, a, a, a],
            ]
        }
        TileType::Fence => vec![vec![DARK_BROWN; 8]; 8],
        TileType::Void => vec![vec![DARK; 8]; 8],
    }
}

/// Generate an 8x16 fence sprite with auto-tiling based on neighbor bitmask.
///
/// Neighbors: N=1, E=2, S=4, W=8. Adjusts post/rail connections.
pub fn fence_sprite(neighbors: u8) -> SpriteData {
    let post = WOOD;
    let rail = DARK_BROWN;
    let cap = WHEAT;

    let has_n = neighbors & 1 != 0;
    let has_e = neighbors & 2 != 0;
    let has_s = neighbors & 4 != 0;
    let has_w = neighbors & 8 != 0;

    let mut sprite = vec![vec![T; 8]; 16];

    // Fence post in center columns (3,4)
    for row in &mut sprite {
        row[3] = post;
        row[4] = post;
    }

    // Top cap when no north neighbor
    if !has_n {
        sprite[0][3] = cap;
        sprite[0][4] = cap;
        sprite[1][3] = cap;
        sprite[1][4] = cap;
    }

    // Bottom anchor when no south neighbor
    if !has_s {
        sprite[14][3] = rail;
        sprite[14][4] = rail;
        sprite[15][3] = rail;
        sprite[15][4] = rail;
    }

    // Horizontal rail west
    if has_w {
        for &r in &[4usize, 5, 10, 11] {
            for pixel in sprite[r].iter_mut().take(3) {
                *pixel = rail;
            }
        }
    }

    // Horizontal rail east
    if has_e {
        for &r in &[4usize, 5, 10, 11] {
            for pixel in sprite[r].iter_mut().skip(5) {
                *pixel = rail;
            }
        }
    }

    // Vertical grain detail on post
    sprite[6][3] = rail;
    sprite[8][4] = rail;
    sprite[12][3] = rail;

    sprite
}

/// Generate a furniture sprite for the given kind.
///
/// Farm furniture types: crop plots, trees, stumps, wells, scarecrows, etc.
/// Unknown kinds return a default placeholder sprite.
pub fn furniture_sprite(kind: &str) -> SpriteData {
    match kind {
        "CROP_PLOT" => crop_plot_sprite(false),
        "CROP_PLOT_ON" => crop_plot_sprite(true),
        "STUMP_FRONT" => stump_front_sprite(),
        "TREE" => tree_sprite(false),
        "TREE_FRUIT" => tree_sprite(true),
        "WELL" => well_sprite(),
        "MAILBOX" => mailbox_sprite(false),
        "MAILBOX_ON" => mailbox_sprite(true),
        "SCARECROW" => scarecrow_sprite(),
        "LANTERN" => lantern_sprite(),
        "CABIN_WALL" => cabin_wall_sprite(),
        "FENCE_H" => fence_h_sprite(),
        "FENCE_V" => fence_v_sprite(),
        "FISHING_SPOT" => fishing_spot_sprite(),
        _ => default_furniture_sprite(),
    }
}

fn crop_plot_sprite(active: bool) -> SpriteData {
    let d = DIRT_BROWN;
    let b = DARK_BROWN;
    let g = GRASS_GREEN;
    let w = WHEAT;

    if active {
        // Taller crops / wheat growing
        vec![
            vec![T, T, g, T, w, T, g, T],
            vec![T, g, w, g, g, w, w, T],
            vec![T, g, g, w, g, g, g, T],
            vec![T, T, g, g, g, g, T, T],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
        ]
    } else {
        // Small green sprouts on dirt
        vec![
            vec![T, T, T, T, T, T, T, T],
            vec![T, T, T, T, T, T, T, T],
            vec![T, T, g, T, T, g, T, T],
            vec![T, T, T, T, T, T, T, T],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
        ]
    }
}

fn stump_front_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let r = DIRT_BROWN; // ring detail

    vec![
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, b, b, b, b, T, T],
        vec![T, b, w, r, w, r, b, T],
        vec![T, b, r, w, r, w, b, T],
        vec![T, b, w, r, w, r, b, T],
        vec![T, b, b, b, b, b, b, T],
        vec![T, T, b, b, b, b, T, T],
        vec![T, T, T, T, T, T, T, T],
    ]
}

fn tree_sprite(has_fruit: bool) -> SpriteData {
    let g = GRASS_GREEN;
    let d = DARK_GREEN;
    let w = WOOD;
    let b = DARK_BROWN;
    let fruit = WARM_RED;

    let f1 = if has_fruit { fruit } else { g };
    let f2 = if has_fruit { fruit } else { d };

    vec![
        vec![T, T, d, g, g, d, T, T],
        vec![T, d, g, f1, g, g, d, T],
        vec![d, g, g, g, f2, g, g, d],
        vec![d, g, f2, g, g, g, g, d],
        vec![T, d, g, g, g, f1, d, T],
        vec![T, T, d, d, d, d, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, b, b, T, T, T],
    ]
}

fn well_sprite() -> SpriteData {
    let s = STONE_GREY;
    let d = DARK;
    let w = WOOD;

    vec![
        vec![T, w, w, w, w, w, w, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, s, s, d, d, s, s, T],
        vec![s, s, d, d, d, d, s, s],
        vec![s, s, d, d, d, d, s, s],
        vec![T, s, s, d, d, s, s, T],
        vec![T, s, s, s, s, s, s, T],
        vec![T, T, s, s, s, s, T, T],
    ]
}

fn mailbox_sprite(has_letter: bool) -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let l = LIGHT;

    let letter_pixel = if has_letter { l } else { b };

    vec![
        vec![T, T, b, b, b, b, T, T],
        vec![T, T, b, letter_pixel, letter_pixel, b, T, T],
        vec![T, T, b, b, b, b, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, b, b, T, T, T],
    ]
}

fn scarecrow_sprite() -> SpriteData {
    let w = WOOD;
    let h = WHEAT;
    let b = DARK_BROWN;

    vec![
        vec![T, T, h, h, h, h, T, T],
        vec![T, T, h, b, b, h, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![w, w, w, w, w, w, w, w],
        vec![T, h, T, w, w, T, h, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, w, T, T, w, T, T],
    ]
}

fn lantern_sprite() -> SpriteData {
    let w = WOOD;
    let g = WHEAT; // warm glow
    let b = DARK_BROWN;

    vec![
        vec![T, T, T, b, b, T, T, T],
        vec![T, T, b, g, g, b, T, T],
        vec![T, T, b, g, g, b, T, T],
        vec![T, T, T, b, b, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, T, w, w, T, T, T],
        vec![T, T, b, b, b, b, T, T],
    ]
}

fn cabin_wall_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;

    vec![
        vec![w, w, w, w, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![w, w, w, w, w, w, w, w],
        vec![w, w, w, w, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![w, w, w, w, w, w, w, w],
        vec![w, w, w, w, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
    ]
}

fn fence_h_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;

    vec![
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, T, T, T, T, T, T],
        vec![w, w, w, w, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![T, T, T, T, T, T, T, T],
        vec![w, w, w, w, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![T, T, T, T, T, T, T, T],
    ]
}

fn fence_v_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;

    vec![
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
        vec![T, T, T, w, b, T, T, T],
    ]
}

fn fishing_spot_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let wa = WATER_BLUE;
    let wd = WATER_DARK;

    vec![
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, T, T, T, T, T, T],
        vec![w, w, w, w, w, b, T, T],
        vec![b, b, b, b, b, b, T, T],
        vec![w, w, w, w, w, b, T, T],
        vec![T, T, T, T, T, T, T, T],
        vec![wa, wa, wd, wa, wa, wd, wa, wa],
        vec![wd, wa, wa, wd, wa, wa, wd, wa],
    ]
}

fn default_furniture_sprite() -> SpriteData {
    let c = STONE_GREY;
    let d = DARK;

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

/// Generate an 8x8 companion animal sprite.
///
/// Two-frame idle animation: frame 0 = normal, frame 1 = slight bob.
pub fn companion_sprite(kind: CompanionKind, frame: u8) -> SpriteData {
    match kind {
        CompanionKind::Chicken => chicken_sprite(frame),
        CompanionKind::Cat => cat_sprite(frame),
        CompanionKind::Dog => dog_sprite(frame),
    }
}

fn chicken_sprite(frame: u8) -> SpriteData {
    let body: Pixel = (218, 212, 94, 255); // wheat/yellow
    let wing: Pixel = (210, 170, 130, 255); // tan
    let beak: Pixel = (210, 125, 44, 255); // orange
    let eye: Pixel = (20, 12, 28, 255); // dark
    let comb: Pixel = (172, 50, 50, 255); // red
    let feet: Pixel = (210, 125, 44, 255); // orange

    if frame.is_multiple_of(2) {
        vec![
            vec![T, T, T, comb, T, T, T, T],
            vec![T, T, body, body, body, T, T, T],
            vec![T, T, eye, body, body, T, T, T],
            vec![T, beak, body, body, body, T, T, T],
            vec![T, T, body, body, body, wing, T, T],
            vec![T, T, body, body, body, T, T, T],
            vec![T, T, T, feet, feet, T, T, T],
            vec![T, T, feet, T, T, feet, T, T],
        ]
    } else {
        // Bob up: shift body 1px up
        vec![
            vec![T, T, body, comb, body, T, T, T],
            vec![T, T, eye, body, body, T, T, T],
            vec![T, beak, body, body, body, T, T, T],
            vec![T, T, body, body, body, wing, T, T],
            vec![T, T, body, body, body, T, T, T],
            vec![T, T, T, feet, feet, T, T, T],
            vec![T, T, T, feet, feet, T, T, T],
            vec![T, T, T, T, T, T, T, T],
        ]
    }
}

fn cat_sprite(frame: u8) -> SpriteData {
    let body: Pixel = (130, 130, 140, 255); // grey
    let dark: Pixel = (78, 74, 78, 255); // dark grey
    let eye: Pixel = (109, 170, 44, 255); // green eyes
    let nose: Pixel = (172, 50, 50, 255); // pink nose
    let ear: Pixel = (130, 130, 140, 255);

    if frame.is_multiple_of(2) {
        vec![
            vec![T, ear, T, T, T, ear, T, T],
            vec![T, body, body, body, body, body, T, T],
            vec![T, body, eye, body, eye, body, T, T],
            vec![T, body, body, nose, body, body, T, T],
            vec![T, T, body, body, body, body, T, T],
            vec![T, T, body, body, body, body, dark, T],
            vec![T, T, dark, T, T, dark, T, T],
            vec![T, T, dark, T, T, dark, T, T],
        ]
    } else {
        // Tail wag
        vec![
            vec![T, ear, T, T, T, ear, T, T],
            vec![T, body, body, body, body, body, T, T],
            vec![T, body, eye, body, eye, body, T, T],
            vec![T, body, body, nose, body, body, T, T],
            vec![T, T, body, body, body, body, T, T],
            vec![T, T, body, body, body, body, T, dark],
            vec![T, T, dark, T, T, dark, T, T],
            vec![T, T, dark, T, T, dark, T, T],
        ]
    }
}

fn dog_sprite(frame: u8) -> SpriteData {
    let body: Pixel = (133, 76, 48, 255); // brown
    let light: Pixel = (210, 170, 130, 255); // tan belly
    let eye: Pixel = (20, 12, 28, 255); // dark
    let nose: Pixel = (20, 12, 28, 255);
    let ear: Pixel = (68, 36, 52, 255); // dark brown ears
    let tongue: Pixel = (172, 50, 50, 255); // red

    if frame.is_multiple_of(2) {
        vec![
            vec![T, ear, body, body, body, ear, T, T],
            vec![T, body, body, body, body, body, T, T],
            vec![T, body, eye, body, eye, body, T, T],
            vec![T, body, body, nose, body, body, T, T],
            vec![T, T, body, body, tongue, T, T, T],
            vec![T, T, body, light, body, body, T, T],
            vec![T, T, body, T, T, body, T, T],
            vec![T, T, body, T, T, body, T, T],
        ]
    } else {
        // Pant: tongue out more
        vec![
            vec![T, ear, body, body, body, ear, T, T],
            vec![T, body, body, body, body, body, T, T],
            vec![T, body, eye, body, eye, body, T, T],
            vec![T, body, body, nose, body, body, T, T],
            vec![T, T, body, body, tongue, tongue, T, T],
            vec![T, T, body, light, body, body, T, T],
            vec![T, T, body, T, T, body, T, T],
            vec![T, T, body, T, T, body, T, T],
        ]
    }
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
        let sprite = floor_sprite(TileType::Grass);
        assert_eq!(sprite.len(), 8);
        assert!(sprite.iter().all(|row| row.len() == 8));
    }

    #[test]
    fn fence_sprite_dimensions() {
        let sprite = fence_sprite(0b0000);
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

    #[test]
    fn all_floor_types_produce_8x8() {
        let tiles = [
            TileType::Grass,
            TileType::GrassDark,
            TileType::Dirt,
            TileType::DirtDark,
            TileType::Water,
            TileType::Sand,
            TileType::Stone,
            TileType::Fence,
            TileType::Void,
        ];
        for tile in tiles {
            let sprite = floor_sprite(tile);
            assert_eq!(sprite.len(), 8);
            assert!(sprite.iter().all(|row| row.len() == 8));
        }
    }

    #[test]
    fn all_farm_furniture_produce_8x8() {
        let kinds = [
            "CROP_PLOT",
            "CROP_PLOT_ON",
            "STUMP_FRONT",
            "TREE",
            "TREE_FRUIT",
            "WELL",
            "MAILBOX",
            "MAILBOX_ON",
            "SCARECROW",
            "LANTERN",
            "CABIN_WALL",
            "FENCE_H",
            "FENCE_V",
            "FISHING_SPOT",
        ];
        for kind in kinds {
            let sprite = furniture_sprite(kind);
            assert_eq!(sprite.len(), 8, "furniture {kind} wrong height");
            assert!(
                sprite.iter().all(|row| row.len() == 8),
                "furniture {kind} wrong width"
            );
        }
    }
}
