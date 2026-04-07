use crate::layout::furniture::FurnitureInstance;
use crate::types::{AnimType, Direction, TilePos, TileType};

/// A seat derived from chair furniture, assignable to an agent.
#[derive(Debug, Clone)]
pub struct Seat {
    /// Grid column of the seat tile.
    pub col: u16,
    /// Grid row of the seat tile.
    pub row: u16,
    /// Direction the seated agent faces.
    pub facing: Direction,
    /// Agent ID occupying this seat, if any.
    pub occupied_by: Option<usize>,
    /// UID of the furniture instance this seat belongs to.
    pub furniture_uid: String,
    /// Animation override when an agent works at this seat.
    pub work_anim: Option<AnimType>,
}

impl Seat {
    /// Tile position of this seat.
    pub fn tile_pos(&self) -> TilePos {
        (self.col, self.row)
    }
}

/// Extract seats from chair furniture instances.
///
/// Multi-tile chairs produce one seat per footprint tile. UIDs are
/// `uid` for the first tile, `uid:1`, `uid:2`, etc. for subsequent tiles.
pub fn derive_seats(furniture: &[FurnitureInstance], tile_map: &[Vec<TileType>]) -> Vec<Seat> {
    let mut seats = Vec::new();

    for inst in furniture.iter().filter(|f| f.is_seat) {
        for (i, &tile_pos) in inst.footprint.iter().enumerate() {
            let (col, row) = tile_pos;
            // Skip footprint tiles outside the map
            if (row as usize) >= tile_map.len()
                || (col as usize) >= tile_map.first().map_or(0, |r| r.len())
            {
                continue;
            }

            let uid = if i == 0 {
                inst.uid.clone()
            } else {
                format!("{}:{}", inst.uid, i)
            };

            let facing = inst
                .facing
                .unwrap_or_else(|| facing_from_context(tile_pos, furniture));

            let work_anim = work_anim_for_seat(&inst.furniture_type, tile_pos, facing, furniture);

            seats.push(Seat {
                col,
                row,
                facing,
                occupied_by: None,
                furniture_uid: uid,
                work_anim,
            });
        }
    }

    seats
}

/// Determine the work animation for a seat based on its furniture type and nearby furniture.
///
/// FISHING_SPOT → Fish, seats facing CROP_PLOT → Farm, seats facing TREE_FRUIT → Harvest.
fn work_anim_for_seat(
    furniture_type: &str,
    pos: TilePos,
    facing: Direction,
    furniture: &[FurnitureInstance],
) -> Option<AnimType> {
    if furniture_type == "FISHING_SPOT" {
        return Some(AnimType::Fish);
    }

    // Check tiles in the facing direction for crop plots or fruit trees
    let (dc, dr) = match facing {
        Direction::Up => (0i32, -1i32),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    };

    for depth in 1..=3 {
        let col = pos.0 as i32 + dc * depth;
        let row = pos.1 as i32 + dr * depth;
        if col < 0 || row < 0 {
            break;
        }
        let check = (col as u16, row as u16);

        for furn in furniture {
            if !furn.footprint.contains(&check) {
                continue;
            }
            match furn.furniture_type.as_str() {
                "CROP_PLOT" | "CROP_PLOT_ON" => return Some(AnimType::Farm),
                "TREE_FRUIT" => return Some(AnimType::Harvest),
                _ => {}
            }
        }
    }

    None
}

/// Determine facing direction from context: chair orientation, adjacent desk, or default Down.
pub fn facing_from_context(pos: TilePos, furniture: &[FurnitureInstance]) -> Direction {
    // Check if any desk is adjacent; face toward it
    let (col, row) = pos;
    let col_i = col as i32;
    let row_i = row as i32;

    let directions = [
        (Direction::Up, (col_i, row_i - 1)),
        (Direction::Down, (col_i, row_i + 1)),
        (Direction::Left, (col_i - 1, row_i)),
        (Direction::Right, (col_i + 1, row_i)),
    ];

    for (dir, (nc, nr)) in &directions {
        if *nc < 0 || *nr < 0 {
            continue;
        }
        let neighbor = (*nc as u16, *nr as u16);
        let has_desk = furniture
            .iter()
            .any(|f| f.category.as_deref() == Some("desk") && f.footprint.contains(&neighbor));
        if has_desk {
            return *dir;
        }
    }

    Direction::Down
}

