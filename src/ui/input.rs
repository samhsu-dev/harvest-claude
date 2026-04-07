use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::constants::TILE_SIZE;
use crate::engine::state::OfficeState;
use crate::types::TilePos;

/// Convert terminal cell coordinates to tile position.
///
/// Half-block aware: each terminal row represents 2 pixel rows,
/// so `pixel_y = row * 2`. The offset accounts for the pixel buffer's
/// position within the terminal viewport.
pub fn tile_from_cell(col: u16, row: u16, offset_x: u16, offset_y: u16) -> TilePos {
    let pixel_x = col.saturating_sub(offset_x);
    let pixel_y = row.saturating_mul(2).saturating_sub(offset_y);
    let tile_col = pixel_x / TILE_SIZE;
    let tile_row = pixel_y / TILE_SIZE;
    (tile_col, tile_row)
}

/// Hit-test a mouse click against characters in the office.
///
/// Converts raw terminal coordinates to tile position, then checks if any
/// character occupies that tile. Returns an `Action::Key` to select the
/// character or dismiss its bubble, or `None` if no character was hit.
pub fn handle_mouse_click(
    state: &OfficeState,
    col: u16,
    row: u16,
    offset_x: u16,
    offset_y: u16,
) -> Option<Action> {
    let tile = tile_from_cell(col, row, offset_x, offset_y);
    let agent_idx = state.character_at_tile(tile)?;

    let character = state.characters.get(agent_idx)?;

    // If the character has a bubble, clicking dismisses it
    if character.bubble.is_some() {
        return Some(Action::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    }

    // Select the character via right-arrow key action
    Some(Action::Key(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::NONE,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_from_cell_origin() {
        let (tx, ty) = tile_from_cell(0, 0, 0, 0);
        assert_eq!(tx, 0);
        assert_eq!(ty, 0);
    }

    #[test]
    fn tile_from_cell_with_offset() {
        // col=16, row=4, offset_x=8, offset_y=0
        // pixel_x = 16 - 8 = 8, pixel_y = 4 * 2 - 0 = 8
        // tile_col = 8 / 8 = 1, tile_row = 8 / 8 = 1
        let (tx, ty) = tile_from_cell(16, 4, 8, 0);
        assert_eq!(tx, 1);
        assert_eq!(ty, 1);
    }

    #[test]
    fn tile_from_cell_half_block() {
        // row=1 → pixel_y=2, tile_row = 2/8 = 0 (still tile row 0)
        let (_, ty) = tile_from_cell(0, 1, 0, 0);
        assert_eq!(ty, 0);
        // row=4 → pixel_y=8, tile_row = 8/8 = 1
        let (_, ty) = tile_from_cell(0, 4, 0, 0);
        assert_eq!(ty, 1);
    }

    #[test]
    fn tile_from_cell_saturates_on_underflow() {
        // offset larger than col/row should saturate to 0
        let (tx, ty) = tile_from_cell(5, 2, 100, 100);
        assert_eq!(tx, 0);
        assert_eq!(ty, 0);
    }
}
