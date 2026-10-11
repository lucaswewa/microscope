//! A specimen from a picture: a PNG, JPEG or TIFF image laid on the slide
//! at a given scale, centred on the origin, with empty slide round it.
//! Copies halved again and again (mip levels) stand in at low
//! magnification, so a pixel shows the average of what it covers.

use std::path::Path;

use image::{ImageResult, RgbImage};

use crate::specimen::Specimen;
use crate::units::Point;

/// A picture on the slide (see the module's description).
#[derive(Debug, Clone)]
pub struct ImageSpecimen {
    /// The picture, then copies of it halved again and again, to a pixel.
    levels: Vec<RgbImage>,
    /// The size of the picture's pixels on the slide, in µm.
    um_per_px: f64,
    /// The colour of the empty slide.
    background: [u8; 3],
}

impl ImageSpecimen {
    /// The picture in the file at `path` (PNG, JPEG or TIFF), its pixels
    /// `um_per_px` across on the slide.
    pub fn open(path: &Path, um_per_px: f64) -> ImageResult<Self> {
        Ok(Self::new(image::open(path)?.to_rgb8(), um_per_px))
    }

    /// `picture`, its pixels `um_per_px` across on the slide.
    pub fn new(picture: RgbImage, um_per_px: f64) -> Self {
        let mut levels = vec![picture];
        loop {
            let last = levels.last().expect("the picture");
            if last.width() <= 1 && last.height() <= 1 {
                break;
            }
            let halved = halve(last);
            levels.push(halved);
        }
        Self {
            levels,
            um_per_px,
            background: [246, 244, 240],
        }
    }
}

impl Specimen for ImageSpecimen {
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage {
        // The level with the largest pixels that are no larger than the frame's.
        let coarsest = (self.levels.len() - 1) as f64;
        let level = (um_per_px / self.um_per_px)
            .log2()
            .floor()
            .clamp(0.0, coarsest) as usize;
        let picture = &self.levels[level];
        let scale = self.um_per_px * f64::from(1u32 << level);
        let full = &self.levels[0];
        let (left, top) = (
            -f64::from(full.width()) * self.um_per_px / 2.0,
            -f64::from(full.height()) * self.um_per_px / 2.0,
        );
        RgbImage::from_fn(width, height, |i, j| {
            let x = origin.x + (f64::from(i) + 0.5) * um_per_px;
            let y = origin.y + (f64::from(j) + 0.5) * um_per_px;
            if x < left || x >= -left || y < top || y >= -top {
                return image::Rgb(self.background);
            }
            // In the level's pixels, from its first pixel's centre.
            image::Rgb(bilinear(
                picture,
                (x - left) / scale - 0.5,
                (y - top) / scale - 0.5,
            ))
        })
    }
}

/// `picture` at `(u, v)`, between pixel centres, with its edges extended.
fn bilinear(picture: &RgbImage, u: f64, v: f64) -> [u8; 3] {
    let (w, h) = (
        f64::from(picture.width() - 1),
        f64::from(picture.height() - 1),
    );
    let (u, v) = (u.clamp(0.0, w), v.clamp(0.0, h));
    let (x0, y0) = (u.floor(), v.floor());
    let (fx, fy) = (u - x0, v - y0);
    let at = |x: f64, y: f64| picture.get_pixel(x.min(w) as u32, y.min(h) as u32).0;
    let (a, b, c, d) = (
        at(x0, y0),
        at(x0 + 1.0, y0),
        at(x0, y0 + 1.0),
        at(x0 + 1.0, y0 + 1.0),
    );
    std::array::from_fn(|k| {
        let top = f64::from(a[k]) + (f64::from(b[k]) - f64::from(a[k])) * fx;
        let bottom = f64::from(c[k]) + (f64::from(d[k]) - f64::from(c[k])) * fx;
        (top + (bottom - top) * fy).round() as u8
    })
}

/// `picture` at half its size, rounded up: each pixel the average of the
/// two by two it covers.
fn halve(picture: &RgbImage) -> RgbImage {
    let (w, h) = picture.dimensions();
    RgbImage::from_fn(w.div_ceil(2), h.div_ceil(2), |x, y| {
        let at = |dx: u32, dy: u32| {
            picture
                .get_pixel((2 * x + dx).min(w - 1), (2 * y + dy).min(h - 1))
                .0
        };
        let corners = [at(0, 0), at(1, 0), at(0, 1), at(1, 1)];
        image::Rgb(std::array::from_fn(|c| {
            ((corners.iter().map(|p| u32::from(p[c])).sum::<u32>() + 2) / 4) as u8
        }))
    })
}
