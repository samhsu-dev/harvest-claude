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

struct CharPalette {
    skin: Pixel,
    hair: Pixel,
    shirt: Pixel,
    pants: Pixel,
    accent: Pixel,
}

const PALETTES: [CharPalette; 6] = [
    // 0: Farmer — bright blue overalls, yellow hat (high contrast on green)
    CharPalette {
        skin: LIGHT,
        hair: WHEAT,
        shirt: (60, 100, 220, 255), // strong blue
        pants: (40, 70, 170, 255),
        accent: WHEAT,
    },
    // 1: Rancher — bold red shirt, dark hat
    CharPalette {
        skin: SKIN_MED,
        hair: DARK_BROWN,
        shirt: (200, 50, 50, 255), // bright red
        pants: DARK_BROWN,
        accent: (220, 80, 60, 255),
    },
    // 2: Gardener — white shirt (stands out on green grass)
    CharPalette {
        skin: LIGHT,
        hair: DIRT_BROWN,
        shirt: LIGHT,
        pants: DARK_GREEN,
        accent: WHEAT,
    },
    // 3: Berry picker — vivid purple outfit
    CharPalette {
        skin: SKIN_MED,
        hair: DARK_BROWN,
        shirt: (140, 50, 200, 255), // vivid purple
        pants: (90, 30, 140, 255),
        accent: (180, 80, 220, 255),
    },
    // 4: Harvest worker — bright orange vest
    CharPalette {
        skin: LIGHT,
        hair: DARK_BROWN,
        shirt: (230, 140, 30, 255), // bright orange
        pants: DIRT_BROWN,
        accent: WHEAT,
    },
    // 5: Fisher — teal/cyan outfit
    CharPalette {
        skin: SKIN_MED,
        hair: STONE_GREY,
        shirt: (30, 180, 180, 255), // bright teal
        pants: (20, 120, 130, 255),
        accent: WATER_BLUE,
    },
];

const T: Pixel = (0, 0, 0, 0); // transparent
const D: Pixel = DARK; // outline / shadow

/// Generate an 8x8 character sprite for the given palette, direction, animation, and frame.
///
/// Kenney micro-roguelike style: 2px head, 3px body, 2px legs.
/// Palette index wraps at 6. LEFT is handled at render time via horizontal flip.
pub fn character_sprite(
    palette: u8,
    direction: Direction,
    anim_type: AnimType,
    frame: u8,
) -> SpriteData {
    let pal = &PALETTES[(palette % 6) as usize];

    let dir = match direction {
        Direction::Left => Direction::Right,
        other => other,
    };

    match dir {
        Direction::Down => build_front(pal, anim_type, frame),
        Direction::Up => build_back(pal, anim_type, frame),
        Direction::Right | Direction::Left => build_side(pal, anim_type, frame),
    }
}

fn build_front(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let s = pal.skin;
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let a = pal.accent;

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, D, a, a, a, a, D, T],
        vec![D, h, s, s, s, s, h, D],
        vec![D, s, D, s, s, D, s, D],
        vec![D, c, c, c, c, c, c, D],
        vec![s, D, c, c, c, c, D, s],
        vec![T, D, c, c, c, c, D, T],
        vec![T, D, p, T, T, p, D, T],
        vec![T, T, p, T, T, p, T, T],
    ];

    apply_walk(&mut rows, anim_type, frame, p);
    apply_action(&mut rows, anim_type, frame, pal);
    rows
}

fn build_back(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let a = pal.accent;

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, D, a, a, a, a, D, T],
        vec![D, h, h, h, h, h, h, D],
        vec![D, D, h, h, h, h, D, D],
        vec![D, c, c, c, c, c, c, D],
        vec![T, D, c, c, c, c, D, T],
        vec![T, D, c, c, c, c, D, T],
        vec![T, D, p, T, T, p, D, T],
        vec![T, T, p, T, T, p, T, T],
    ];

    apply_walk(&mut rows, anim_type, frame, p);
    apply_action_back(&mut rows, anim_type, frame, pal);
    rows
}

