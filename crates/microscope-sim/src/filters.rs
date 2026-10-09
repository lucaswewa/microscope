//! Image filters on interleaved RGB samples (`f32`, 0–255): a fast Gaussian
//! blur and sensor noise.

use crate::random::Sequence;

/// Blurs the image, `width` pixels wide, about as a Gaussian of `sigma_px`
/// would: three box blurs in each direction, each pass taking the same time
/// whatever the blur's size. Edges repeat their outermost pixels.
pub fn gaussian_blur(samples: &mut [f32], width: usize, height: usize, sigma_px: f64) {
    if sigma_px < 0.3 {
        return;
    }
    let mut scratch = vec![0.0; samples.len()];
    for radius in box_radii(sigma_px) {
        blur_rows(samples, &mut scratch, width, height, radius);
        blur_columns(&scratch, samples, width, height, radius);
    }
}

/// The radii of three box blurs that together approximate a Gaussian of
/// `sigma`: boxes of two odd widths, chosen so the variances add up to σ².
fn box_radii(sigma: f64) -> [usize; 3] {
    let n = 3.0;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut lower = ideal.floor() as i64;
    if lower % 2 == 0 {
        lower -= 1;
    }
    let lower = lower.max(1);
    let l = lower as f64;
    let narrow = ((12.0 * sigma * sigma - n * l * l - 4.0 * n * l - 3.0 * n) / (-4.0 * l - 4.0))
        .round()
        .clamp(0.0, n) as usize;
    std::array::from_fn(|i| {
        let width = if i < narrow { lower } else { lower + 2 };
        (width as usize - 1) / 2
    })
}

/// A box blur of `radius` along each row, from `source` into `target`.
fn blur_rows(source: &[f32], target: &mut [f32], width: usize, height: usize, radius: usize) {
    let scale = 1.0 / (2 * radius + 1) as f32;
    for y in 0..height {
        let row = y * width * 3;
        for c in 0..3 {
            let at = |x: usize| source[row + x.min(width - 1) * 3 + c];
            let mut sum = at(0) * (radius + 1) as f32 + (1..=radius).map(at).sum::<f32>();
            for x in 0..width {
                target[row + x * 3 + c] = sum * scale;
                sum += at(x + radius + 1) - at(x.saturating_sub(radius));
            }
        }
    }
}

/// A box blur of `radius` down each column, from `source` into `target`,
/// a row at a time so memory is read in order.
fn blur_columns(source: &[f32], target: &mut [f32], width: usize, height: usize, radius: usize) {
    let stride = width * 3;
    let scale = 1.0 / (2 * radius + 1) as f32;
    let row = |y: usize| &source[y.min(height - 1) * stride..][..stride];
    let mut sums: Vec<f32> = row(0).iter().map(|v| v * (radius + 1) as f32).collect();
    for y in 1..=radius {
        sums.iter_mut().zip(row(y)).for_each(|(sum, v)| *sum += v);
    }
    for y in 0..height {
        let (entering, leaving) = (row(y + radius + 1), row(y.saturating_sub(radius)));
        let out = &mut target[y * stride..][..stride];
        for i in 0..stride {
            out[i] = sums[i] * scale;
            sums[i] += entering[i] - leaving[i];
        }
    }
}

/// Sensor noise: adds to each sample a normally distributed value of
/// standard deviation `sigma` (grey levels), the same for the same `seed`.
pub fn add_noise(samples: &mut [f32], sigma: f32, seed: u64) {
    let mut random = Sequence::new(seed);
    for sample in samples {
        *sample += random.next_normal() * sigma;
    }
}
