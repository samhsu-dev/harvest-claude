use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::types::{Pixel, SpriteData};

/// In-memory RGBA framebuffer for pixel-art compositing.
///
/// All sprites are composited here before terminal output.
/// Each terminal cell maps to two vertical pixels via the `▀` half-block.
#[derive(Debug, Clone)]
pub struct PixelBuffer {
    width: u16,
    height: u16,
    pixels: Vec<Pixel>,
}

impl PixelBuffer {
    /// Allocate a zeroed buffer of the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            pixels: vec![(0, 0, 0, 0); len],
        }
    }

    /// Fill all pixels with a single color.
    pub fn clear(&mut self, color: Pixel) {
        self.pixels.fill(color);
    }

    /// Alpha-composite a sprite at signed coordinates (allows partial off-screen).
    pub fn blit(&mut self, sprite: &SpriteData, x: i16, y: i16) {
        for (sy, row) in sprite.iter().enumerate() {
            let py = y + sy as i16;
            if py < 0 || py >= self.height as i16 {
                continue;
            }
            for (sx, &pixel) in row.iter().enumerate() {
                let px = x + sx as i16;
                if px < 0 || px >= self.width as i16 {
                    continue;
                }
                if pixel.3 == 0 {
                    continue;
                }
                let dst_idx = py as usize * self.width as usize + px as usize;
                self.pixels[dst_idx] = alpha_blend(self.pixels[dst_idx], pixel);
            }
        }
    }

    /// Alpha-composite a horizontally flipped sprite (for LEFT direction).
    pub fn blit_flipped(&mut self, sprite: &SpriteData, x: i16, y: i16) {
        for (sy, row) in sprite.iter().enumerate() {
            let py = y + sy as i16;
            if py < 0 || py >= self.height as i16 {
                continue;
            }
            let row_len = row.len();
            for (sx, &pixel) in row.iter().enumerate() {
                let px = x + (row_len - 1 - sx) as i16;
                if px < 0 || px >= self.width as i16 {
                    continue;
                }
                if pixel.3 == 0 {
                    continue;
                }
                let dst_idx = py as usize * self.width as usize + px as usize;
                self.pixels[dst_idx] = alpha_blend(self.pixels[dst_idx], pixel);
            }
        }
    }

    /// Read a pixel at (x, y). Returns transparent black if out of bounds.
    pub fn get(&self, x: u16, y: u16) -> Pixel {
        if x >= self.width || y >= self.height {
            return (0, 0, 0, 0);
        }
        self.pixels[y as usize * self.width as usize + x as usize]
    }

    /// Write a pixel at (x, y). No-op if out of bounds.
    pub fn set(&mut self, x: u16, y: u16, color: Pixel) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.pixels[y as usize * self.width as usize + x as usize] = color;
    }

    /// Buffer width in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Buffer height in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }
}

/// Alpha-blend `src` over `dst` using standard Porter-Duff src-over.
fn alpha_blend(dst: Pixel, src: Pixel) -> Pixel {
    if src.3 == 255 {
        return src;
    }
    let sa = src.3 as u16;
    let da = 255 - sa;
    let r = (src.0 as u16 * sa + dst.0 as u16 * da) / 255;
    let g = (src.1 as u16 * sa + dst.1 as u16 * da) / 255;
    let b = (src.2 as u16 * sa + dst.2 as u16 * da) / 255;
    let a = sa + (dst.3 as u16 * da) / 255;
    (r as u8, g as u8, b as u8, a.min(255) as u8)
}

/// Convert a `Pixel` to a ratatui `Color`. Transparent pixels become black.
fn pixel_to_color(pixel: Pixel) -> Color {
    Color::Rgb(pixel.0, pixel.1, pixel.2)
}

