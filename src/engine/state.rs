use std::collections::{HashMap, HashSet};

use rand::Rng;

use crate::constants::{AUTO_ON_FACING_DEPTH, FURNITURE_ANIM_INTERVAL_SECS, TILE_SIZE};
use crate::engine::character::Character;
use crate::engine::seat::{self, Seat};
use crate::layout::furniture::{FurnitureInstance, is_electronics};
use crate::types::{Direction, OfficeLayout, TilePos, TileType};

/// Runtime office world state: grid, characters, seats, furniture.
#[derive(Debug)]
pub struct OfficeState {
    /// Original layout data.
    pub layout: OfficeLayout,
    /// 2D tile grid: `tile_map[row][col]`.
    pub tile_map: Vec<Vec<TileType>>,
    /// All active characters.
    pub characters: Vec<Character>,
    /// Resolved furniture instances with sprites and footprints.
    pub furniture: Vec<FurnitureInstance>,
    /// Derived seats from chair furniture.
    pub seats: Vec<Seat>,
    /// Tiles blocked by furniture footprints.
    pub blocked: HashSet<TilePos>,
    /// Walkable floor tiles (floor minus blocked).
    pub walkable: HashSet<TilePos>,
    /// Furniture animation timer (accumulates across frames).
    pub furniture_anim_timer: f32,
    /// Pre-computed desk z-sort values per tile.
    pub desk_z_by_tile: HashMap<TilePos, f32>,
}

impl OfficeState {
    /// Build runtime state from a serialized layout.
    ///
    /// Computes tile_map, furniture instances, seats, blocked/walkable sets,
    /// and desk z-sort map.
    pub fn from_layout(layout: OfficeLayout) -> Self {
        let tile_map = build_tile_map(&layout);
        let furniture = build_furniture(&layout);
        let seats = seat::derive_seats(&furniture, &tile_map);
        let blocked = build_blocked(&furniture);
        let walkable = build_walkable(&tile_map, &blocked);
        let desk_z_by_tile = build_desk_z_map(&furniture);

        Self {
            layout,
            tile_map,
            characters: Vec::new(),
            furniture,
            seats,
            blocked,
            walkable,
            furniture_anim_timer: 0.0,
            desk_z_by_tile,
        }
    }

    /// Spawn a character for the given agent, returning its index.
    ///
    /// Assigns a free seat if available, otherwise spawns at a random walkable tile.
    pub fn add_character(&mut self, agent_id: usize, palette: u8, hue_shift: Option<i16>) -> usize {
        let (spawn_pos, seat_idx) = if let Some(si) = self.find_free_seat() {
            let seat = &self.seats[si];
            ((seat.col, seat.row), Some(si))
        } else {
            let pos = self.random_walkable_tile();
            (pos, None)
        };

        let mut ch = Character::new(agent_id, spawn_pos, palette, hue_shift);
        ch.seat_id = seat_idx;

        if let Some(si) = seat_idx {
            self.seats[si].occupied_by = Some(agent_id);
            ch.direction = self.seats[si].facing;
        }

        self.characters.push(ch);
        self.characters.len() - 1
    }

    /// Remove a character by agent ID. Frees the assigned seat.
    pub fn remove_character(&mut self, agent_id: usize) {
        if let Some(pos) = self.characters.iter().position(|c| c.agent_id == agent_id) {
            let ch = &self.characters[pos];
            if let Some(si) = ch.seat_id
                && let Some(seat) = self.seats.get_mut(si)
            {
                seat.occupied_by = None;
            }
            self.characters.remove(pos);
        }
    }

    /// Tick all characters, matrix effects, and furniture animation.
    pub fn update(&mut self, dt: f64) {
        let dt_f32 = dt as f32;

        // Advance furniture animation timer
        let prev_frame = furniture_anim_frame(self.furniture_anim_timer);
        self.furniture_anim_timer += dt_f32;
        let curr_frame = furniture_anim_frame(self.furniture_anim_timer);

        if prev_frame != curr_frame {
            self.rebuild_furniture_sprites();
        }

        // Clone walkable/seats to avoid borrow conflict during character updates
        let walkable = self.walkable.clone();
        let seats = self.seats.clone();
        for ch in &mut self.characters {
            ch.update(dt, &walkable, &seats);
        }
    }

    /// Find a free seat, preferring seats facing electronics.
    ///
    /// Scans `AUTO_ON_FACING_DEPTH` tiles in the seat's facing direction
    /// to detect electronics. Prefers those seats over non-facing ones.
    pub fn find_free_seat(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        let mut best_has_electronics = false;

        for (i, seat) in self.seats.iter().enumerate() {
            if seat.occupied_by.is_some() {
                continue;
            }

            let has_elec = self.seat_faces_electronics(seat);

            if !best_has_electronics && has_elec {
                best = Some(i);
                best_has_electronics = true;
            } else if best.is_none() {
                best = Some(i);
            }
        }

        best
    }

