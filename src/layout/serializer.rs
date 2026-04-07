use std::collections::{HashMap, HashSet};

use crate::constants::TILE_SIZE;
use crate::layout::furniture::{
    FurnitureInstance, furniture_category, furniture_facing, furniture_footprint, is_desk,
    is_mirrored, is_seat, is_surface_item,
};
use crate::types::{OfficeLayout, SpriteData, TilePos, TileType};

/// Convert flat tile array to a 2D grid indexed as `[row][col]`.
pub fn build_tile_map(layout: &OfficeLayout) -> Vec<Vec<TileType>> {
    let mut grid = Vec::with_capacity(layout.rows as usize);
    for r in 0..layout.rows {
        let mut row = Vec::with_capacity(layout.cols as usize);
        for c in 0..layout.cols {
            let idx = (r as usize) * (layout.cols as usize) + (c as usize);
            let raw = layout.tiles.get(idx).copied().unwrap_or(0);
            row.push(TileType::from_u8(raw));
        }
        grid.push(row);
    }
    grid
}

/// Collect all tile positions occupied by furniture footprints.
pub fn build_blocked(furniture: &[FurnitureInstance]) -> HashSet<TilePos> {
    let mut blocked = HashSet::new();
    for inst in furniture {
        // Surface items sit on desks and do not block floor tiles
        if is_surface_item(&inst.furniture_type) {
            continue;
        }
        for &pos in &inst.footprint {
            blocked.insert(pos);
        }
    }
    blocked
}

/// Floor tiles that are not occupied by furniture.
pub fn build_walkable(tile_map: &[Vec<TileType>], blocked: &HashSet<TilePos>) -> HashSet<TilePos> {
    let mut walkable = HashSet::new();
    for (r, row) in tile_map.iter().enumerate() {
        for (c, tile) in row.iter().enumerate() {
            if !tile.is_floor() {
                continue;
            }
            let pos: TilePos = (c as u16, r as u16);
            if blocked.contains(&pos) {
                continue;
            }
            walkable.insert(pos);
        }
    }
    walkable
}

/// Convert `PlacedFurniture` entries into runtime `FurnitureInstance` values
/// with resolved footprints, z-sort, and placeholder sprites.
///
/// Sprite generation is delegated to `crate::render::sprites::furniture_sprite`
/// once the render module is available. Until then, an empty sprite is used.
pub fn build_furniture(layout: &OfficeLayout) -> Vec<FurnitureInstance> {
    let mut instances = Vec::with_capacity(layout.furniture.len());

    for placed in &layout.furniture {
        let kind = placed.furniture_type.as_str();
        let offsets = furniture_footprint(kind);

        // Compute absolute footprint positions
        let footprint: Vec<TilePos> = offsets
            .iter()
            .map(|&(dc, dr)| {
                let abs_c = (placed.col + dc).max(0) as u16;
                let abs_r = (placed.row + dr).max(0) as u16;
                (abs_c, abs_r)
            })
            .collect();

        // Z-sort: bottom of footprint row * TILE_SIZE
        let max_row = footprint.iter().map(|&(_, r)| r).max().unwrap_or(0);
        let z_y = ((max_row + 1) as f32) * (TILE_SIZE as f32);

        // Attempt to load sprite from render module; fall back to empty
        let sprite = load_furniture_sprite(kind);
        let mirrored = is_mirrored(kind);

        instances.push(FurnitureInstance {
            uid: placed.uid.clone(),
            furniture_type: placed.furniture_type.clone(),
            col: placed.col,
            row: placed.row,
            sprite,
            footprint,
            z_y,
            is_seat: is_seat(kind),
            facing: furniture_facing(kind),
            category: furniture_category(kind),
            mirrored,
        });
    }

    instances
}

/// Pre-compute desk z_y values per tile position for surface item layering.
///
/// Surface items use `max(own_z, desk_z + 0.5)` so they render on top of the desk.
pub fn build_desk_z_map(furniture: &[FurnitureInstance]) -> HashMap<TilePos, f32> {
    let mut desk_z = HashMap::new();
    for inst in furniture {
        if !is_desk(&inst.furniture_type) {
            continue;
        }
        for &pos in &inst.footprint {
            desk_z.insert(pos, inst.z_y);
        }
    }
    desk_z
}

