//! A procedural specimen of coloured blobs, drawn by our own generator.
//!
//! The plane is divided into square cells. Each cell's blobs (how many,
//! where, how big, which colour) come from a hash of the seed and the cell,
//! so any region draws the same blobs without storing them. Blobs absorb
//! light, as stained cells do in bright field: where they overlap, the
//! light they let through multiplies.

use image::RgbImage;

use crate::random::{hash, mix, unit};
use crate::specimen::{Extent, Specimen};
use crate::units::Point;

/// What a [`BlobSpecimen`] looks like.
#[derive(Debug, Clone, PartialEq)]
pub struct BlobConfig {
    /// The same seed draws the same blobs.
    pub seed: u64,
    /// How many blobs, on average, per mm².
    pub density_per_mm2: f64,
    /// The smallest and largest radius, in µm.
    pub radius_um: (f64, f64),
    /// The colours of the light that blobs let through; each blob takes one.
    pub colours: Vec<[u8; 3]>,
    /// The colour of the empty slide.
    pub background: [u8; 3],
    /// How far the specimen extends.
    pub extent: Extent,
}

impl Default for BlobConfig {
    /// Purple, pink and blue blobs 4–16 µm across, on a 20 × 20 mm sample.
    fn default() -> Self {
        Self {
            seed: 1,
            density_per_mm2: 1500.0,
            radius_um: (2.0, 8.0),
            colours: vec![
                [150, 90, 170],
                [210, 120, 160],
                [110, 120, 190],
                [190, 150, 200],
            ],
            background: [246, 244, 240],
            extent: Extent::Finite {
                width_um: 20_000.0,
                height_um: 20_000.0,
            },
        }
    }
}

/// One blob, in µm, with the share of each channel's light it lets through.
#[derive(Debug, Clone, Copy)]
struct Blob {
    centre: Point,
    radius: f64,
    transmits: [f32; 3],
}

/// Coloured blobs scattered over the sample (see the module's description).
#[derive(Debug, Clone)]
pub struct BlobSpecimen {
    config: BlobConfig,
    /// The side of a cell, in µm: four of the largest radius, so a blob only
    /// reaches the cells next to its own.
    cell_um: f64,
    /// The average number of blobs per cell.
    per_cell: f64,
    /// For a repeating specimen, its period in cells.
    period_cells: Option<i64>,
}

impl BlobSpecimen {
    /// The specimen `config` describes.
    pub fn new(config: BlobConfig) -> Self {
        let cell_um = (4.0 * config.radius_um.1).max(1.0);
        let per_cell = config.density_per_mm2 * cell_um * cell_um / 1e6;
        let period_cells = match config.extent {
            Extent::Repeating { period_um } => Some(((period_um / cell_um).round() as i64).max(1)),
            Extent::Finite { .. } => None,
        };
        Self {
            config,
            cell_um,
            per_cell,
            period_cells,
        }
    }

    /// For a repeating specimen, its actual period in µm: a whole number of cells.
    pub fn period_um(&self) -> Option<f64> {
        self.period_cells.map(|cells| cells as f64 * self.cell_um)
    }

    /// The blobs whose centres are in cell `(cx, cy)`.
    fn blobs_in(&self, cx: i64, cy: i64, blobs: &mut Vec<Blob>) {
        let (kx, ky) = match self.period_cells {
            Some(period) => (cx.rem_euclid(period), cy.rem_euclid(period)),
            None => (cx, cy),
        };
        let cell = hash(&[self.config.seed, kx as u64, ky as u64]);
        let (low, high) = self.config.radius_um;
        for i in 0..poisson(self.per_cell, unit(mix(cell))) {
            let blob = hash(&[cell, i + 1]);
            let centre = Point::new(
                (cx as f64 + unit(mix(blob ^ 1))) * self.cell_um,
                (cy as f64 + unit(mix(blob ^ 2))) * self.cell_um,
            );
            if let Extent::Finite {
                width_um,
                height_um,
            } = self.config.extent
                && (centre.x.abs() > width_um / 2.0 || centre.y.abs() > height_um / 2.0)
            {
                continue;
            }
            let colours = &self.config.colours;
            let colour = colours[(mix(blob ^ 4) % colours.len().max(1) as u64) as usize];
            blobs.push(Blob {
                centre,
                radius: low + (high - low) * unit(mix(blob ^ 3)),
                transmits: colour.map(|c| f32::from(c) / 255.0),
            });
        }
    }
}

