use crate::constants::MATRIX_DURATION_SECS;
use crate::render::buffer::PixelBuffer;

/// Per-column state for the matrix rain effect.
#[derive(Debug, Clone)]
pub struct MatrixColumn {
    /// Pixel x-offset within the sprite region.
    pub x: u16,
    /// Random time offset for staggered column starts (0..30% of duration).
    pub offset: f32,
    /// Pre-generated rain characters as RGB triples.
    pub chars: Vec<(u8, u8, u8)>,
}

/// Green rain spawn/despawn visual effect over `MATRIX_DURATION_SECS`.
#[derive(Debug, Clone)]
pub struct MatrixEffect {
    /// True = spawn (reveal), false = despawn (consume).
    pub spawning: bool,
    /// Elapsed time in seconds.
    pub elapsed: f32,
    /// Per-column rain data.
    pub columns: Vec<MatrixColumn>,
}

impl MatrixEffect {
    /// Create a spawn (reveal) effect for a region of the given pixel dimensions.
    pub fn new_spawn(width: u16, height: u16) -> Self {
        Self::new_inner(true, width, height)
    }

    /// Create a despawn (consume) effect for a region of the given pixel dimensions.
    pub fn new_despawn(width: u16, height: u16) -> Self {
        Self::new_inner(false, width, height)
    }

    fn new_inner(spawning: bool, width: u16, height: u16) -> Self {
        let columns = (0..width)
            .map(|x| {
                // Deterministic stagger based on column index
                let offset = (pseudo_hash(x as u32, 0) % 30) as f32 / 100.0 * MATRIX_DURATION_SECS;
                let trail_len = height as usize + 6;
                let chars = (0..trail_len).map(|i| rain_color(x, i as u16)).collect();
                MatrixColumn { x, offset, chars }
            })
            .collect();

        Self {
            spawning,
            elapsed: 0.0,
            columns,
        }
    }

    /// Advance the effect by `dt` seconds. Returns `true` when the effect is complete.
    pub fn update(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.elapsed >= MATRIX_DURATION_SECS
    }

    /// Composite the matrix rain onto a pixel buffer at the given offset.
    pub fn apply(&self, buf: &mut PixelBuffer, origin_x: i16, origin_y: i16) {
        let buf_w = buf.width();
        let buf_h = buf.height();

        for col in &self.columns {
            let px_x = origin_x + col.x as i16;
            if px_x < 0 || px_x >= buf_w as i16 {
                continue;
            }

            let effective_time = (self.elapsed - col.offset).max(0.0);
            let progress = (effective_time / MATRIX_DURATION_SECS).clamp(0.0, 1.0);

            let sprite_h = col.chars.len() as f32;
            let sweep_pos = progress * sprite_h;

            for (i, &(r, g, b)) in col.chars.iter().enumerate() {
                let row_f = i as f32;

                // Determine visibility based on spawn/despawn direction
                let visible = if self.spawning {
                    row_f < sweep_pos
                } else {
                    row_f >= sweep_pos
                };

                if !visible {
                    continue;
                }

                // Distance from sweep head determines brightness
                let dist = if self.spawning {
                    sweep_pos - row_f
                } else {
                    row_f - sweep_pos
                };

                let alpha = trail_alpha(dist, sprite_h);
                if alpha == 0 {
                    continue;
                }

                // Hash-based 30fps flicker: ~70% visibility
                let frame_hash = pseudo_hash(col.x as u32 + i as u32 * 97, self.elapsed as u32);
                if frame_hash % 10 < 3 {
                    continue;
                }

                let py = origin_y + i as i16;
                if py < 0 || py >= buf_h as i16 {
                    continue;
                }

                buf.set(px_x as u16, py as u16, (r, g, b, alpha));
            }
        }
    }
}

// Generate a green-tinted rain color for a given column and row.
fn rain_color(col: u16, row: u16) -> (u8, u8, u8) {
    let h = pseudo_hash(col as u32, row as u32);
    let brightness = 100 + (h % 156) as u8; // 100..255
    // Head-like bright green: #ccffcc range
    (brightness / 3, brightness, brightness / 3)
}

// Determine alpha from distance to sweep head.
// Head = bright (255), trail fades over 6 rows in 3 brightness tiers.
fn trail_alpha(dist: f32, _sprite_h: f32) -> u8 {
    if dist < 1.0 {
        // Head: bright green
        255
    } else if dist < 3.0 {
        // Near trail: 66% threshold
        200
    } else if dist < 5.0 {
        // Mid trail: 33% threshold
        140
    } else if dist < 7.0 {
        // Far trail
        80
    } else {
        0
    }
}

// Simple deterministic hash for stagger and flicker.
fn pseudo_hash(a: u32, b: u32) -> u32 {
    let mut x = a.wrapping_mul(2654435761);
    x ^= b.wrapping_mul(2246822519);
    x ^= x >> 16;
    x.wrapping_mul(0x45d9f3b) ^ (x >> 13)
}

#[cfg(test)]
mod tests {
    use super::MatrixEffect;

    #[test]
    fn spawn_completes_after_duration() {
        let mut fx = MatrixEffect::new_spawn(8, 16);
        assert!(!fx.update(0.1));
        assert!(fx.update(0.3));
    }

    #[test]
    fn despawn_completes_after_duration() {
        let mut fx = MatrixEffect::new_despawn(8, 16);
        assert!(!fx.update(0.1));
        assert!(fx.update(0.3));
    }

    #[test]
    fn columns_match_width() {
        let fx = MatrixEffect::new_spawn(12, 8);
        assert_eq!(fx.columns.len(), 12);
    }

    #[test]
    fn column_chars_include_trail() {
        let fx = MatrixEffect::new_spawn(4, 10);
        // Each column has height + 6 trail rows
        assert_eq!(fx.columns[0].chars.len(), 16);
    }
}
