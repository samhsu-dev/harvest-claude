use crate::types::{Direction, SpriteData, TilePos};

/// Runtime furniture instance with resolved sprite and footprint.
#[derive(Debug, Clone)]
pub struct FurnitureInstance {
    /// Unique identifier for this furniture placement.
    pub uid: String,
    /// Furniture catalog type (e.g. "DESK_FRONT", "MONITOR").
    pub furniture_type: String,
    /// Grid column position.
    pub col: i16,
    /// Grid row position.
    pub row: i16,
    /// Rendered sprite data for this furniture.
    pub sprite: SpriteData,
    /// Absolute tile positions this furniture occupies.
    pub footprint: Vec<TilePos>,
    /// Z-sort value derived from row position and sprite height.
    pub z_y: f32,
    /// True if an agent can sit in this furniture.
    pub is_seat: bool,
    /// Direction this furniture faces, if applicable.
    pub facing: Option<Direction>,
    /// Semantic category: "electronics", "desk", "decor", "seating".
    pub category: Option<String>,
    /// True if this is a horizontally mirrored variant (LEFT flipped from RIGHT).
    pub mirrored: bool,
}

/// Relative tile offsets `(dcol, drow)` for a furniture type's footprint.
///
/// Unknown types return a single-tile `[(0, 0)]` footprint.
pub fn furniture_footprint(kind: &str) -> Vec<(i16, i16)> {
    match kind {
        // Desks: FRONT is 2x1 (two columns), LEFT/RIGHT are 1x2 (two rows)
        "DESK_FRONT" => vec![(0, 0), (1, 0)],
        "DESK_LEFT" => vec![(0, 0), (0, 1)],
        "DESK_RIGHT" => vec![(0, 0), (0, 1)],
        // Chairs: all 1x1
        "WOODEN_CHAIR_FRONT" | "WOODEN_CHAIR_BACK" | "WOODEN_CHAIR_LEFT" | "WOODEN_CHAIR_RIGHT" => {
            vec![(0, 0)]
        }
        // Electronics: 1x1 surface items
        "MONITOR" | "LAPTOP" => vec![(0, 0)],
        // Decor
        "LAMP" | "PLANT" => vec![(0, 0)],
        "BOOKSHELF" => vec![(0, 0), (1, 0)],
        // Unknown
        _ => vec![(0, 0)],
    }
}

/// True if this furniture type is placed on a desk surface rather than on the floor.
pub fn is_surface_item(kind: &str) -> bool {
    matches!(kind, "MONITOR" | "LAPTOP" | "LAMP")
}

/// True if this furniture type activates (switches ON) when an agent faces it.
pub fn is_electronics(kind: &str) -> bool {
    matches!(kind, "MONITOR" | "LAPTOP")
}

/// True if this furniture type is a seat an agent can occupy.
pub(crate) fn is_seat(kind: &str) -> bool {
    matches!(
        kind,
        "WOODEN_CHAIR_FRONT" | "WOODEN_CHAIR_BACK" | "WOODEN_CHAIR_LEFT" | "WOODEN_CHAIR_RIGHT"
    )
}

/// Resolve the facing direction for a furniture type, if applicable.
pub(crate) fn furniture_facing(kind: &str) -> Option<Direction> {
    match kind {
        "WOODEN_CHAIR_FRONT" | "DESK_FRONT" => Some(Direction::Down),
        "WOODEN_CHAIR_BACK" => Some(Direction::Up),
        "WOODEN_CHAIR_LEFT" | "DESK_LEFT" => Some(Direction::Left),
        "WOODEN_CHAIR_RIGHT" | "DESK_RIGHT" => Some(Direction::Right),
        _ => None,
    }
}

/// Resolve the semantic category for a furniture type.
pub(crate) fn furniture_category(kind: &str) -> Option<String> {
    match kind {
        "DESK_FRONT" | "DESK_LEFT" | "DESK_RIGHT" => Some("desk".to_owned()),
        "WOODEN_CHAIR_FRONT" | "WOODEN_CHAIR_BACK" | "WOODEN_CHAIR_LEFT" | "WOODEN_CHAIR_RIGHT" => {
            Some("seating".to_owned())
        }
        "MONITOR" | "LAPTOP" => Some("electronics".to_owned()),
        "LAMP" | "PLANT" | "BOOKSHELF" => Some("decor".to_owned()),
        _ => None,
    }
}

/// True if this is a mirrored (LEFT) variant that should flip the RIGHT sprite.
pub(crate) fn is_mirrored(kind: &str) -> bool {
    matches!(kind, "WOODEN_CHAIR_LEFT" | "DESK_LEFT")
}

/// True if this furniture type is a desk.
pub(crate) fn is_desk(kind: &str) -> bool {
    matches!(kind, "DESK_FRONT" | "DESK_LEFT" | "DESK_RIGHT")
}

#[cfg(test)]
mod tests {
    use super::{
        furniture_category, furniture_facing, furniture_footprint, is_electronics, is_mirrored,
        is_seat, is_surface_item,
    };
    use crate::types::Direction;

    #[test]
    fn desk_front_footprint_is_2x1() {
        let fp = furniture_footprint("DESK_FRONT");
        assert_eq!(fp, vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn unknown_type_gets_default_footprint() {
        let fp = furniture_footprint("UNKNOWN_THING");
        assert_eq!(fp, vec![(0, 0)]);
    }

    #[test]
    fn monitor_is_surface_and_electronics() {
        assert!(is_surface_item("MONITOR"));
        assert!(is_electronics("MONITOR"));
    }

    #[test]
    fn chair_is_seat_not_surface() {
        assert!(is_seat("WOODEN_CHAIR_FRONT"));
        assert!(!is_surface_item("WOODEN_CHAIR_FRONT"));
    }

    #[test]
    fn chair_front_faces_down() {
        assert_eq!(
            furniture_facing("WOODEN_CHAIR_FRONT"),
            Some(Direction::Down)
        );
    }

    #[test]
    fn desk_left_is_mirrored() {
        assert!(is_mirrored("DESK_LEFT"));
        assert!(!is_mirrored("DESK_RIGHT"));
    }

    #[test]
    fn category_resolution() {
        assert_eq!(
            furniture_category("MONITOR"),
            Some("electronics".to_owned())
        );
        assert_eq!(furniture_category("PLANT"), Some("decor".to_owned()));
        assert_eq!(furniture_category("RANDOM"), None);
    }

    #[test]
    fn desk_is_not_seat() {
        assert!(!is_seat("DESK_FRONT"));
        assert!(!is_seat("DESK_LEFT"));
        assert!(!is_seat("DESK_RIGHT"));
    }

    #[test]
    fn monitor_is_electronics() {
        assert!(is_electronics("MONITOR"));
        assert!(is_electronics("LAPTOP"));
        assert!(!is_electronics("DESK_FRONT"));
        assert!(!is_electronics("PLANT"));
    }

    #[test]
    fn unknown_furniture_has_default_footprint() {
        assert_eq!(furniture_footprint("TOTALLY_UNKNOWN"), vec![(0, 0)]);
        assert_eq!(furniture_footprint(""), vec![(0, 0)]);
        assert_eq!(furniture_footprint("RANDOM_TYPE_XYZ"), vec![(0, 0)]);
    }
}
