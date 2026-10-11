//! A procedural tissue specimen, like a section stained with haematoxylin
//! and eosin: pink stroma streaked with fibres, purple nuclei, and gaps where
//! there's no tissue. Like the blobs, it's drawn from hashes of the place, so
//! any region draws the same, and detail finer than a pixel averages out.

use image::RgbImage;

use crate::blobs::{draw, poisson};
use crate::noise::{fractal, value_noise};
use crate::random::{hash, mix, unit};
use crate::specimen::Specimen;
use crate::units::Point;

/// The size of the tissue's patches, in µm: the largest gaps are about this.
const PATCH_UM: f64 = 600.0;
/// How far the tissue's edge fades, in the mask's units.
const EDGE: f64 = 0.01;
/// The side of the cells that nuclei are scattered by, in µm.
const CELL_UM: f64 = 30.0;
/// How far apart the tissue's mask is sampled, in µm: a tenth of its
/// finest features.
const MASK_STEP_UM: f64 = 4.0;
/// How far apart the stroma's fibres are sampled, in µm: a third of their
/// finest. Their finest that a pixel shows is coarser at low magnification,
/// so they're sampled at least every one and a half pixels.
const FIBRE_STEP_UM: f64 = 0.5;
/// The nuclei's smallest and largest radius, in µm.
const NUCLEUS_UM: (f64, f64) = (2.0, 3.5);
/// The share of each channel's light that eosin, in the stroma, lets through.
const EOSIN: [f32; 3] = [0.93, 0.55, 0.75];
/// The share that haematoxylin, in the nuclei, lets through.
const HAEMATOXYLIN: [f32; 3] = [0.35, 0.25, 0.62];

/// What a [`TissueSpecimen`] looks like.
#[derive(Debug, Clone, PartialEq)]
pub struct TissueConfig {
    /// The same seed draws the same tissue.
    pub seed: u64,
    /// The share of the slide the tissue covers, from 0 to 1.
    pub coverage: f64,
    /// How many nuclei, on average, per mm² of tissue.
    pub nuclei_per_mm2: f64,
    /// The colour of the empty slide.
    pub background: [u8; 3],
    /// The section's width and height, in µm, centred on the origin.
    pub size_um: f64,
}

impl Default for TissueConfig {
    /// Tissue over 70% of a 20 × 20 mm section, with 3000 nuclei per mm².
    fn default() -> Self {
        Self {
            seed: 1,
            coverage: 0.7,
            nuclei_per_mm2: 3000.0,
            background: [246, 244, 240],
            size_um: 20_000.0,
        }
    }
}

/// A section of tissue (see the module's description).
#[derive(Debug, Clone)]
pub struct TissueSpecimen {
    config: TissueConfig,
    /// Where the mask turns to tissue: its quantile at 1 − coverage.
    threshold: f64,
}

