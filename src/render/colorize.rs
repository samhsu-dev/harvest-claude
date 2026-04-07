use crate::types::{Pixel, SpriteData, TileColor};

/// Convert RGB (0-255) to HSL. Returns (h: 0..360, s: 0..1, l: 0..1).
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta < f32::EPSILON {
        return (0.0, 0.0, l);
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let h = if (max - rf).abs() < f32::EPSILON {
        let segment = (gf - bf) / delta;
        if segment < 0.0 {
            segment + 6.0
        } else {
            segment
        }
    } else if (max - gf).abs() < f32::EPSILON {
        (bf - rf) / delta + 2.0
    } else {
        (rf - gf) / delta + 4.0
    };

    (h * 60.0, s, l)
}

/// Convert HSL to RGB. Input: h: 0..360, s: 0..1, l: 0..1.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s < f32::EPSILON {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let h_norm = ((h % 360.0) + 360.0) % 360.0;

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hk = h_norm / 360.0;

    let r = hue_to_rgb(p, q, hk + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, hk);
    let b = hue_to_rgb(p, q, hk - 1.0 / 3.0);

    (
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, t_raw: f32) -> f32 {
    let t = if t_raw < 0.0 {
        t_raw + 1.0
    } else if t_raw > 1.0 {
        t_raw - 1.0
    } else {
        t_raw
    };

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// Photoshop-style colorization: grayscale input mapped to fixed HSL output.
///
/// Used for floor and wall tiles. Perceived luminance formula:
/// `L = (0.299*R + 0.587*G + 0.114*B) / 255`. Alpha preserved.
pub fn colorize_sprite(sprite: &SpriteData, color: &TileColor) -> SpriteData {
    sprite
        .iter()
        .map(|row| {
            row.iter()
                .map(|&pixel| colorize_pixel(pixel, color))
                .collect()
        })
        .collect()
}

fn colorize_pixel(pixel: Pixel, color: &TileColor) -> Pixel {
    if pixel.3 == 0 {
        return pixel;
    }

    // Perceived luminance
    let lum = (0.299 * pixel.0 as f32 + 0.587 * pixel.1 as f32 + 0.114 * pixel.2 as f32) / 255.0;

    // Contrast adjustment: c = color.s * 100 (reuse s field for contrast)
    let c = color.s * 100.0;
    let l_contrast = 0.5 + (lum - 0.5) * ((100.0 + c) / 100.0);

    // Brightness adjustment
    let l_final = (l_contrast + color.b / 200.0).clamp(0.0, 1.0);

    // Apply target hue and saturation
    let target_s = (color.s).clamp(0.0, 1.0);
    let (r, g, b) = hsl_to_rgb(color.h, target_s, l_final);

    (r, g, b, pixel.3)
}

/// HSL shift mode: shifts original pixel HSL values.
///
/// Used for furniture and character hue shifts. Alpha preserved.
pub fn adjust_sprite(sprite: &SpriteData, color: &TileColor) -> SpriteData {
    sprite
        .iter()
        .map(|row| {
            row.iter()
                .map(|&pixel| adjust_pixel(pixel, color))
                .collect()
        })
        .collect()
}

fn adjust_pixel(pixel: Pixel, color: &TileColor) -> Pixel {
    if pixel.3 == 0 {
        return pixel;
    }

    let (h, s, l) = rgb_to_hsl(pixel.0, pixel.1, pixel.2);

    // Hue rotation with wraparound
    let new_h = ((h + color.h) % 360.0 + 360.0) % 360.0;

    // Saturation shift (clamped)
    let new_s = (s + color.s).clamp(0.0, 1.0);

    // Brightness: contrast then brightness shift
    let c = color.s * 100.0;
    let l_contrast = 0.5 + (l - 0.5) * ((100.0 + c) / 100.0);
    let new_l = (l_contrast + color.b / 200.0).clamp(0.0, 1.0);

    let (r, g, b) = hsl_to_rgb(new_h, new_s, new_l);
    (r, g, b, pixel.3)
}

/// Rotate the hue of a single pixel by the given degrees.
///
/// Only hue is affected; saturation and lightness remain unchanged. Alpha preserved.
pub fn adjust_hue(pixel: Pixel, degrees: i16) -> Pixel {
    if pixel.3 == 0 {
        return pixel;
    }

    let (h, s, l) = rgb_to_hsl(pixel.0, pixel.1, pixel.2);
    let new_h = ((h + degrees as f32) % 360.0 + 360.0) % 360.0;
    let (r, g, b) = hsl_to_rgb(new_h, s, l);
    (r, g, b, pixel.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_hsl_pure_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);
    }

    #[test]
    fn rgb_to_hsl_white() {
        let (_, s, l) = rgb_to_hsl(255, 255, 255);
        assert!(s < 0.01);
        assert!((l - 1.0).abs() < 0.01);
    }

    #[test]
    fn rgb_to_hsl_black() {
        let (_, s, l) = rgb_to_hsl(0, 0, 0);
        assert!(s < 0.01);
        assert!(l < 0.01);
    }

    #[test]
    fn hsl_roundtrip() {
        let original = (180u8, 100u8, 50u8);
        let (h, s, l) = rgb_to_hsl(original.0, original.1, original.2);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r as i16 - original.0 as i16).abs() <= 1);
        assert!((g as i16 - original.1 as i16).abs() <= 1);
        assert!((b as i16 - original.2 as i16).abs() <= 1);
    }

    #[test]
    fn colorize_preserves_alpha() {
        let sprite = vec![vec![(128, 128, 128, 100)]];
        let color = TileColor {
            h: 200.0,
            s: 0.5,
            b: 0.0,
        };
        let result = colorize_sprite(&sprite, &color);
        assert_eq!(result[0][0].3, 100);
    }

    #[test]
    fn colorize_skips_transparent() {
        let sprite = vec![vec![(0, 0, 0, 0)]];
        let color = TileColor {
            h: 100.0,
            s: 1.0,
            b: 0.0,
        };
        let result = colorize_sprite(&sprite, &color);
        assert_eq!(result[0][0], (0, 0, 0, 0));
    }

    #[test]
    fn adjust_hue_wraps() {
        let pixel = (255, 0, 0, 255); // red, hue ~0
        let shifted = adjust_hue(pixel, 120);
        // Should be roughly green
        assert!(shifted.1 > shifted.0);
    }

    #[test]
    fn adjust_hue_preserves_transparent() {
        let pixel = (0, 0, 0, 0);
        let shifted = adjust_hue(pixel, 90);
        assert_eq!(shifted, (0, 0, 0, 0));
    }

    #[test]
    fn adjust_sprite_preserves_alpha() {
        let sprite = vec![vec![(200, 100, 50, 180)]];
        let color = TileColor {
            h: 30.0,
            s: 0.0,
            b: 0.0,
        };
        let result = adjust_sprite(&sprite, &color);
        assert_eq!(result[0][0].3, 180);
    }

    #[test]
    fn rgb_to_hsl_and_back_roundtrip() {
        // Test several distinct colors
        let colors: Vec<(u8, u8, u8)> =
            vec![(200, 100, 50), (0, 128, 255), (255, 255, 0), (64, 64, 64)];
        for (r, g, b) in colors {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!(
                (r as i16 - r2 as i16).abs() <= 1
                    && (g as i16 - g2 as i16).abs() <= 1
                    && (b as i16 - b2 as i16).abs() <= 1,
                "roundtrip mismatch for ({r},{g},{b}) -> ({r2},{g2},{b2})"
            );
        }
    }

    #[test]
    fn colorize_sprite_preserves_alpha() {
        let sprite = vec![vec![(100, 100, 100, 77), (200, 50, 50, 200)]];
        let color = TileColor {
            h: 120.0,
            s: 0.5,
            b: 0.0,
        };
        let result = colorize_sprite(&sprite, &color);
        assert_eq!(result[0][0].3, 77);
        assert_eq!(result[0][1].3, 200);
    }

    #[test]
    fn adjust_hue_wraps_around() {
        let pixel = (255, 0, 0, 255); // red, hue ~0
        // Shift by 480 degrees — should wrap to 120 (green range)
        let shifted = adjust_hue(pixel, 480);
        let (h, _, _) = rgb_to_hsl(shifted.0, shifted.1, shifted.2);
        assert!((h - 120.0).abs() < 5.0, "hue should wrap to ~120, got {h}");
    }

    #[test]
    fn grayscale_input_colorize() {
        let sprite = vec![vec![(128, 128, 128, 255)]];
        let color = TileColor {
            h: 200.0,
            s: 0.8,
            b: 0.0,
        };
        let result = colorize_sprite(&sprite, &color);
        let (h, s, _) = rgb_to_hsl(result[0][0].0, result[0][0].1, result[0][0].2);
        assert!(
            (h - 200.0).abs() < 5.0,
            "output hue should match target ~200, got {h}"
        );
        assert!(s > 0.1, "output should have some saturation, got {s}");
    }
}