fn build_side(pal: &CharPalette, anim_type: AnimType, frame: u8) -> SpriteData {
    let s = pal.skin;
    let h = pal.hair;
    let c = pal.shirt;
    let p = pal.pants;
    let a = pal.accent;

    let mut rows: Vec<Vec<Pixel>> = vec![
        vec![T, D, a, a, a, D, T, T],
        vec![D, h, s, s, s, h, D, T],
        vec![D, s, h, D, s, s, D, T],
        vec![D, c, c, c, c, c, D, T],
        vec![T, D, c, c, c, c, s, D],
        vec![T, D, c, c, c, D, T, T],
        vec![T, T, D, p, p, D, T, T],
        vec![T, T, T, p, p, T, T, T],
    ];

    apply_walk_side(&mut rows, anim_type, frame, p);
    apply_action_side(&mut rows, anim_type, frame, pal);
    rows
}

fn apply_walk(rows: &mut [Vec<Pixel>], anim_type: AnimType, frame: u8, p: Pixel) {
    if !matches!(anim_type, AnimType::Walk) {
        return;
    }
    match frame % 4 {
        1 => {
            rows[6] = vec![T, p, T, T, T, T, p, T];
            rows[7] = vec![T, p, T, T, T, T, p, T];
        }
        3 => {
            rows[6] = vec![T, T, T, p, p, T, T, T];
            rows[7] = vec![T, T, T, p, p, T, T, T];
        }
        _ => {}
    }
}

fn apply_walk_side(rows: &mut [Vec<Pixel>], anim_type: AnimType, frame: u8, p: Pixel) {
    if !matches!(anim_type, AnimType::Walk) {
        return;
    }
    match frame % 4 {
        1 => {
            rows[6] = vec![T, T, p, T, T, p, T, T];
            rows[7] = vec![T, T, p, T, T, p, T, T];
        }
        3 => {
            rows[6] = vec![T, T, T, p, T, T, T, T];
            rows[7] = vec![T, T, T, p, T, T, T, T];
        }
        _ => {}
    }
}

fn apply_action(rows: &mut [Vec<Pixel>], anim_type: AnimType, frame: u8, pal: &CharPalette) {
    let c = pal.shirt;
    let s = pal.skin;
    let rod: Pixel = WOOD; // fishing rod
    let hoe: Pixel = STONE_GREY; // hoe head
    let handle: Pixel = WOOD; // tool handle
    match anim_type {
        AnimType::Type => {
            if !frame.is_multiple_of(2) {
                // Arms raise outward
                rows[3] = vec![s, c, c, c, c, c, c, s];
                rows[4] = vec![T, T, c, c, c, c, T, T];
            }
        }
        AnimType::Read => {
            if !frame.is_multiple_of(2) {
                // Lean forward
                rows[4] = vec![T, D, c, c, c, c, c, T];
                rows[5] = vec![T, T, T, c, c, c, T, T];
            }
        }
        AnimType::Fish => {
            // Fishing rod held to the right
            if frame.is_multiple_of(2) {
                rows[3] = vec![D, c, c, c, c, c, s, D];
                rows[4] = vec![T, D, c, c, c, c, D, rod];
                rows[5] = vec![T, D, c, c, c, c, D, rod];
            } else {
                // Rod dipped down (casting)
                rows[3] = vec![D, c, c, c, c, c, s, D];
                rows[4] = vec![T, D, c, c, c, c, s, D];
                rows[5] = vec![T, D, c, c, c, c, D, rod];
            }
        }
        AnimType::Farm => {
            // Hoeing: arm swings down with hoe
            if frame.is_multiple_of(2) {
                // Hoe raised
                rows[3] = vec![D, c, c, c, c, c, s, handle];
                rows[4] = vec![T, D, c, c, c, c, D, hoe];
            } else {
                // Hoe down — bent over working
                rows[3] = vec![D, c, c, c, c, c, c, D];
                rows[4] = vec![T, s, c, c, c, c, s, T];
                rows[5] = vec![T, T, handle, D, D, hoe, T, T];
            }
        }
        AnimType::Harvest => {
            // Reaching up to pick fruit
            if frame.is_multiple_of(2) {
                // Arms raised high
                rows[0] = vec![T, s, pal.accent, pal.accent, pal.accent, pal.accent, s, T];
                rows[3] = vec![D, c, c, c, c, c, c, D];
                rows[4] = vec![T, T, c, c, c, c, T, T];
            } else {
                // One arm up holding fruit
                rows[0] = vec![T, D, pal.accent, pal.accent, pal.accent, pal.accent, s, T];
                rows[3] = vec![D, c, c, c, c, c, c, D];
                rows[4] = vec![T, D, c, c, c, c, T, T];
            }
        }
        AnimType::Walk => {}
    }
}

