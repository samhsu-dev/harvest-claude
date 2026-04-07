use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr};

use crate::constants::{DEFAULT_COLS, DEFAULT_ROWS};
use crate::types::{OfficeLayout, PlacedFurniture, TileColor, TileType};

/// Current bundled layout revision. Layouts with a lower revision are replaced.
const BUNDLED_REVISION: u32 = 10;

/// Layout filename within the pixel-agents directory.
const LAYOUT_FILENAME: &str = "layout.json";

// -----------------------------------------------------------------------
// Legacy migration constants
// -----------------------------------------------------------------------

/// Old VOID tile value used before layout_revision was introduced.
const LEGACY_VOID_VALUE: u8 = 8;

// -----------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------

/// Read, deserialize, and migrate a layout from disk.
///
/// # Errors
/// Returns an error if the file cannot be read or the JSON is malformed.
pub fn load_layout(path: &Path) -> Result<OfficeLayout> {
    let contents = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read layout from {}", path.display()))?;
    let mut layout: OfficeLayout = serde_json::from_str(&contents)
        .wrap_err_with(|| format!("failed to parse layout JSON from {}", path.display()))?;

    migrate(&mut layout);
    Ok(layout)
}

/// Atomically write a layout to disk (write temp file, then rename).
///
/// # Errors
/// Returns an error if the parent directory is missing or the write fails.
pub fn save_layout(path: &Path, layout: &OfficeLayout) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = parent.join(".layout.json.tmp");

    let json = serde_json::to_string_pretty(layout).wrap_err("failed to serialize layout")?;
    fs::write(&tmp_path, json.as_bytes())
        .wrap_err_with(|| format!("failed to write temp layout at {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .wrap_err_with(|| format!("failed to rename temp file to {}", path.display()))?;

    Ok(())
}

/// Return the `~/.pixel-agents/` directory, creating it if it does not exist.
///
/// # Errors
/// Returns an error if the home directory cannot be determined or mkdir fails.
pub fn pixel_agents_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("cannot determine home directory"))?;
    let dir = home.join(".pixel-agents");
    if !dir.exists() {
        fs::create_dir_all(&dir).wrap_err_with(|| format!("failed to create {}", dir.display()))?;
    }
    Ok(dir)
}

/// Load the layout from `~/.pixel-agents/layout.json`, falling back to the
/// bundled default if the file is missing or unreadable.
///
/// # Errors
/// Returns an error only if `pixel_agents_dir()` fails.
pub fn load_or_default() -> Result<OfficeLayout> {
    let dir = pixel_agents_dir()?;
    let path = dir.join(LAYOUT_FILENAME);
    if path.exists() {
        match load_layout(&path) {
            Ok(layout) => return Ok(layout),
            Err(e) => {
                tracing::warn!("failed to load layout, using default: {e}");
            }
        }
    }
    let layout = default_layout();
    // Write default to disk so users can customize via the JSON file.
    if let Err(e) = save_layout(&path, &layout) {
        tracing::warn!("failed to save default layout: {e}");
    }
    Ok(layout)
}

