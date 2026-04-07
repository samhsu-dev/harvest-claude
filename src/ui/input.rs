use crate::constants::TILE_SIZE;
use crate::engine::state::OfficeState;
use crate::types::TilePos;

/// Convert terminal cell coordinates to tile position.
///
/// Half-block aware: each terminal row represents 2 pixel rows.
/// `offset_x` and `offset_y` are the terminal cell coordinates of
/// the render area origin (main_area.x, main_area.y).
pub fn tile_from_cell(col: u16, row: u16, offset_x: u16, offset_y: u16) -> TilePos {
    let pixel_x = col.saturating_sub(offset_x);
    let pixel_y = row.saturating_sub(offset_y).saturating_mul(2);
    let tile_col = pixel_x / TILE_SIZE;
    let tile_row = pixel_y / TILE_SIZE;
    (tile_col, tile_row)
}

/// Hit-test a mouse click against characters in the office.
///
/// Returns the agent_id of the character at the clicked tile, or `None`.
pub fn hit_test_character(
    state: &OfficeState,
    col: u16,
    row: u16,
    offset_x: u16,
    offset_y: u16,
) -> Option<usize> {
    let tile = tile_from_cell(col, row, offset_x, offset_y);
    let idx = state.character_at_tile(tile)?;
    Some(state.characters[idx].agent_id)
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
        // pixel_x = 16 - 8 = 8, pixel_y = (4 - 0) * 2 = 8
        // tile_col = 8 / 8 = 1, tile_row = 8 / 8 = 1
        let (tx, ty) = tile_from_cell(16, 4, 8, 0);
        assert_eq!(tx, 1);
        assert_eq!(ty, 1);
    }

    #[test]
    fn tile_from_cell_with_y_offset() {
        // col=0, row=6, offset_x=0, offset_y=2
        // pixel_x = 0, pixel_y = (6 - 2) * 2 = 8
        // tile_col = 0, tile_row = 8 / 8 = 1
        let (tx, ty) = tile_from_cell(0, 6, 0, 2);
        assert_eq!(tx, 0);
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
