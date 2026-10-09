//! Specimens: what is on the slide.

use image::RgbImage;

use crate::units::Point;

/// A sample on the slide, which can be drawn at any place and scale.
pub trait Specimen: Send + Sync {
    /// Draws the region whose top-left corner is `origin` (µm), at
    /// `um_per_px`, into an image of `width` by `height` pixels: pixel
    /// `(i, j)` covers the square from `origin + (i, j) × um_per_px`.
    ///
    /// The same specimen draws the same pixels for the same arguments, and a
    /// region drawn in two parts matches the region drawn whole. At low
    /// magnification, details smaller than a pixel are drawn as their average.
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage;
}

/// How far a specimen extends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    /// A rectangle of the given size, in µm, centred on the origin, with
    /// empty slide all round it.
    Finite {
        /// Its width, in µm.
        width_um: f64,
        /// Its height, in µm.
        height_um: f64,
    },
    /// The same pattern repeated in x and y, about every `period_um`.
    Repeating {
        /// The pattern's period, in µm, before it is rounded to a whole
        /// number of the specimen's cells.
        period_um: f64,
    },
}
