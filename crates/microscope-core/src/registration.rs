//! Image registration by phase correlation (ADR-0023): how far one image's
//! content has moved in another, to a fraction of a pixel.
//!
//! Both images are shrunk (optionally), cut to the same centred
//! power-of-two square, and tapered towards their edges with a Hann window.
//! The normalised cross-power spectrum of their Fourier transforms keeps only
//! the phase difference, which is the shift. Its inverse transform is a
//! sharp peak at the shift. The peak's neighbours refine it to a fraction of
//! a pixel, and the next highest peak says how sure it is.

use std::f64::consts::PI;

use crate::fft::{Complex, fft2};

/// A greyscale image, stored by rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Gray {
    /// Its width, in pixels.
    pub width: usize,
    /// Its height, in pixels.
    pub height: usize,
    /// Its pixels' brightness, in any unit.
    pub pixels: Vec<f32>,
}

impl Gray {
    /// An image whose pixel `(x, y)` is `f(x, y)`.
    pub fn from_fn(width: usize, height: usize, f: impl Fn(usize, usize) -> f32) -> Self {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y));
        Self {
            width,
            height,
            pixels: pixels.collect(),
        }
    }

    /// The luminance of 8-bit RGB pixels, three bytes a pixel, stored by
    /// rows, as `image::RgbImage` holds them.
    pub fn from_rgb8(width: usize, height: usize, rgb: &[u8]) -> Self {
        assert_eq!(rgb.len(), width * height * 3, "three bytes a pixel");
        let (pixels, _) = rgb.as_chunks::<3>();
        let pixels = pixels
            .iter()
            .map(|[r, g, b]| 0.299 * f32::from(*r) + 0.587 * f32::from(*g) + 0.114 * f32::from(*b));
        Self {
            width,
            height,
            pixels: pixels.collect(),
        }
    }

    fn at(&self, x: usize, y: usize) -> f32 {
        self.pixels[y * self.width + x]
    }

    /// The `width` × `height` part of the image whose top-left corner is
    /// `(x, y)`, which must be within it.
    pub fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> Gray {
        assert!(
            x + width <= self.width && y + height <= self.height,
            "within the image"
        );
        Gray::from_fn(width, height, |i, j| self.at(x + i, y + j))
    }

    /// Shrunk by a whole `factor`: each pixel is the mean of a block.
    /// Pixels left over at the right and bottom edges are dropped.
    pub fn downsample(&self, factor: usize) -> Gray {
        let factor = factor.max(1);
        let block = (factor * factor) as f32;
        Gray::from_fn(self.width / factor, self.height / factor, |x, y| {
            let mut sum = 0.0;
            for dy in 0..factor {
                for dx in 0..factor {
                    sum += self.at(x * factor + dx, y * factor + dy);
                }
            }
            sum / block
        })
    }
}

/// How images are registered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Shrink both images by this factor first, for speed; 1 to keep them.
    pub downsample: usize,
    /// The largest square compared, after shrinking: a power of two.
    pub max_size: usize,
}

impl Default for Options {
    /// Halved, then at most 256 × 256: a camera's 820 × 616 preview takes a
    /// few milliseconds.
    fn default() -> Self {
        Self {
            downsample: 2,
            max_size: 256,
        }
    }
}

/// How far an image's content moved in another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shift {
    /// Rightwards, in the original images' pixels.
    pub dx: f64,
    /// Downwards, in the original images' pixels.
    pub dy: f64,
    /// How sure: 1 for one clear peak, falling towards 0 as another peak
    /// rises as high, as it does for a periodic pattern, two unrelated
    /// images, or noise.
    pub confidence: f64,
}

/// How far `b`'s content has moved from where it is in `a`: `b(x, y)` is
/// about `a(x − dx, y − dy)`.
///
/// The images must be the same size. A shift is found modulo the compared
/// square, so it must be less than half of it (128 pixels after shrinking,
/// by default), and the images must overlap well within it.
pub fn register(a: &Gray, b: &Gray, options: &Options) -> Shift {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "the images must be the same size"
    );
    let factor = options.downsample.max(1);
    let (a, b) = (a.downsample(factor), b.downsample(factor));
    let n = largest_power_of_two(a.width.min(a.height).min(options.max_size));
    assert!(n >= 8, "the images are too small to register");
    let (mut fa, mut fb) = (windowed(&a, n), windowed(&b, n));
    fft2(&mut fa, n, false);
    fft2(&mut fb, n, false);
    let mut cross: Vec<Complex> = fa
        .iter()
        .zip(&fb)
        .map(|(a, b)| {
            let product = a.conj() * *b;
            product.scale(1.0 / (product.norm() + 1e-12))
        })
        .collect();
    fft2(&mut cross, n, true);
    let surface: Vec<f64> = cross
        .iter()
        .map(|value| value.re / (n * n) as f64)
        .collect();

    let (peak, &height) = surface
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("a non-empty surface");
    let (px, py) = (peak % n, peak / n);
    let value = |x: isize, y: isize| {
        let wrap = |i: isize| i.rem_euclid(n as isize) as usize;
        surface[wrap(y) * n + wrap(x)]
    };
    let (x, y) = (px as isize, py as isize);
    let dx = refine(value(x - 1, y), height, value(x + 1, y));
    let dy = refine(value(x, y - 1), height, value(x, y + 1));

    // The highest point away from the peak.
    let second = (0..n * n)
        .filter(|i| {
            let near = |a: usize, b: usize| {
                let d = a.abs_diff(b);
                d.min(n - d) <= 2
            };
            !(near(i % n, px) && near(i / n, py))
        })
        .map(|i| surface[i])
        .fold(f64::NEG_INFINITY, f64::max);
    let confidence = if height > 0.0 {
        ((height - second.max(0.0)) / height).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let unwrap = |i: usize| {
        if i >= n / 2 {
            i as f64 - n as f64
        } else {
            i as f64
        }
    };
    Shift {
        dx: (unwrap(px) + dx) * factor as f64,
        dy: (unwrap(py) + dy) * factor as f64,
        confidence,
    }
}

fn largest_power_of_two(limit: usize) -> usize {
    if limit == 0 {
        0
    } else {
        1 << (usize::BITS - 1 - limit.leading_zeros())
    }
}

/// The image's centred `n` × `n` square, less its mean, tapered to zero at
/// the edges by a Hann window, so that the edges' jump doesn't swamp the
/// image's own detail.
fn windowed(image: &Gray, n: usize) -> Vec<Complex> {
    let (left, top) = ((image.width - n) / 2, (image.height - n) / 2);
    let square: Vec<f64> = (0..n * n)
        .map(|i| f64::from(image.at(left + i % n, top + i / n)))
        .collect();
    let mean = square.iter().sum::<f64>() / square.len() as f64;
    let hann: Vec<f64> = (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos())
        .collect();
    square
        .iter()
        .enumerate()
        .map(|(i, value)| Complex::new((value - mean) * hann[i % n] * hann[i / n], 0.0))
        .collect()
}

/// Where the peak really is, between −½ and ½ of a pixel from the highest
/// point: the top of the parabola through it and its two neighbours. Over a
/// range of synthetic shifts this is within 0.05 pixels on average, and
/// 0.09 at worst (ADR-0023).
fn refine(before: f64, peak: f64, after: f64) -> f64 {
    let curvature = before - 2.0 * peak + after;
    if curvature.abs() < 1e-12 {
        0.0
    } else {
        (0.5 * (before - after) / curvature).clamp(-0.5, 0.5)
    }
}