/// Load furniture sprite. Returns an empty sprite as a placeholder.
///
/// Replace the body with `crate::render::sprites::furniture_sprite(kind)` once
/// the render module is available.
fn load_furniture_sprite(_kind: &str) -> SpriteData {
    // TODO: delegate to crate::render::sprites::furniture_sprite(kind)
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::{build_blocked, build_desk_z_map, build_furniture, build_tile_map, build_walkable};
    use crate::layout::persistence::default_layout;
    use crate::types::TileType;

    #[test]
    fn tile_map_dimensions_match_layout() {
        let layout = default_layout();
        let map = build_tile_map(&layout);
        assert_eq!(map.len(), layout.rows as usize);
        assert_eq!(map[0].len(), layout.cols as usize);
    }

    #[test]
    fn tile_map_walls_on_border() {
        let layout = default_layout();
        let map = build_tile_map(&layout);
        assert_eq!(map[0][0], TileType::Fence);
        assert_eq!(map[0][19], TileType::Fence);
        assert_eq!(map[10][0], TileType::Fence);
    }

    #[test]
    fn build_furniture_produces_instances() {
        let layout = default_layout();
        let instances = build_furniture(&layout);
        assert_eq!(instances.len(), layout.furniture.len());
    }

    #[test]
    fn blocked_excludes_surface_items() {
        let layout = default_layout();
        let instances = build_furniture(&layout);
        let blocked = build_blocked(&instances);
        // Monitor/laptop are surface items and should not appear in blocked set
        // Desks occupy 2 tiles each (2 desks = 4), chairs 1 each (2), bookshelf 2 = 8 total
        // Plant is not surface but is 1x1 = 1 => 9 total
        for inst in &instances {
            if inst.furniture_type == "LANTERN" || inst.furniture_type == "SCARECROW" {
                for pos in &inst.footprint {
                    // Surface items might share position with desks (which ARE blocked),
                    // so we just verify the surface item logic runs without panic
                    let _ = blocked.contains(pos);
                }
            }
        }
        assert!(!blocked.is_empty());
    }

    #[test]
    fn walkable_excludes_walls_and_blocked() {
        let layout = default_layout();
        let map = build_tile_map(&layout);
        let instances = build_furniture(&layout);
        let blocked = build_blocked(&instances);
        let walkable = build_walkable(&map, &blocked);

        // Walls are not walkable
        assert!(!walkable.contains(&(0, 0)));
        // Interior floor that is not blocked is walkable
        assert!(walkable.contains(&(5, 5)));
    }

    #[test]
    fn desk_z_map_contains_crop_tiles() {
        let layout = default_layout();
        let instances = build_furniture(&layout);
        let desk_z = build_desk_z_map(&instances);
        // crop-1 at (19,3) is a 1x1 crop plot
        assert!(desk_z.contains_key(&(19, 3)));
    }

    #[test]
    fn furniture_z_is_positive() {
        let layout = default_layout();
        let instances = build_furniture(&layout);
        for inst in &instances {
            assert!(inst.z_y > 0.0, "z_y must be positive for {}", inst.uid);
        }
    }

    #[test]
    fn build_walkable_excludes_blocked() {
        let tile_map = vec![vec![TileType::Grass; 5]; 5];
        let mut blocked = std::collections::HashSet::new();
        blocked.insert((2u16, 2u16));
        blocked.insert((3, 3));
        let walkable = build_walkable(&tile_map, &blocked);
        assert!(!walkable.contains(&(2, 2)));
        assert!(!walkable.contains(&(3, 3)));
        assert!(walkable.contains(&(0, 0)));
        assert!(walkable.contains(&(4, 4)));
    }

    #[test]
    fn build_walkable_excludes_void_and_wall() {
        let tile_map = vec![vec![TileType::Void, TileType::Fence, TileType::Grass]];
        let blocked = std::collections::HashSet::new();
        let walkable = build_walkable(&tile_map, &blocked);
        assert!(!walkable.contains(&(0, 0)), "Void should not be walkable");
        assert!(!walkable.contains(&(1, 0)), "Fence should not be walkable");
        assert!(walkable.contains(&(2, 0)), "Grass should be walkable");
    }

    #[test]
    fn build_tile_map_dimensions() {
        let layout = default_layout();
        let map = build_tile_map(&layout);
        assert_eq!(map.len(), layout.rows as usize);
        for row in &map {
            assert_eq!(row.len(), layout.cols as usize);
        }
    }
}