impl Specimen for BlobSpecimen {
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage {
        let (w, h) = (width as usize, height as usize);
        // The share of the light that reaches each pixel, per channel.
        let mut light = vec![1.0f32; w * h * 3];

        // The cells whose blobs can reach the region, kept within a finite sample.
        let reach = self.config.radius_um.1;
        let mut x_range = (
            origin.x - reach,
            origin.x + width as f64 * um_per_px + reach,
        );
        let mut y_range = (
            origin.y - reach,
            origin.y + height as f64 * um_per_px + reach,
        );
        if let Extent::Finite {
            width_um,
            height_um,
        } = self.config.extent
        {
            x_range = (
                x_range.0.max(-width_um / 2.0),
                x_range.1.min(width_um / 2.0),
            );
            y_range = (
                y_range.0.max(-height_um / 2.0),
                y_range.1.min(height_um / 2.0),
            );
        }
        let cells = |(start, end): (f64, f64)| {
            (start / self.cell_um).floor() as i64..=(end / self.cell_um).floor() as i64
        };

        let mut blobs = Vec::new();
        if x_range.0 <= x_range.1 && y_range.0 <= y_range.1 {
            for cy in cells(y_range) {
                for cx in cells(x_range) {
                    self.blobs_in(cx, cy, &mut blobs);
                }
            }
        }
        for blob in &blobs {
            let centre = (
                (blob.centre.x - origin.x) / um_per_px,
                (blob.centre.y - origin.y) / um_per_px,
            );
            draw(
                &mut light,
                w,
                h,
                centre,
                blob.radius / um_per_px,
                blob.transmits,
            );
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

/// Darkens the pixels a blob covers. A blob smaller than half a pixel is the
/// level of detail for low magnification: it darkens the pixel its centre is
/// in by the share of the pixel it covers.
fn draw(light: &mut [f32], w: usize, h: usize, centre: (f64, f64), r: f64, transmits: [f32; 3]) {
    let mut absorb = |x: usize, y: usize, coverage: f32| {
        let p = (y * w + x) * 3;
        for c in 0..3 {
            light[p + c] *= 1.0 - coverage * (1.0 - transmits[c]);
        }
    };
    if r < 0.5 {
        let (x, y) = (centre.0.floor(), centre.1.floor());
        if x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h {
            absorb(
                x as usize,
                y as usize,
                (std::f64::consts::PI * r * r).min(1.0) as f32,
            );
        }
        return;
    }
    let clamp = |v: f64, limit: usize| v.max(0.0).min(limit as f64) as usize;
    let (x0, x1) = (clamp(centre.0 - r - 1.0, w), clamp(centre.0 + r + 2.0, w));
    let (y0, y1) = (clamp(centre.1 - r - 1.0, h), clamp(centre.1 + r + 2.0, h));
    for y in y0..y1 {
        let dy = y as f64 + 0.5 - centre.1;
        for x in x0..x1 {
            let dx = x as f64 + 0.5 - centre.0;
            // Soft edges: full inside, fading over a pixel at the rim.
            let coverage = (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if coverage > 0.0 {
                absorb(x, y, coverage as f32);
            }
        }
    }
}

/// A Poisson-distributed count with mean `mean`, from a uniform number `u`.
fn poisson(mean: f64, u: f64) -> u64 {
    let (mut k, mut p) = (0, (-mean).exp());
    let mut cumulative = p;
    while u > cumulative && k < 1000 {
        k += 1;
        p *= mean / k as f64;
        cumulative += p;
    }
    k
}
