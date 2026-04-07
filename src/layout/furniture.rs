use crate::types::{Direction, SpriteData, TilePos};

/// Runtime furniture instance with resolved sprite and footprint.
#[derive(Debug, Clone)]
pub struct FurnitureInstance {
    /// Unique identifier for this furniture placement.
    pub uid: String,
    /// Furniture catalog type (e.g. "CROP_PLOT", "STUMP_FRONT").
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
    /// Semantic category: "crop", "seating", "decor".
    pub category: Option<String>,
    /// True if this is a horizontally mirrored variant.
    pub mirrored: bool,
}

/// Relative tile offsets `(dcol, drow)` for a furniture type's footprint.
///
/// Unknown types return a single-tile `[(0, 0)]` footprint.
pub fn furniture_footprint(kind: &str) -> Vec<(i16, i16)> {
    match kind {
        // Crop plots: 1x1 each
        "CROP_PLOT" | "CROP_PLOT_ON" => vec![(0, 0)],
        // Stumps (seats): 1x1
        "STUMP_FRONT" | "STUMP_BACK" | "STUMP_LEFT" | "STUMP_RIGHT" => vec![(0, 0)],
        // Fishing spot: 1x1
        "FISHING_SPOT" => vec![(0, 0)],
        // Trees: 1x1 footprint (sprite extends above)
        "TREE" | "TREE_FRUIT" => vec![(0, 0)],
        // Wall segments: 1x1
        "CABIN_WALL" | "BARN_WALL" => vec![(0, 0)],
        // Home: 1x1
        "HOME" => vec![(0, 0)],
        // Animal pens: 1x1
        "CHICKEN_COOP" | "COW_PEN" => vec![(0, 0)],
        // Structures
        "WELL" => vec![(0, 0)],
        "MAILBOX" | "MAILBOX_ON" => vec![(0, 0)],
        "SCARECROW" => vec![(0, 0)],
        "LANTERN" | "FLOWER" | "BUSH" => vec![(0, 0)],
        "FENCE_H" | "FENCE_V" => vec![(0, 0)],
        // Unknown
        _ => vec![(0, 0)],
    }
}

/// True if this furniture type is placed on a surface rather than on the ground.
pub fn is_surface_item(kind: &str) -> bool {
    matches!(kind, "LANTERN")
}

/// True if this furniture type activates when an agent faces it.
pub fn is_electronics(kind: &str) -> bool {
    matches!(
        kind,
        "CROP_PLOT" | "CROP_PLOT_ON" | "MAILBOX" | "MAILBOX_ON"
    )
}

/// True if this furniture type is a seat an agent can occupy.
pub(crate) fn is_seat(kind: &str) -> bool {
    matches!(
        kind,
        "STUMP_FRONT" | "STUMP_BACK" | "STUMP_LEFT" | "STUMP_RIGHT" | "FISHING_SPOT"
    )
}

/// Resolve the facing direction for a furniture type, if applicable.
pub(crate) fn furniture_facing(kind: &str) -> Option<Direction> {
    match kind {
        "STUMP_FRONT" => Some(Direction::Down),
        "STUMP_BACK" => Some(Direction::Up),
        "STUMP_LEFT" => Some(Direction::Left),
        "STUMP_RIGHT" => Some(Direction::Right),
        "FISHING_SPOT" => Some(Direction::Down),
        _ => None,
    }
}

/// Resolve the semantic category for a furniture type.
pub(crate) fn furniture_category(kind: &str) -> Option<String> {
    match kind {
        "CROP_PLOT" | "CROP_PLOT_ON" => Some("desk".to_owned()),
        "STUMP_FRONT" | "STUMP_BACK" | "STUMP_LEFT" | "STUMP_RIGHT" | "FISHING_SPOT" => {
            Some("seating".to_owned())
        }
        "MAILBOX" | "MAILBOX_ON" => Some("electronics".to_owned()),
        "TREE" | "TREE_FRUIT" | "WELL" | "SCARECROW" | "LANTERN" | "CABIN_WALL" | "BARN_WALL"
        | "FENCE_H" | "FENCE_V" | "FLOWER" | "BUSH" | "CHICKEN_COOP" | "COW_PEN" => {
            Some("decor".to_owned())
        }
        "HOME" => Some("building".to_owned()),
        _ => None,
    }
}

/// True if this is a mirrored variant that should flip the sprite.
pub(crate) fn is_mirrored(kind: &str) -> bool {
    matches!(kind, "STUMP_LEFT")
}

/// True if this furniture type is a crop plot (work area).
pub(crate) fn is_desk(kind: &str) -> bool {
    matches!(kind, "CROP_PLOT" | "CROP_PLOT_ON")
}

#[cfg(test)]
mod tests {
    use super::{
        furniture_category, furniture_facing, furniture_footprint, is_electronics, is_mirrored,
        is_seat, is_surface_item,
    };
    use crate::types::Direction;

    #[test]
    fn crop_plot_footprint_is_1x1() {
        let fp = furniture_footprint("CROP_PLOT");
        assert_eq!(fp, vec![(0, 0)]);
    }

    #[test]
    fn unknown_type_gets_default_footprint() {
        let fp = furniture_footprint("UNKNOWN_THING");
        assert_eq!(fp, vec![(0, 0)]);
    }

    #[test]
    fn crop_plot_is_electronics() {
        assert!(is_electronics("CROP_PLOT"));
        assert!(is_electronics("CROP_PLOT_ON"));
    }

    #[test]
    fn stump_is_seat_not_surface() {
        assert!(is_seat("STUMP_FRONT"));
        assert!(!is_surface_item("STUMP_FRONT"));
    }

    #[test]
    fn stump_front_faces_down() {
        assert_eq!(furniture_facing("STUMP_FRONT"), Some(Direction::Down));
    }

    #[test]
    fn stump_left_is_mirrored() {
        assert!(is_mirrored("STUMP_LEFT"));
        assert!(!is_mirrored("STUMP_RIGHT"));
    }

    #[test]
    fn category_resolution() {
        assert_eq!(
            furniture_category("MAILBOX"),
            Some("electronics".to_owned())
        );
        assert_eq!(furniture_category("TREE"), Some("decor".to_owned()));
        assert_eq!(furniture_category("RANDOM"), None);
    }

    #[test]
    fn crop_is_not_seat() {
        assert!(!is_seat("CROP_PLOT"));
        assert!(!is_seat("CROP_PLOT_ON"));
    }

    #[test]
    fn mailbox_is_electronics() {
        assert!(is_electronics("MAILBOX"));
        assert!(is_electronics("MAILBOX_ON"));
        assert!(!is_electronics("TREE"));
        assert!(!is_electronics("WELL"));
    }

    #[test]
    fn unknown_furniture_has_default_footprint() {
        assert_eq!(furniture_footprint("TOTALLY_UNKNOWN"), vec![(0, 0)]);
        assert_eq!(furniture_footprint(""), vec![(0, 0)]);
        assert_eq!(furniture_footprint("RANDOM_TYPE_XYZ"), vec![(0, 0)]);
    }

    #[test]
    fn fishing_spot_is_seat() {
        assert!(is_seat("FISHING_SPOT"));
        assert_eq!(furniture_facing("FISHING_SPOT"), Some(Direction::Down));
    }
}