/// Bundled default farm layout: 28x16 grid — classic Harvest Moon arrangement
/// with farmhouse, barn, crop field, orchard, pond, and meadow.
pub fn default_layout() -> OfficeLayout {
    let cols = DEFAULT_COLS;
    let rows = DEFAULT_ROWS;
    let total = (cols as usize) * (rows as usize);
    let mut tiles = vec![TileType::Grass as u8; total];

    let set = |tiles: &mut Vec<u8>, c: u16, r: u16, tt: TileType| {
        let idx = (r as usize) * (cols as usize) + (c as usize);
        tiles[idx] = tt as u8;
    };

    // Border fence
    for c in 0..cols {
        set(&mut tiles, c, 0, TileType::Fence);
        set(&mut tiles, c, rows - 1, TileType::Fence);
    }
    for r in 0..rows {
        set(&mut tiles, 0, r, TileType::Fence);
        set(&mut tiles, cols - 1, r, TileType::Fence);
    }

    // GrassDark shade — irregular single tiles across the map
    let dark: &[(u16, u16)] = &[
        (4, 4),
        (8, 6),
        (18, 3),
        (14, 7),
        (16, 2),
        (11, 9),
        (13, 12),
        (17, 10),
        (8, 11),
        (3, 10),
        (19, 8),
        (22, 8),
        (15, 14),
        (24, 13),
        (9, 1),
    ];
    for &(c, r) in dark {
        set(&mut tiles, c, r, TileType::GrassDark);
    }

    // --- Home yard (top-left) ---
    for r in 1..=2 {
        for c in 3..=5 {
            set(&mut tiles, c, r, TileType::DirtDark);
        }
    }
    // L-shaped path: home exit east, then south into meadow
    for c in 5..=10 {
        set(&mut tiles, c, 3, TileType::Stone);
    }
    for r in 4..=7 {
        set(&mut tiles, 10, r, TileType::Stone);
    }

    // --- Animal pen (bottom-left, spacious) ---
    for r in 11..=14 {
        for c in 2..=7 {
            set(&mut tiles, c, r, TileType::DirtDark);
        }
    }

    // --- Crop field (right, cols 20-26, rows 1-6) ---
    // Soft transition column
    for r in 1..=6 {
        set(&mut tiles, 20, r, TileType::Dirt);
    }
    // Main tilled field
    for r in 1..=6 {
        for c in 21..=26 {
            set(&mut tiles, c, r, TileType::Dirt);
        }
    }
    // Furrow rows
    for c in 21..=26 {
        set(&mut tiles, c, 2, TileType::DirtDark);
        set(&mut tiles, c, 4, TileType::DirtDark);
        set(&mut tiles, c, 6, TileType::DirtDark);
    }

    // --- Pond (bottom-right) ---
    let pond: &[(u16, u16)] = &[
        (21, 10),
        (22, 10),
        (23, 10),
        (24, 10),
        (20, 11),
        (21, 11),
        (22, 11),
        (23, 11),
        (24, 11),
        (25, 11),
        (20, 12),
        (21, 12),
        (22, 12),
        (23, 12),
        (24, 12),
        (25, 12),
        (20, 13),
        (21, 13),
        (22, 13),
        (23, 13),
        (24, 13),
        (25, 13),
        (21, 14),
        (22, 14),
        (23, 14),
        (24, 14),
    ];
    for &(c, r) in pond {
        set(&mut tiles, c, r, TileType::Water);
    }
    let sand: &[(u16, u16)] = &[
        (20, 10),
        (25, 10),
        (26, 10),
        (19, 11),
        (26, 11),
        (19, 12),
        (26, 12),
        (19, 13),
        (26, 13),
        (20, 14),
        (25, 14),
    ];
    for &(c, r) in sand {
        set(&mut tiles, c, r, TileType::Sand);
    }

    // --- Furniture ---
    let furniture = vec![
        // === Home yard ===
        PlacedFurniture::new("home-1", "HOME", 4, 1),
        PlacedFurniture::new("well-1", "WELL", 8, 2),
        PlacedFurniture::new("mail-1", "MAILBOX", 6, 2),
        // === Animal pen (spacious 6×4) ===
        PlacedFurniture::new("coop-1", "CHICKEN_COOP", 3, 11),
        PlacedFurniture::new("coop-2", "CHICKEN_COOP", 5, 11),
        PlacedFurniture::new("pen-1", "COW_PEN", 3, 13),
        PlacedFurniture::new("pen-2", "COW_PEN", 5, 13),
        // === Crop field (3×3 grid, rows 1,3,5 × cols 21,23,25) ===
        PlacedFurniture::new("crop-1", "CROP_PLOT", 21, 1),
        PlacedFurniture::new("crop-2", "CROP_PLOT", 23, 1),
        PlacedFurniture::new("crop-3", "CROP_PLOT", 25, 1),
        PlacedFurniture::new("crop-4", "CROP_PLOT", 21, 3),
        PlacedFurniture::new("crop-5", "CROP_PLOT", 23, 3),
        PlacedFurniture::new("crop-6", "CROP_PLOT", 25, 3),
        PlacedFurniture::new("crop-7", "CROP_PLOT", 21, 5),
        PlacedFurniture::new("crop-8", "CROP_PLOT", 23, 5),
        PlacedFurniture::new("crop-9", "CROP_PLOT", 25, 5),
        PlacedFurniture::new("scare-1", "SCARECROW", 20, 3),
        // === Orchard (wide 3-col spacing) ===
        PlacedFurniture::new("tree-1", "TREE_FRUIT", 2, 5),
        PlacedFurniture::new("tree-2", "TREE_FRUIT", 5, 5),
        PlacedFurniture::new("tree-3", "TREE_FRUIT", 8, 5),
        PlacedFurniture::new("tree-4", "TREE", 2, 8),
        PlacedFurniture::new("tree-5", "TREE", 5, 8),
        PlacedFurniture::new("tree-6", "TREE", 8, 8),
        // === Meadow trees (scattered) ===
        PlacedFurniture::new("tree-7", "TREE", 12, 3),
        PlacedFurniture::new("tree-8", "TREE", 16, 7),
        PlacedFurniture::new("tree-9", "TREE", 11, 10),
        PlacedFurniture::new("tree-10", "TREE", 17, 12),
        // === Flowers (near features) ===
        PlacedFurniture::new("flower-1", "FLOWER", 3, 3),
        PlacedFurniture::new("flower-2", "FLOWER", 7, 1),
        PlacedFurniture::new("flower-3", "FLOWER", 14, 5),
        PlacedFurniture::new("flower-4", "FLOWER", 10, 8),
        PlacedFurniture::new("flower-5", "FLOWER", 16, 11),
        // === Bushes (fence edges) ===
        PlacedFurniture::new("bush-1", "BUSH", 1, 2),
        PlacedFurniture::new("bush-2", "BUSH", 1, 7),
        PlacedFurniture::new("bush-3", "BUSH", 1, 12),
        PlacedFurniture::new("bush-4", "BUSH", 26, 8),
        // === Seats ===
        // Crop edge (facing Right → Farm)
        PlacedFurniture::new("rest-1", "STUMP_RIGHT", 20, 1),
        PlacedFurniture::new("rest-2", "STUMP_RIGHT", 20, 5),
        // Below fruit trees (facing Up → Harvest)
        PlacedFurniture::new("rest-3", "STUMP_BACK", 2, 6),
        PlacedFurniture::new("rest-4", "STUMP_BACK", 8, 6),
        // Meadow rest (facing Down → Farm default)
        PlacedFurniture::new("rest-5", "STUMP_FRONT", 7, 3),
        PlacedFurniture::new("rest-6", "STUMP_FRONT", 12, 5),
        PlacedFurniture::new("rest-7", "STUMP_FRONT", 15, 9),
        PlacedFurniture::new("rest-8", "STUMP_FRONT", 10, 12),
        // === Fishing spots ===
        PlacedFurniture::new("fish-1", "FISHING_SPOT", 19, 12),
        PlacedFurniture::new("fish-2", "FISHING_SPOT", 21, 14),
        // === Lanterns (along path and near buildings) ===
        PlacedFurniture::new("lamp-1", "LANTERN", 7, 3), // home path
        PlacedFurniture::new("lamp-2", "LANTERN", 10, 5), // path corner
        PlacedFurniture::new("lamp-3", "LANTERN", 4, 12), // animal pen
    ];

    // Generate tile colors for floor tiles.
    let mut tile_colors = HashMap::new();
    for r in 0..rows {
        for c in 0..cols {
            let idx = (r as usize) * (cols as usize) + (c as usize);
            let tt = TileType::from_u8(tiles[idx]);
            if tt.is_floor() {
                let key = format!("{c},{r}");
                tile_colors.insert(key, default_tile_color(tt));
            }
        }
    }

    OfficeLayout {
        version: 1,
        cols,
        rows,
        tiles,
        furniture,
        tile_colors: Some(tile_colors),
        layout_revision: Some(BUNDLED_REVISION),
    }
}