    /// Find the nearest free seat to a given tile (Manhattan distance).
    pub fn find_nearest_free_seat(&self, near: TilePos) -> Option<usize> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.occupied_by.is_none())
            .min_by_key(|(_, s)| {
                let dc = (s.col as i32 - near.0 as i32).unsigned_abs();
                let dr = (s.row as i32 - near.1 as i32).unsigned_abs();
                dc + dr
            })
            .map(|(i, _)| i)
    }

    /// Rebuild furniture sprites based on agent facing detection.
    ///
    /// Electronics facing an active agent switch to ON sprite.
    pub fn rebuild_furniture_sprites(&mut self) {
        // Collect facing info first to avoid borrow conflict
        let facing_flags: Vec<bool> = self
            .furniture
            .iter()
            .map(|furn| {
                if !is_electronics(&furn.furniture_type) {
                    return false;
                }
                self.has_active_agent_facing(furn)
            })
            .collect();

        for (furn, facing_active) in self.furniture.iter_mut().zip(facing_flags) {
            if !is_electronics(&furn.furniture_type) {
                continue;
            }

            let base = furn.furniture_type.trim_end_matches("_ON");
            if facing_active {
                if !furn.furniture_type.ends_with("_ON") {
                    furn.furniture_type = format!("{base}_ON");
                }
            } else if furn.furniture_type.ends_with("_ON") {
                furn.furniture_type = base.to_owned();
            }
        }
    }

    /// Lookup a character by agent ID (immutable).
    pub fn character_by_agent(&self, agent_id: usize) -> Option<&Character> {
        self.characters.iter().find(|c| c.agent_id == agent_id)
    }

    /// Lookup a character by agent ID (mutable).
    pub fn character_by_agent_mut(&mut self, agent_id: usize) -> Option<&mut Character> {
        self.characters.iter_mut().find(|c| c.agent_id == agent_id)
    }

    /// Find the character index at a given tile (hit-test).
    ///
    /// Characters are 2 tiles tall, so this checks both the foot tile
    /// and the tile above it.
    pub fn character_at_tile(&self, pos: TilePos) -> Option<usize> {
        self.characters.iter().position(|c| {
            let foot = c.current_tile();
            foot == pos || (pos.1 + 1 == foot.1 && pos.0 == foot.0)
        })
    }

    fn random_walkable_tile(&self) -> TilePos {
        if self.walkable.is_empty() {
            return (0, 0);
        }
        let tiles: Vec<TilePos> = self.walkable.iter().copied().collect();
        let mut rng = rand::rng();
        tiles[rng.random_range(0..tiles.len())]
    }

    fn seat_faces_electronics(&self, seat: &Seat) -> bool {
        let (dc, dr) = direction_delta(seat.facing);

        for depth in 1..=AUTO_ON_FACING_DEPTH as i32 {
            let col = seat.col as i32 + dc * depth;
            let row = seat.row as i32 + dr * depth;
            if col < 0 || row < 0 {
                continue;
            }

            let check_pos = (col as u16, row as u16);
            let has = self
                .furniture
                .iter()
                .any(|f| is_electronics(&f.furniture_type) && f.footprint.contains(&check_pos));
            if has {
                return true;
            }
        }
        false
    }

    fn has_active_agent_facing(&self, furn: &FurnitureInstance) -> bool {
        for ch in &self.characters {
            if !ch.is_active {
                continue;
            }
            let Some(seat_idx) = ch.seat_id else {
                continue;
            };
            let Some(seat) = self.seats.get(seat_idx) else {
                continue;
            };

            let (dc, dr) = direction_delta(seat.facing);

            for depth in 1..=AUTO_ON_FACING_DEPTH as i32 {
                let col = seat.col as i32 + dc * depth;
                let row = seat.row as i32 + dr * depth;
                if col < 0 || row < 0 {
                    continue;
                }
                let check_pos = (col as u16, row as u16);
                if furn.footprint.contains(&check_pos) {
                    return true;
                }
            }
        }
        false
    }
}

/// Convert facing direction to (dcol, drow) delta.
fn direction_delta(dir: Direction) -> (i32, i32) {
    match dir {
        Direction::Up => (0, -1),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    }
}

/// Compute current furniture animation frame from timer.
fn furniture_anim_frame(timer: f32) -> u32 {
    (timer / FURNITURE_ANIM_INTERVAL_SECS) as u32
}

// -- Layout building helpers --

fn build_tile_map(layout: &OfficeLayout) -> Vec<Vec<TileType>> {
    let cols = layout.cols as usize;
    let rows = layout.rows as usize;
    let mut map = Vec::with_capacity(rows);

    for r in 0..rows {
        let mut row = Vec::with_capacity(cols);
        for c in 0..cols {
            let idx = r * cols + c;
            let byte = layout.tiles.get(idx).copied().unwrap_or(0);
            row.push(TileType::from_u8(byte));
        }
        map.push(row);
    }

    map
}

