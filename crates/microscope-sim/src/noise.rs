//! Smooth noise, for the tissue specimen and the focal surface: value noise
//! on a grid of hashed corners, and fractal sums of it that leave out what
//! a pixel is too coarse to show.

use crate::random::mix;

/// Smooth noise in [0, 1] at `(x, y)`, in grid cells: random values at the
/// corners of the cell it's in, blended with a smooth step, so it's
/// continuous, and so is its slope.
pub fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let corner = |dx: i64, dy: i64| {
        let (cx, cy) = (x0 as i64 + dx, y0 as i64 + dy);
        let key = seed
            ^ (cx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (cy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        (mix(key) >> 11) as f64 / (1u64 << 53) as f64
    };
    let (fx, fy) = (smooth(x - x0), smooth(y - y0));
    let top = lerp(corner(0, 0), corner(1, 0), fx);
    let bottom = lerp(corner(0, 1), corner(1, 1), fx);
    lerp(top, bottom, fy)
}

/// Fractal noise in [0, 1] at `(x_um, y_um)`: `octaves` layers of value
/// noise, the first with features about `scale_um` across, and each next one
/// half the size and half the weight. A layer whose features are smaller
/// than two pixels of `um_per_px` counts as its average, 0.5: that's what
/// pixels that size would show of it.
pub fn fractal(
    seed: u64,
    x_um: f64,
    y_um: f64,
    scale_um: f64,
    octaves: u32,
    um_per_px: f64,
) -> f64 {
    let (mut sum, mut total, mut size, mut weight) = (0.0, 0.0, scale_um, 1.0);
    for octave in 0..octaves {
        let layer = mix(seed ^ u64::from(octave));
        let value = if size >= 2.0 * um_per_px {
            value_noise(layer, x_um / size, y_um / size)
        } else {
            0.5
        };
        sum += weight * value;
        total += weight;
        size /= 2.0;
        weight /= 2.0;
    }
    sum / total
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_noise_is_smooth_and_hits_its_corners() {
        let at = |x: f64, y: f64| value_noise(7, x, y);
        // Continuous across the corner of four cells, too.
        assert!((at(3.0, -2.0) - at(3.0 - 1e-9, -2.0 - 1e-9)).abs() < 1e-6);
        for (x, y) in [(0.3, 0.7), (-4.2, 9.9), (12.5, -0.01)] {
            let v = at(x, y);
            assert!((0.0..=1.0).contains(&v));
            assert!(
                (at(x + 1e-6, y) - v).abs() < 1e-4,
                "continuous at ({x}, {y})"
            );
        }
    }

    #[test]
    fn fractal_noise_leaves_out_layers_finer_than_the_pixels() {
        // Pixels coarser than every layer see only the average.
        assert_eq!(fractal(3, 10.0, 20.0, 50.0, 4, 100.0), 0.5);
        // Pixels finer than every layer see them all.
        let fine = fractal(3, 10.0, 20.0, 50.0, 4, 0.1);
        assert_ne!(fine, 0.5);
        assert!((0.0..=1.0).contains(&fine));
    }
}