#[cfg(test)]
mod tests {
    use super::{derive_seats, facing_from_context, work_anim_for_seat};
    use crate::layout::furniture::FurnitureInstance;
    use crate::types::{AnimType, Direction, TileType};

    fn make_chair(uid: &str, col: i16, row: i16, facing: Direction) -> FurnitureInstance {
        FurnitureInstance {
            uid: uid.to_owned(),
            furniture_type: "WOODEN_CHAIR_FRONT".to_owned(),
            col,
            row,
            sprite: vec![],
            footprint: vec![(col as u16, row as u16)],
            z_y: 0.0,
            is_seat: true,
            facing: Some(facing),
            category: Some("seating".to_owned()),
            mirrored: false,
        }
    }

    fn make_desk(uid: &str, col: i16, row: i16) -> FurnitureInstance {
        FurnitureInstance {
            uid: uid.to_owned(),
            furniture_type: "DESK_FRONT".to_owned(),
            col,
            row,
            sprite: vec![],
            footprint: vec![(col as u16, row as u16), ((col + 1) as u16, row as u16)],
            z_y: 0.0,
            is_seat: false,
            facing: Some(Direction::Down),
            category: Some("desk".to_owned()),
            mirrored: false,
        }
    }

    #[test]
    fn derive_single_chair() {
        let furniture = vec![make_chair("c1", 3, 4, Direction::Down)];
        let tile_map = vec![vec![TileType::Grass; 10]; 10];
        let seats = derive_seats(&furniture, &tile_map);
        assert_eq!(seats.len(), 1);
        assert_eq!(seats[0].col, 3);
        assert_eq!(seats[0].row, 4);
        assert_eq!(seats[0].facing, Direction::Down);
    }

    #[test]
    fn facing_from_adjacent_desk() {
        let desk = make_desk("d1", 3, 5);
        let furniture = vec![desk];
        let facing = facing_from_context((3, 4), &furniture);
        assert_eq!(facing, Direction::Down);
    }

    #[test]
    fn facing_defaults_to_down() {
        let facing = facing_from_context((5, 5), &[]);
        assert_eq!(facing, Direction::Down);
    }

    #[test]
    fn fishing_spot_has_fish_anim() {
        let anim = work_anim_for_seat("FISHING_SPOT", (5, 5), Direction::Down, &[]);
        assert_eq!(anim, Some(AnimType::Fish));
    }

    #[test]
    fn seat_facing_crop_has_farm_anim() {
        let crop = FurnitureInstance {
            uid: "crop".to_owned(),
            furniture_type: "CROP_PLOT".to_owned(),
            col: 5,
            row: 6,
            sprite: vec![],
            footprint: vec![(5, 6)],
            z_y: 0.0,
            is_seat: false,
            facing: None,
            category: Some("desk".to_owned()),
            mirrored: false,
        };
        let anim = work_anim_for_seat("STUMP_FRONT", (5, 5), Direction::Down, &[crop]);
        assert_eq!(anim, Some(AnimType::Farm));
    }

    #[test]
    fn seat_facing_fruit_tree_has_harvest_anim() {
        let tree = FurnitureInstance {
            uid: "tree".to_owned(),
            furniture_type: "TREE_FRUIT".to_owned(),
            col: 5,
            row: 4,
            sprite: vec![],
            footprint: vec![(5, 4)],
            z_y: 0.0,
            is_seat: false,
            facing: None,
            category: Some("decor".to_owned()),
            mirrored: false,
        };
        let anim = work_anim_for_seat("STUMP_FRONT", (5, 5), Direction::Up, &[tree]);
        assert_eq!(anim, Some(AnimType::Harvest));
    }
}