fn build_furniture(layout: &OfficeLayout) -> Vec<FurnitureInstance> {
    use crate::layout::furniture::{
        furniture_category, furniture_facing, furniture_footprint, is_mirrored, is_seat,
    };

    layout
        .furniture
        .iter()
        .map(|pf| {
            let offsets = furniture_footprint(&pf.furniture_type);
            let footprint: Vec<TilePos> = offsets
                .iter()
                .map(|&(dc, dr)| ((pf.col + dc) as u16, (pf.row + dr) as u16))
                .collect();

            let z_y = footprint
                .iter()
                .map(|&(_, row)| (row as f32 + 1.0) * TILE_SIZE as f32)
                .fold(0.0_f32, f32::max);

            FurnitureInstance {
                uid: pf.uid.clone(),
                furniture_type: pf.furniture_type.clone(),
                col: pf.col,
                row: pf.row,
                sprite: vec![],
                footprint,
                z_y,
                is_seat: is_seat(&pf.furniture_type),
                facing: furniture_facing(&pf.furniture_type),
                category: furniture_category(&pf.furniture_type),
                mirrored: is_mirrored(&pf.furniture_type),
            }
        })
        .collect()
}

fn build_blocked(furniture: &[FurnitureInstance]) -> HashSet<TilePos> {
    furniture
        .iter()
        .flat_map(|f| f.footprint.iter().copied())
        .collect()
}

fn build_walkable(tile_map: &[Vec<TileType>], blocked: &HashSet<TilePos>) -> HashSet<TilePos> {
    let mut walkable = HashSet::new();
    for (r, row) in tile_map.iter().enumerate() {
        for (c, &tile) in row.iter().enumerate() {
            let pos = (c as u16, r as u16);
            if tile.is_floor() && !blocked.contains(&pos) {
                walkable.insert(pos);
            }
        }
    }
    walkable
}

fn build_desk_z_map(furniture: &[FurnitureInstance]) -> HashMap<TilePos, f32> {
    let mut map = HashMap::new();
    for furn in furniture {
        if furn.category.as_deref() != Some("desk") {
            continue;
        }
        for &tile in &furn.footprint {
            let z = furn.z_y + 0.5;
            map.entry(tile)
                .and_modify(|existing: &mut f32| *existing = existing.max(z))
                .or_insert(z);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::OfficeState;
    use crate::types::{OfficeLayout, PlacedFurniture};

    fn minimal_layout() -> OfficeLayout {
        OfficeLayout {
            version: 1,
            cols: 4,
            rows: 3,
            tiles: vec![1; 12],
            furniture: vec![],
            tile_colors: None,
            layout_revision: None,
        }
    }

    fn layout_with_seat() -> OfficeLayout {
        let mut layout = minimal_layout();
        layout
            .furniture
            .push(PlacedFurniture::new("stump1", "STUMP_FRONT", 2, 1));
        layout
    }

    #[test]
    fn from_layout_builds_tile_map() {
        let state = OfficeState::from_layout(minimal_layout());
        assert_eq!(state.tile_map.len(), 3);
        assert_eq!(state.tile_map[0].len(), 4);
    }

    #[test]
    fn from_layout_computes_walkable() {
        let state = OfficeState::from_layout(minimal_layout());
        assert_eq!(state.walkable.len(), 12);
    }

    #[test]
    fn chair_creates_seat() {
        let state = OfficeState::from_layout(layout_with_seat());
        assert_eq!(state.seats.len(), 1);
        assert_eq!(state.seats[0].col, 2);
        assert_eq!(state.seats[0].row, 1);
    }

    #[test]
    fn chair_tile_is_blocked() {
        let state = OfficeState::from_layout(layout_with_seat());
        assert!(state.blocked.contains(&(2, 1)));
        assert!(!state.walkable.contains(&(2, 1)));
    }

    #[test]
    fn add_and_remove_character() {
        let mut state = OfficeState::from_layout(layout_with_seat());
        let idx = state.add_character(42, 0, None);
        assert_eq!(state.characters.len(), 1);
        assert_eq!(state.characters[idx].agent_id, 42);
        assert!(state.seats[0].occupied_by.is_some());

        state.remove_character(42);
        assert!(state.characters.is_empty());
        assert!(state.seats[0].occupied_by.is_none());
    }

    #[test]
    fn find_free_seat_returns_unoccupied() {
        let mut state = OfficeState::from_layout(layout_with_seat());
        assert!(state.find_free_seat().is_some());

        state.add_character(1, 0, None);
        assert!(state.find_free_seat().is_none());
    }

    #[test]
    fn character_by_agent_lookup() {
        let mut state = OfficeState::from_layout(minimal_layout());
        state.add_character(7, 0, None);
        assert!(state.character_by_agent(7).is_some());
        assert!(state.character_by_agent(99).is_none());
    }

    #[test]
    fn find_nearest_free_seat_by_distance() {
        let mut layout = minimal_layout();
        layout
            .furniture
            .push(PlacedFurniture::new("s1", "STUMP_FRONT", 0, 0));
        layout
            .furniture
            .push(PlacedFurniture::new("s2", "STUMP_FRONT", 3, 2));

        let state = OfficeState::from_layout(layout);
        let nearest = state.find_nearest_free_seat((3, 1));
        // (3,2) is closer to (3,1) than (0,0)
        assert_eq!(nearest, Some(1));
    }
}
