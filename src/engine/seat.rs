use crate::layout::furniture::FurnitureInstance;
use crate::types::{Direction, TilePos, TileType};

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

            seats.push(Seat {
                col,
                row,
                facing,
                occupied_by: None,
                furniture_uid: uid,
            });
        }
    }

    seats
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
    use super::{derive_seats, facing_from_context};
    use crate::layout::furniture::FurnitureInstance;
    use crate::types::{Direction, TileType};

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
}