fn apply_action_back(rows: &mut [Vec<Pixel>], anim_type: AnimType, frame: u8, pal: &CharPalette) {
    let c = pal.shirt;
    let handle: Pixel = WOOD;
    match anim_type {
        AnimType::Fish => {
            if frame.is_multiple_of(2) {
                rows[4] = vec![T, D, c, c, c, c, D, handle];
            }
        }
        AnimType::Farm => {
            if !frame.is_multiple_of(2) {
                rows[4] = vec![T, D, c, c, c, c, D, T];
                rows[5] = vec![T, T, handle, c, c, handle, T, T];
            }
        }
        AnimType::Harvest => {
            let s = pal.skin;
            if frame.is_multiple_of(2) {
                rows[0] = vec![T, s, pal.accent, pal.accent, pal.accent, pal.accent, s, T];
                rows[3] = vec![D, c, c, c, c, c, c, D];
            }
        }
        _ => {}
    }
}

fn apply_action_side(rows: &mut [Vec<Pixel>], anim_type: AnimType, frame: u8, pal: &CharPalette) {
    let c = pal.shirt;
    let s = pal.skin;
    let rod: Pixel = WOOD;
    let hoe: Pixel = STONE_GREY;
    let handle: Pixel = WOOD;
    match anim_type {
        AnimType::Type => {
            if !frame.is_multiple_of(2) {
                rows[4] = vec![T, D, c, c, c, s, T, T];
            }
        }
        AnimType::Fish => {
            if frame.is_multiple_of(2) {
                rows[4] = vec![T, D, c, c, c, s, rod, rod];
            } else {
                rows[4] = vec![T, D, c, c, c, s, rod, T];
                rows[5] = vec![T, D, c, c, c, D, rod, T];
            }
        }
        AnimType::Farm => {
            if frame.is_multiple_of(2) {
                rows[4] = vec![T, D, c, c, c, s, handle, hoe];
            } else {
                rows[4] = vec![T, D, c, c, c, s, T, T];
                rows[5] = vec![T, T, handle, c, c, hoe, T, T];
            }
        }
        AnimType::Harvest => {
            if frame.is_multiple_of(2) {
                rows[0] = vec![T, D, pal.accent, pal.accent, pal.accent, s, T, T];
                rows[4] = vec![T, T, c, c, c, c, T, T];
            } else {
                rows[1] = vec![D, pal.hair, s, s, s, pal.hair, s, T];
                rows[4] = vec![T, D, c, c, c, c, T, T];
            }
        }
        _ => {}
    }
}