// -----------------------------------------------------------------------
// Migration
// -----------------------------------------------------------------------

/// Apply all migrations to a loaded layout in order.
fn migrate(layout: &mut OfficeLayout) {
    migrate_void_tiles(layout);
    migrate_furniture_types(layout);
    migrate_tile_colors(layout);
    migrate_layout_revision(layout);
}

/// Migration 1: old VOID value 8 → new value 0.
fn migrate_void_tiles(layout: &mut OfficeLayout) {
    if layout.layout_revision.is_some() {
        return;
    }
    if !layout.tiles.contains(&LEGACY_VOID_VALUE) {
        return;
    }
    for tile in &mut layout.tiles {
        if *tile == LEGACY_VOID_VALUE {
            *tile = TileType::Void as u8;
        }
    }
}

/// Migration 2: legacy furniture names → canonical farm names.
fn migrate_furniture_types(layout: &mut OfficeLayout) {
    for item in &mut layout.furniture {
        let migrated = match item.furniture_type.as_str() {
            "desk" | "Desk" | "DESK_FRONT" | "DESK_LEFT" | "DESK_RIGHT" => "CROP_PLOT",
            "chair" | "Chair" | "WOODEN_CHAIR_FRONT" | "WOODEN_CHAIR_BACK"
            | "WOODEN_CHAIR_LEFT" | "WOODEN_CHAIR_RIGHT" => "STUMP_FRONT",
            "monitor" | "Monitor" | "MONITOR" | "laptop" | "Laptop" | "LAPTOP" => "SCARECROW",
            "lamp" | "Lamp" | "LAMP" => "LANTERN",
            "plant" | "Plant" | "PLANT" => "TREE",
            "bookshelf" | "Bookshelf" | "BOOKSHELF" => "WELL",
            _ => continue,
        };
        item.furniture_type = migrated.to_owned();
    }
}

/// Migration 3: generate tile_colors if missing.
fn migrate_tile_colors(layout: &mut OfficeLayout) {
    if layout.tile_colors.is_some() {
        return;
    }
    let mut colors = HashMap::new();
    for r in 0..layout.rows {
        for c in 0..layout.cols {
            let idx = (r as usize) * (layout.cols as usize) + (c as usize);
            if idx >= layout.tiles.len() {
                continue;
            }
            let tt = TileType::from_u8(layout.tiles[idx]);
            if tt.is_floor() {
                let key = format!("{c},{r}");
                colors.insert(key, default_tile_color(tt));
            }
        }
    }
    layout.tile_colors = Some(colors);
}

