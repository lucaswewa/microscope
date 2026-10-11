//! Phase correlation (P22) on synthetic images whose shifts are known
//! exactly: whole and fractional pixels, noise, shrinking, and the
//! ambiguity of periodic patterns.

use microscope_core::registration::{Gray, Options, Shift, register};

/// A number in [0, 1) for a grid point, from a fixed sequence.
fn hash(x: i64, y: i64, seed: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth noise: random values on a grid of `cell`-pixel squares, blended
/// smoothly between them, so it can be drawn at any fraction of a pixel.
fn noise(x: f64, y: f64, cell: f64, seed: u64) -> f64 {
    let (fx, fy) = (x / cell, y / cell);
    let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
    let smooth = |t: f64| t * t * (3.0 - 2.0 * t);
    let (tx, ty) = (smooth(fx - fx.floor()), smooth(fy - fy.floor()));
    let row = |y: i64| hash(ix, y, seed) * (1.0 - tx) + hash(ix + 1, y, seed) * tx;
    row(iy) * (1.0 - ty) + row(iy + 1) * ty
}

/// A textured image, its content moved by (`dx`, `dy`) pixels.
fn texture(size: usize, seed: u64, dx: f64, dy: f64) -> Gray {
    Gray::from_fn(size, size, |x, y| {
        let (x, y) = (x as f64 - dx, y as f64 - dy);
        (100.0 * noise(x, y, 9.0, seed) + 50.0 * noise(x, y, 4.0, seed + 1)) as f32
    })
}

const WHOLE: Options = Options {
    downsample: 1,
    max_size: 256,
};

fn assert_near(shift: Shift, dx: f64, dy: f64, within: f64) {
    let error = (shift.dx - dx).hypot(shift.dy - dy);
    assert!(
        error <= within,
        "{shift:?}: {error:.3} px from ({dx}, {dy})"
    );
}

#[test]
fn finds_whole_pixel_shifts_with_confidence() {
    let original = texture(256, 1, 0.0, 0.0);
    for (dx, dy) in [(0.0, 0.0), (10.0, -7.0), (-25.0, 31.0), (60.0, 45.0)] {
        let shift = register(&original, &texture(256, 1, dx, dy), &WHOLE);
        assert_near(shift, dx, dy, 0.1);
        assert!(shift.confidence > 0.5, "{shift:?}");
    }
}

#[test]
fn finds_fractions_of_a_pixel() {
    let original = texture(256, 2, 0.0, 0.0);
    for (dx, dy) in [(3.3, -1.75), (-0.5, 0.25), (12.8, 7.6)] {
        assert_near(
            register(&original, &texture(256, 2, dx, dy), &WHOLE),
            dx,
            dy,
            0.12,
        );
    }
}

#[test]
fn copes_with_noise() {
    let noisy = |dx: f64, dy: f64, seed: u64| {
        let mut image = texture(256, 3, dx, dy);
        for (i, value) in image.pixels.iter_mut().enumerate() {
            *value += (60.0 * (hash(i as i64, 0, seed) - 0.5)) as f32;
        }
        image
    };
    let shift = register(&noisy(0.0, 0.0, 10), &noisy(-14.2, 9.4, 11), &WHOLE);
    assert_near(shift, -14.2, 9.4, 0.3);
    assert!(shift.confidence > 0.2, "{shift:?}");
}

#[test]
fn shrinking_trades_precision_for_speed() {
    let options = Options {
        downsample: 4,
        max_size: 256,
    };
    let shift = register(
        &texture(512, 4, 0.0, 0.0),
        &texture(512, 4, 37.0, -22.0),
        &options,
    );
    // Within half a shrunk pixel.
    assert_near(shift, 37.0, -22.0, 2.0);
}

#[test]
fn periodic_patterns_are_ambiguous_and_say_so() {
    // Stripes 16 pixels apart: a shift of 20 looks the same as one of 4, or
    // −12, so only the shift modulo 16 can be known, and confidence is low.
    let stripes = |dx: f64| {
        Gray::from_fn(256, 256, |x, _| {
            (100.0 * (2.0 * std::f64::consts::PI * (x as f64 - dx) / 16.0).cos()) as f32
        })
    };
    let shift = register(&stripes(0.0), &stripes(20.0), &WHOLE);
    let modulo = (shift.dx - 20.0).rem_euclid(16.0);
    assert!(modulo.min(16.0 - modulo) < 0.2, "{shift:?}");
    assert!(shift.confidence < 0.2, "{shift:?}");
}

#[test]
fn unrelated_images_get_low_confidence() {
    let shift = register(
        &texture(256, 5, 0.0, 0.0),
        &texture(256, 6, 0.0, 0.0),
        &WHOLE,
    );
    assert!(shift.confidence < 0.3, "{shift:?}");
}

#[test]
fn reads_rgb_as_luminance_and_shrinks_by_blocks() {
    let image = Gray::from_rgb8(2, 1, &[255, 0, 0, 0, 0, 255]);
    assert_eq!(image.pixels, [0.299 * 255.0, 0.114 * 255.0]);
    let blocks = Gray::from_fn(5, 4, |x, y| (x + 10 * y) as f32).downsample(2);
    assert_eq!((blocks.width, blocks.height), (2, 2));
    assert_eq!(blocks.pixels, [5.5, 7.5, 25.5, 27.5]);
}