/// Generate an 8x8 floor tile sprite for the given tile type.
///
/// Uses Dawnbringer 16 palette for farm terrain.
pub fn floor_sprite(tile: TileType) -> SpriteData {
    match tile {
        TileType::Grass => {
            // Lush grass with subtle blade texture
            let a = GRASS_GREEN;
            let b = MID_GREEN;
            let d = DARK_GREEN;
            vec![
                vec![a, a, b, a, a, a, a, a],
                vec![a, a, a, a, a, b, a, a],
                vec![a, b, a, a, a, a, a, b],
                vec![a, a, a, a, b, a, a, a],
                vec![a, a, a, a, a, a, b, a],
                vec![a, d, a, a, a, a, a, a],
                vec![a, a, a, b, a, a, a, a],
                vec![a, a, a, a, a, a, a, b],
            ]
        }
        TileType::GrassDark => {
            // Shaded grass with more variation
            let a = MID_GREEN;
            let b = DARK_GREEN;
            let c = GRASS_GREEN;
            vec![
                vec![a, a, b, a, a, a, c, a],
                vec![a, a, a, a, b, a, a, a],
                vec![a, b, a, a, a, a, a, b],
                vec![a, a, a, b, a, a, a, a],
                vec![a, a, a, a, a, b, a, a],
                vec![a, c, a, a, a, a, a, a],
                vec![a, a, a, a, b, a, a, b],
                vec![a, a, b, a, a, a, a, a],
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
            // Clean tilled soil — regular horizontal furrow lines
            let a = DIRT_BROWN;
            let b = DARK_BROWN;
            vec![
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![b, b, b, b, b, b, b, b],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, a, a, a, a, a],
                vec![b, b, b, b, b, b, b, b],
                vec![a, a, a, a, a, a, a, a],
                vec![a, a, a, a, a, a, a, a],
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
            // Packed dirt path — warm and natural
            let a = SKIN_MED;
            let b = DIRT_BROWN;
            let c = WHEAT;
            vec![
                vec![a, a, b, a, a, c, a, a],
                vec![a, c, a, a, a, a, b, a],
                vec![a, a, a, a, b, a, a, a],
                vec![b, a, a, c, a, a, a, a],
                vec![a, a, a, a, a, a, c, a],
                vec![a, b, a, a, c, a, a, a],
                vec![a, a, a, a, a, a, a, b],
                vec![a, a, c, a, a, b, a, a],
            ]
        }
        TileType::Fence => vec![vec![DARK_BROWN; 8]; 8],
        TileType::Void => vec![vec![DARK; 8]; 8],
    }
}

/// Generate an 8x16 fence post sprite with auto-tiling based on neighbor bitmask.
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
        "STUMP_FRONT" | "STUMP_BACK" | "STUMP_LEFT" | "STUMP_RIGHT" => stump_front_sprite(),
        "TREE" => tree_sprite(false),
        "TREE_FRUIT" => tree_sprite(true),
        "WELL" => well_sprite(),
        "MAILBOX" => mailbox_sprite(false),
        "MAILBOX_ON" => mailbox_sprite(true),
        "SCARECROW" => scarecrow_sprite(),
        "LANTERN" => lantern_sprite(),
        "CABIN_WALL" => cabin_wall_sprite(),
        "BARN_WALL" => barn_wall_sprite(),
        "FLOWER" => flower_sprite(),
        "BUSH" => bush_sprite(),
        "FENCE_H" => fence_h_sprite(),
        "FENCE_V" => fence_v_sprite(),
        "FISHING_SPOT" => fishing_spot_sprite(),
        "HOME" => home_sprite(),
        "CHICKEN_COOP" => chicken_coop_sprite(),
        "COW_PEN" => cow_pen_sprite(),
        _ => default_furniture_sprite(),
    }
}