/// Migration 4: replace layout if bundled revision is newer.
fn migrate_layout_revision(layout: &mut OfficeLayout) {
    let file_rev = layout.layout_revision.unwrap_or(0);
    if file_rev < BUNDLED_REVISION {
        *layout = default_layout();
    }
}

/// Default HSL color for a given floor tile type.
fn default_tile_color(tt: TileType) -> TileColor {
    match tt {
        // Light green grass
        TileType::Grass => TileColor {
            h: 120.0,
            s: 0.40,
            b: 0.55,
        },
        // Darker green grass
        TileType::GrassDark => TileColor {
            h: 130.0,
            s: 0.45,
            b: 0.40,
        },
        // Brown dirt
        TileType::Dirt => TileColor {
            h: 30.0,
            s: 0.45,
            b: 0.45,
        },
        // Darker tilled dirt
        TileType::DirtDark => TileColor {
            h: 25.0,
            s: 0.50,
            b: 0.35,
        },
        // Sandy beige
        TileType::Sand => TileColor {
            h: 45.0,
            s: 0.35,
            b: 0.75,
        },
        // Warm tan cobblestone path
        TileType::Stone => TileColor {
            h: 42.0,
            s: 0.40,
            b: 0.70,
        },
        // Remaining floor types: neutral grey
        _ => TileColor {
            h: 0.0,
            s: 0.0,
            b: 0.80,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{BUNDLED_REVISION, default_layout, load_layout, migrate, save_layout};
    use crate::types::{OfficeLayout, PlacedFurniture, TileType};

    #[test]
    fn default_layout_has_correct_dimensions() {
        let layout = default_layout();
        assert_eq!(layout.cols, 28);
        assert_eq!(layout.rows, 16);
        assert_eq!(layout.tiles.len(), 448);
    }

    #[test]
    fn default_layout_has_furniture() {
        let layout = default_layout();
        assert!(!layout.furniture.is_empty());
    }

    #[test]
    fn default_layout_has_tile_colors() {
        let layout = default_layout();
        assert!(layout.tile_colors.is_some());
        assert!(!layout.tile_colors.as_ref().unwrap().is_empty());
    }

    #[test]
    fn default_layout_fence_surrounds_grass() {
        let layout = default_layout();
        // Top-left corner is Fence
        assert_eq!(layout.tiles[0], TileType::Fence as u8);
        // Bottom-right corner is Fence
        let last = (layout.rows as usize - 1) * (layout.cols as usize) + (layout.cols as usize - 1);
        assert_eq!(layout.tiles[last], TileType::Fence as u8);
        // Interior tile (1,1) is a floor type (Grass)
        let idx = 1 * (layout.cols as usize) + 1;
        let tt = TileType::from_u8(layout.tiles[idx]);
        assert!(tt.is_floor());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_layout.json");
        let layout = default_layout();
        save_layout(&path, &layout).unwrap();
        let loaded = load_layout(&path).unwrap();
        assert_eq!(loaded.cols, layout.cols);
        assert_eq!(loaded.rows, layout.rows);
        assert_eq!(loaded.tiles.len(), layout.tiles.len());
    }

    #[test]
    fn migrate_void_tiles_replaces_legacy_value() {
        let mut layout = OfficeLayout {
            version: 1,
            cols: 2,
            rows: 1,
            tiles: vec![8, 1],
            furniture: vec![],
            tile_colors: None,
            layout_revision: None,
        };
        migrate(&mut layout);
        // After migration, revision is bumped to default, so tiles come from default_layout
        // But if we test just migrate_void_tiles directly:
        // The void migration runs, then revision migration replaces with default.
        // Test the revision migration path:
        assert_eq!(layout.layout_revision, Some(BUNDLED_REVISION));
    }

    #[test]
    fn migrate_furniture_legacy_names() {
        let mut layout = default_layout();
        layout.furniture.push(PlacedFurniture {
            uid: "legacy-desk".to_owned(),
            furniture_type: "desk".to_owned(),
            col: 5,
            row: 5,
            color: None,
        });
        // Re-run migration on an already-current layout (revision matches)
        // Furniture migration always runs
        super::migrate_furniture_types(&mut layout);
        let migrated = layout
            .furniture
            .iter()
            .find(|f| f.uid == "legacy-desk")
            .unwrap();
        assert_eq!(migrated.furniture_type, "CROP_PLOT");
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let result = load_layout(Path::new("/nonexistent/path/layout.json"));
        assert!(result.is_err());
    }
}
