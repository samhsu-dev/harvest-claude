use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Grid primitives
// ---------------------------------------------------------------------------

/// RGBA pixel: (red, green, blue, alpha).
pub type Pixel = (u8, u8, u8, u8);

/// 2D sprite: rows of pixels. `sprite[y][x]` = pixel at (x, y).
pub type SpriteData = Vec<Vec<Pixel>>;

/// Tile coordinate on the office grid: (col, row).
pub type TilePos = (u16, u16);

// ---------------------------------------------------------------------------
// Tile types
// ---------------------------------------------------------------------------

/// Tile type stored in the office grid. `repr(u8)` matches the serialized format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TileType {
    Void = 0,
    Floor1 = 1,
    Floor2 = 2,
    Floor3 = 3,
    Floor4 = 4,
    Floor5 = 5,
    Floor6 = 6,
    Floor7 = 7,
    Wall = 100,
}

impl TileType {
    /// Convert a raw byte to a tile type.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Void,
            1 => Self::Floor1,
            2 => Self::Floor2,
            3 => Self::Floor3,
            4 => Self::Floor4,
            5 => Self::Floor5,
            6 => Self::Floor6,
            7 => Self::Floor7,
            100 => Self::Wall,
            // Legacy VOID value and unknown tiles map to Void
            _ => Self::Void,
        }
    }

    /// Returns true if this tile is a walkable floor type.
    pub fn is_floor(self) -> bool {
        matches!(
            self,
            Self::Floor1
                | Self::Floor2
                | Self::Floor3
                | Self::Floor4
                | Self::Floor5
                | Self::Floor6
                | Self::Floor7
        )
    }
}

// ---------------------------------------------------------------------------
// Direction and state enums
// ---------------------------------------------------------------------------

/// Cardinal direction for character facing and movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Down,
    Up,
    Left,
    Right,
}

/// Character finite state machine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharState {
    Idle,
    Walk,
    Type,
}

/// Animation type for sprite lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimType {
    Walk,
    Type,
    Read,
}

/// Agent status inferred from JSONL parsing and heuristic timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AgentStatus {
    Active,
    Idle,
    Waiting,
    Permission,
}

/// Speech bubble kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleKind {
    Permission,
    Waiting,
}

// ---------------------------------------------------------------------------
// Agent events (produced by JSONL parser, consumed by App)
// ---------------------------------------------------------------------------

/// Events extracted from JSONL records by the watcher parser.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AgentEvent {
    ToolStart {
        tool_id: String,
        tool_name: String,
    },
    ToolDone {
        tool_id: String,
    },
    TurnEnd,
    TextOnly,
    SubAgentToolStart {
        parent_tool_id: String,
        tool_id: String,
        tool_name: String,
    },
    SubAgentToolDone {
        parent_tool_id: String,
        tool_id: String,
    },
    SubAgentSpawn {
        parent_tool_id: String,
    },
    BashProgress {
        tool_id: String,
    },
    BackgroundAgentDetected {
        tool_id: String,
    },
}

// ---------------------------------------------------------------------------
// Layout serialization types
// ---------------------------------------------------------------------------

/// HSL-based tile color for colorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileColor {
    pub h: f32,
    pub s: f32,
    pub b: f32,
}

/// Furniture placement in the serialized layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedFurniture {
    pub uid: String,
    #[serde(rename = "type")]
    pub furniture_type: String,
    pub col: i16,
    pub row: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<TileColor>,
}

/// Serializable office layout. Compatible with the VS Code extension format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeLayout {
    pub version: u32,
    pub cols: u16,
    pub rows: u16,
    pub tiles: Vec<u8>,
    pub furniture: Vec<PlacedFurniture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_colors: Option<HashMap<String, TileColor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_revision: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_type_from_u8_known_values() {
        assert_eq!(TileType::from_u8(0), TileType::Void);
        assert_eq!(TileType::from_u8(1), TileType::Floor1);
        assert_eq!(TileType::from_u8(2), TileType::Floor2);
        assert_eq!(TileType::from_u8(3), TileType::Floor3);
        assert_eq!(TileType::from_u8(4), TileType::Floor4);
        assert_eq!(TileType::from_u8(5), TileType::Floor5);
        assert_eq!(TileType::from_u8(6), TileType::Floor6);
        assert_eq!(TileType::from_u8(7), TileType::Floor7);
        assert_eq!(TileType::from_u8(100), TileType::Wall);
    }

    #[test]
    fn tile_type_from_u8_unknown_maps_to_void() {
        assert_eq!(TileType::from_u8(8), TileType::Void);
        assert_eq!(TileType::from_u8(50), TileType::Void);
        assert_eq!(TileType::from_u8(255), TileType::Void);
    }

    #[test]
    fn tile_type_is_floor() {
        assert!(TileType::Floor1.is_floor());
        assert!(TileType::Floor2.is_floor());
        assert!(TileType::Floor3.is_floor());
        assert!(TileType::Floor4.is_floor());
        assert!(TileType::Floor5.is_floor());
        assert!(TileType::Floor6.is_floor());
        assert!(TileType::Floor7.is_floor());
        assert!(!TileType::Void.is_floor());
        assert!(!TileType::Wall.is_floor());
    }

    #[test]
    fn tile_type_legacy_void_maps_correctly() {
        // Value 8 was the legacy void in older layouts
        assert_eq!(TileType::from_u8(8), TileType::Void);
    }
}