impl TissueSpecimen {
    /// The specimen `config` describes.
    pub fn new(config: TissueConfig) -> Self {
        // The quantile, from samples farther apart than the patches.
        let mut samples: Vec<f64> = (0..64 * 64)
            .map(|i| {
                let (x, y) = (f64::from(i % 64) * 997.0, f64::from(i / 64) * 991.0);
                fractal(config.seed, x, y, PATCH_UM, 5, 0.0)
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        let threshold = match config.coverage {
            c if c <= 0.0 => f64::INFINITY,
            c if c >= 1.0 => f64::NEG_INFINITY,
            c => samples[((1.0 - c) * (samples.len() - 1) as f64).round() as usize],
        };
        Self { config, threshold }
    }

    /// The tissue's mask at `(x, y)`, as pixels of `um_per_px` see it: it's
    /// tissue where the mask is above the threshold.
    fn mask(&self, x: f64, y: f64, um_per_px: f64) -> f64 {
        fractal(self.config.seed, x, y, PATCH_UM, 5, um_per_px)
    }

    /// How much of the point `(x, y)` is tissue, from 0 to 1, where the mask
    /// is `mask`.
    fn tissue(&self, x: f64, y: f64, mask: f64) -> f64 {
        let half = self.config.size_um / 2.0;
        if x.abs() > half || y.abs() > half {
            return 0.0;
        }
        ((mask - self.threshold) / EDGE).clamp(0.0, 1.0)
    }

    /// How densely the stroma is stained at `(x, y)`: fibres, streaked
    /// along a direction that wanders across the section.
    fn stroma(&self, x: f64, y: f64, um_per_px: f64) -> f64 {
        let seed = self.config.seed;
        let angle = std::f64::consts::TAU * value_noise(mix(seed ^ 2), x / 400.0, y / 400.0);
        let (sin, cos) = angle.sin_cos();
        let (along, across) = (x * cos + y * sin, y * cos - x * sin);
        0.35 + 0.65 * fractal(mix(seed ^ 3), along / 4.0, across, 6.0, 3, um_per_px)
    }

    /// The nuclei in cell `(cx, cy)`, in tissue: their centres and radii.
    fn nuclei_in(&self, cx: i64, cy: i64, nuclei: &mut Vec<(Point, f64)>) {
        let cell = hash(&[self.config.seed, 7, cx as u64, cy as u64]);
        let per_cell = self.config.nuclei_per_mm2 * CELL_UM * CELL_UM / 1e6;
        for i in 0..poisson(per_cell, unit(mix(cell))) {
            let nucleus = hash(&[cell, i + 1]);
            let centre = Point::new(
                (cx as f64 + unit(mix(nucleus ^ 1))) * CELL_UM,
                (cy as f64 + unit(mix(nucleus ^ 2))) * CELL_UM,
            );
            if self.tissue(centre.x, centre.y, self.mask(centre.x, centre.y, 0.0)) >= 1.0 {
                let (low, high) = NUCLEUS_UM;
                nuclei.push((centre, low + (high - low) * unit(mix(nucleus ^ 3))));
            }
        }
    }
}

impl Specimen for TissueSpecimen {
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage {
        let (w, h) = (width as usize, height as usize);
        let region = (origin, um_per_px, w, h);
        let mask = sampled(region, MASK_STEP_UM, |x, y| self.mask(x, y, um_per_px));
        let stroma = sampled(region, FIBRE_STEP_UM.max(1.5 * um_per_px), |x, y| {
            self.stroma(x, y, um_per_px)
        });
        // The share of the light that reaches each pixel, per channel.
        let mut light = vec![1.0f32; w * h * 3];
        for j in 0..h {
            let y = origin.y + (j as f64 + 0.5) * um_per_px;
            for i in 0..w {
                let x = origin.x + (i as f64 + 0.5) * um_per_px;
                let tissue = self.tissue(x, y, mask[j * w + i]);
                if tissue > 0.0 {
                    let stain = (tissue * stroma[j * w + i]) as f32;
                    let p = (j * w + i) * 3;
                    for c in 0..3 {
                        light[p + c] = 1.0 - stain * (1.0 - EOSIN[c]);
                    }
                }
            }
        }

        let reach = NUCLEUS_UM.1;
        let cells = |start: f64, pixels: u32| {
            let end = start + f64::from(pixels) * um_per_px;
            ((start - reach) / CELL_UM).floor() as i64..=((end + reach) / CELL_UM).floor() as i64
        };
        let mut nuclei = Vec::new();
        for cy in cells(origin.y, height) {
            for cx in cells(origin.x, width) {
                self.nuclei_in(cx, cy, &mut nuclei);
            }
        }
        for (centre, radius) in nuclei {
            let at = (
                (centre.x - origin.x) / um_per_px,
                (centre.y - origin.y) / um_per_px,
            );
            draw(&mut light, w, h, at, radius / um_per_px, HAEMATOXYLIN);
        }

        let background = self.config.background.map(f32::from);
        RgbImage::from_fn(width, height, |x, y| {
            let p = (y as usize * w + x as usize) * 3;
            image::Rgb(std::array::from_fn(|c| {
                (background[c] * light[p + c]).round() as u8
            }))
        })
    }
}

/// `field` at each pixel of a region (its origin, µm per pixel, width and
/// height): sampled on a grid every `step_um` and interpolated between, or
/// at each pixel if the grid would be finer than the pixels. The grid is
/// fixed to the slide, so every region sees the same values.
fn sampled(
    (origin, um_per_px, w, h): (Point, f64, usize, usize),
    step_um: f64,
    field: impl Fn(f64, f64) -> f64,
) -> Vec<f64> {
    let at = |i: usize, start: f64| start + (i as f64 + 0.5) * um_per_px;
    if step_um <= um_per_px || w == 0 || h == 0 {
        return (0..w * h)
            .map(|k| field(at(k % w, origin.x), at(k / w, origin.y)))
            .collect();
    }
    let node = |v: f64| (v / step_um).floor() as i64;
    let (x0, y0) = (node(at(0, origin.x)), node(at(0, origin.y)));
    let cols = (node(at(w - 1, origin.x)) - x0 + 2) as usize;
    let rows = (node(at(h - 1, origin.y)) - y0 + 2) as usize;
    let grid: Vec<f64> = (0..rows * cols)
        .map(|k| {
            let (c, r) = ((k % cols) as i64 + x0, (k / cols) as i64 + y0);
            field(c as f64 * step_um, r as f64 * step_um)
        })
        .collect();
    let mut values = Vec::with_capacity(w * h);
    for j in 0..h {
        // The node before the pixel, as `node` finds it, so it's on the grid.
        let v = at(j, origin.y) / step_um;
        let (r, fy) = ((v.floor() as i64 - y0) as usize, v - v.floor());
        for i in 0..w {
            let u = at(i, origin.x) / step_um;
            let (c, fx) = ((u.floor() as i64 - x0) as usize, u - u.floor());
            let node = |dc: usize, dr: usize| grid[(r + dr) * cols + c + dc];
            let top = node(0, 0) + (node(1, 0) - node(0, 0)) * fx;
            let bottom = node(0, 1) + (node(1, 1) - node(0, 1)) * fx;
            values.push(top + (bottom - top) * fy);
        }
    }
    values
}