fn crop_plot_sprite(active: bool) -> SpriteData {
    let d = DIRT_BROWN;
    let b = DARK_BROWN;
    let g = GRASS_GREEN;
    let m = MID_GREEN;
    let w = WHEAT;

    if active {
        // Dense wheat/crops growing tall on tilled rows
        vec![
            vec![T, g, w, g, w, g, w, T],
            vec![T, w, g, w, g, w, g, T],
            vec![T, g, w, g, w, g, w, T],
            vec![T, m, g, m, g, m, g, T],
            vec![T, m, m, m, m, m, m, T],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
            vec![d, d, d, d, d, d, d, d],
        ]
    } else {
        // Clean tilled rows with tiny green sprouts
        vec![
            vec![T, T, T, T, T, T, T, T],
            vec![T, T, g, T, T, g, T, T],
            vec![T, T, T, T, T, T, T, T],
            vec![T, T, T, g, T, T, g, T],
            vec![T, T, T, T, T, T, T, T],
            vec![d, d, d, d, d, d, d, d],
            vec![b, b, b, b, b, b, b, b],
            vec![d, d, d, d, d, d, d, d],
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
    let m = MID_GREEN;
    let d = DARK_GREEN;
    let w = WOOD;
    let b = DARK_BROWN;
    let fruit = WARM_RED;

    let f1 = if has_fruit { fruit } else { g };
    let f2 = if has_fruit { fruit } else { m };

    vec![
        vec![T, T, d, g, g, d, T, T],
        vec![T, d, g, g, f1, g, d, T],
        vec![T, d, g, m, g, g, d, T],
        vec![d, g, f2, g, g, m, g, d],
        vec![T, d, g, g, f1, g, d, T],
        vec![T, T, d, g, g, d, T, T],
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
    let r = DIRT_BROWN;

    vec![
        vec![b, b, b, b, b, b, b, b],
        vec![w, w, r, w, w, w, r, w],
        vec![w, w, w, w, r, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![w, r, w, w, w, w, r, w],
        vec![w, w, w, r, w, w, w, w],
        vec![b, b, b, b, b, b, b, b],
        vec![w, w, w, w, w, r, w, w],
    ]
}

fn barn_wall_sprite() -> SpriteData {
    let r = WARM_RED;
    let d = DARK_BROWN;
    let w = WOOD;

    vec![
        vec![d, d, d, d, d, d, d, d],
        vec![r, r, w, r, r, r, w, r],
        vec![r, r, r, r, w, r, r, r],
        vec![d, d, d, d, d, d, d, d],
        vec![r, w, r, r, r, r, w, r],
        vec![r, r, r, w, r, r, r, r],
        vec![d, d, d, d, d, d, d, d],
        vec![r, r, r, r, r, w, r, r],
    ]
}

fn flower_sprite() -> SpriteData {
    let g = GRASS_GREEN;
    let m = MID_GREEN;
    let y = WHEAT;
    let r = WARM_RED;

    vec![
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, T, y, T, T, T, T],
        vec![T, T, y, r, y, T, T, T],
        vec![T, T, T, y, T, T, T, T],
        vec![T, T, T, m, T, y, T, T],
        vec![T, T, T, m, y, r, y, T],
        vec![T, T, T, g, T, y, T, T],
        vec![T, T, T, g, T, g, T, T],
    ]
}

fn bush_sprite() -> SpriteData {
    let g = GRASS_GREEN;
    let m = MID_GREEN;
    let d = DARK_GREEN;

    vec![
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, d, g, g, d, T, T],
        vec![T, d, g, m, g, g, d, T],
        vec![T, d, g, g, m, g, d, T],
        vec![T, d, m, g, g, g, d, T],
        vec![T, T, d, g, m, d, T, T],
        vec![T, T, T, d, d, T, T, T],
        vec![T, T, T, T, T, T, T, T],
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

fn home_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let r = WARM_RED;
    let l = LIGHT;

    vec![
        vec![T, T, r, r, r, r, T, T],
        vec![T, r, r, r, r, r, r, T],
        vec![b, w, w, w, w, w, w, b],
        vec![b, w, l, w, w, l, w, b],
        vec![b, w, w, b, b, w, w, b],
        vec![b, w, w, b, b, w, w, b],
        vec![b, b, b, b, b, b, b, b],
        vec![T, T, T, T, T, T, T, T],
    ]
}

fn chicken_coop_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let r = WARM_RED;
    let y = WHEAT;

    vec![
        vec![T, b, b, b, b, b, b, T],
        vec![b, w, w, w, w, w, w, b],
        vec![b, w, T, r, T, r, w, b],
        vec![b, w, T, y, T, y, w, b],
        vec![b, w, y, y, y, y, w, b],
        vec![b, b, b, T, T, b, b, b],
        vec![T, T, T, T, T, T, T, T],
        vec![T, T, T, T, T, T, T, T],
    ]
}

fn cow_pen_sprite() -> SpriteData {
    let w = WOOD;
    let b = DARK_BROWN;
    let l = LIGHT;
    let d = DARK;
    let s = SKIN_MED;

    vec![
        vec![w, b, w, b, w, b, w, b],
        vec![b, T, T, l, l, T, T, b],
        vec![b, T, l, d, l, l, T, b],
        vec![b, T, l, l, d, l, T, b],
        vec![b, T, s, l, l, s, T, b],
        vec![b, T, l, T, T, l, T, b],
        vec![w, b, w, b, w, b, w, b],
        vec![T, T, T, T, T, T, T, T],
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
        assert_eq!(sprite.len(), 8);
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
            assert_eq!(sprite.len(), 8);
        }
    }

    #[test]
    fn activity_animations_produce_valid_sprites() {
        let anims = [
            AnimType::Type,
            AnimType::Read,
            AnimType::Fish,
            AnimType::Farm,
            AnimType::Harvest,
        ];
        for anim in anims {
            for frame in 0..2 {
                for dir in [
                    Direction::Down,
                    Direction::Up,
                    Direction::Right,
                    Direction::Left,
                ] {
                    let sprite = character_sprite(0, dir, anim, frame);
                    assert_eq!(
                        sprite.len(),
                        8,
                        "{anim:?} {dir:?} frame {frame} wrong height"
                    );
                    assert!(
                        sprite.iter().all(|row| row.len() == 8),
                        "{anim:?} {dir:?} frame {frame} wrong width"
                    );
                }
            }
        }
    }

    #[test]
    fn walk_animation_has_four_frames() {
        for frame in 0..4 {
            let sprite = character_sprite(0, Direction::Down, AnimType::Walk, frame);
            assert_eq!(sprite.len(), 8);
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
            "STUMP_BACK",
            "STUMP_LEFT",
            "STUMP_RIGHT",
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
            "HOME",
            "CHICKEN_COOP",
            "COW_PEN",
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