impl Widget for &PixelBuffer {
    /// Render the pixel buffer to a terminal area.
    ///
    /// Each terminal cell represents two vertical pixels using the `▀` half-block
    /// character with fg = top pixel and bg = bottom pixel.
    fn render(self, area: Rect, buf: &mut Buffer) {
        for ty in 0..area.height {
            let top_y = ty * 2;
            let bot_y = top_y + 1;
            for tx in 0..area.width {
                let top_px = self.get(tx, top_y);
                let bot_px = self.get(tx, bot_y);

                // Skip fully transparent cells
                if top_px.3 == 0 && bot_px.3 == 0 {
                    continue;
                }

                let pos = Position::new(area.x + tx, area.y + ty);
                if let Some(cell) = buf.cell_mut(pos) {
                    cell.set_symbol("\u{2580}"); // ▀
                    cell.set_style(
                        Style::default()
                            .fg(pixel_to_color(top_px))
                            .bg(pixel_to_color(bot_px)),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_transparent() {
        let buf = PixelBuffer::new(4, 4);
        assert_eq!(buf.get(0, 0), (0, 0, 0, 0));
        assert_eq!(buf.get(3, 3), (0, 0, 0, 0));
    }

    #[test]
    fn clear_fills_all_pixels() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.clear((255, 0, 0, 255));
        assert_eq!(buf.get(0, 0), (255, 0, 0, 255));
        assert_eq!(buf.get(1, 1), (255, 0, 0, 255));
    }

    #[test]
    fn set_and_get_roundtrip() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.set(2, 3, (10, 20, 30, 255));
        assert_eq!(buf.get(2, 3), (10, 20, 30, 255));
    }

    #[test]
    fn out_of_bounds_returns_transparent() {
        let buf = PixelBuffer::new(4, 4);
        assert_eq!(buf.get(5, 0), (0, 0, 0, 0));
        assert_eq!(buf.get(0, 5), (0, 0, 0, 0));
    }

    #[test]
    fn blit_opaque_overwrites() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.clear((100, 100, 100, 255));

        let sprite: SpriteData = vec![vec![(255, 0, 0, 255)]];
        buf.blit(&sprite, 1, 1);
        assert_eq!(buf.get(1, 1), (255, 0, 0, 255));
        assert_eq!(buf.get(0, 0), (100, 100, 100, 255));
    }

    #[test]
    fn blit_transparent_no_change() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.clear((50, 50, 50, 255));

        let sprite: SpriteData = vec![vec![(0, 0, 0, 0)]];
        buf.blit(&sprite, 0, 0);
        assert_eq!(buf.get(0, 0), (50, 50, 50, 255));
    }

    #[test]
    fn blit_negative_coords_clips() {
        let mut buf = PixelBuffer::new(4, 4);
        let sprite: SpriteData = vec![
            vec![(255, 0, 0, 255), (0, 255, 0, 255)],
            vec![(0, 0, 255, 255), (255, 255, 0, 255)],
        ];
        buf.blit(&sprite, -1, -1);
        // Only bottom-right pixel (255, 255, 0) should land at (0, 0)
        assert_eq!(buf.get(0, 0), (255, 255, 0, 255));
    }

    #[test]
    fn blit_flipped_mirrors_horizontally() {
        let mut buf = PixelBuffer::new(4, 4);
        let sprite: SpriteData = vec![vec![(255, 0, 0, 255), (0, 255, 0, 255)]];
        buf.blit_flipped(&sprite, 0, 0);
        assert_eq!(buf.get(0, 0), (0, 255, 0, 255));
        assert_eq!(buf.get(1, 0), (255, 0, 0, 255));
    }

    #[test]
    fn dimensions() {
        let buf = PixelBuffer::new(16, 32);
        assert_eq!(buf.width(), 16);
        assert_eq!(buf.height(), 32);
    }

    #[test]
    fn blit_alpha_compositing() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.clear((100, 0, 0, 255)); // opaque red background

        // Semi-transparent green overlay (alpha=128)
        let sprite: SpriteData = vec![vec![(0, 255, 0, 128)]];
        buf.blit(&sprite, 0, 0);

        let result = buf.get(0, 0);
        // Green channel should be blended upward, red channel blended downward
        assert!(
            result.1 > result.0,
            "green channel should dominate after blend"
        );
        assert!(result.3 > 200, "alpha should remain high");
    }

    #[test]
    fn blit_flipped_produces_mirror() {
        let mut buf_normal = PixelBuffer::new(4, 1);
        let mut buf_flipped = PixelBuffer::new(4, 1);

        let sprite: SpriteData = vec![vec![(255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 255)]];

        buf_normal.blit(&sprite, 0, 0);
        buf_flipped.blit_flipped(&sprite, 0, 0);

        // Normal: R G B at x=0,1,2
        assert_eq!(buf_normal.get(0, 0), (255, 0, 0, 255));
        assert_eq!(buf_normal.get(2, 0), (0, 0, 255, 255));

        // Flipped: B G R at x=0,1,2
        assert_eq!(buf_flipped.get(0, 0), (0, 0, 255, 255));
        assert_eq!(buf_flipped.get(2, 0), (255, 0, 0, 255));
    }

    #[test]
    fn blit_out_of_bounds_clips() {
        let mut buf = PixelBuffer::new(4, 4);
        let sprite: SpriteData = vec![vec![(255, 0, 0, 255); 10], vec![(255, 0, 0, 255); 10]];
        // Blit at negative coords — should not panic
        buf.blit(&sprite, -5, -5);
        // Blit extending past right/bottom — should not panic
        buf.blit(&sprite, 3, 3);
    }

    #[test]
    fn clear_fills_all_pixels_comprehensive() {
        let mut buf = PixelBuffer::new(8, 8);
        let color = (42, 128, 200, 255);
        buf.clear(color);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(buf.get(x, y), color, "mismatch at ({x}, {y})");
            }
        }
    }
}
