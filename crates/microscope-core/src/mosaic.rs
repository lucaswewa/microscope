//! Mosaics (P38, ADR-0030): tiles placed by where the stage was when each
//! was taken, through the camera–stage mapping, and blended where they
//! overlap. A preview draws the whole mosaic at a reduced scale;
//! [`crate::dzi`] writes it as a deep-zoom pyramid.

use std::fmt;

use image::imageops::{self, FilterType};
use image::{Rgb, RgbImage};

use crate::mapping::{Matrix2, apply, invert};

/// The colour where no tile is: white, like the empty slide.
pub const BACKGROUND: [u8; 3] = [255, 255, 255];

/// Gets the tile with an index, or says why it can't.
pub type Tiles<'a> = dyn FnMut(usize) -> Result<RgbImage, String> + 'a;

/// Why a mosaic can't be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MosaicError {
    /// There are no tiles.
    NoTiles,
    /// The camera–stage mapping can't be inverted.
    Singular,
    /// A tile couldn't be had: its index, and why.
    Tile(usize, String),
    /// A tile isn't the size of the others: its index, and its size.
    TileSize(usize, [u32; 2]),
    /// A file couldn't be written: why.
    Write(String),
}

impl fmt::Display for MosaicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTiles => write!(f, "there are no tiles"),
            Self::Singular => write!(f, "the camera–stage mapping can't be inverted"),
            Self::Tile(index, why) => write!(f, "tile {index}: {why}"),
            Self::TileSize(index, [w, h]) => {
                write!(f, "tile {index} is {w} × {h}, not the size of the others")
            }
            Self::Write(why) => write!(f, "couldn't write the mosaic: {why}"),
        }
    }
}

impl std::error::Error for MosaicError {}

/// A rectangle of the mosaic, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Its left edge.
    pub x: u32,
    /// Its top edge.
    pub y: u32,
    /// Its width.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

/// Where a mosaic's tiles go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Each tile's top-left corner, in the mosaic's pixels.
    pub origins: Vec<[u32; 2]>,
    /// The tiles' size: width and height.
    pub tile: [u32; 2],
    /// The mosaic's size: width and height.
    pub size: [u32; 2],
}

impl Layout {
    /// Tiles `tile` pixels in size, taken with the stage at `positions` (x
    /// and y, in steps), placed through `matrix`, the camera–stage mapping's
    /// image-to-stage matrix for frames `calibrated` pixels wide. Tiles
    /// larger than those frames are scaled to match. Each lands on the
    /// nearest whole pixel.
    pub fn new(
        positions: &[[i64; 2]],
        matrix: &Matrix2,
        calibrated: [u32; 2],
        tile: [u32; 2],
    ) -> Result<Layout, MosaicError> {
        let first = positions.first().ok_or(MosaicError::NoTiles)?;
        let inverse = invert(matrix).ok_or(MosaicError::Singular)?;
        let scale = f64::from(tile[0]) / f64::from(calibrated[0]);
        let placed: Vec<[i64; 2]> = (positions.iter())
            .map(|p| {
                let moved = [(p[0] - first[0]) as f64, (p[1] - first[1]) as f64];
                apply(&inverse, moved).map(|v| (v * scale).round() as i64)
            })
            .collect();
        let low = |axis: usize| placed.iter().map(|p| p[axis]).min().unwrap_or(0);
        let high = |axis: usize| placed.iter().map(|p| p[axis]).max().unwrap_or(0);
        let (left, top) = (low(0), low(1));
        Ok(Layout {
            origins: (placed.iter())
                .map(|p| [(p[0] - left) as u32, (p[1] - top) as u32])
                .collect(),
            tile,
            size: [
                (high(0) - left) as u32 + tile[0],
                (high(1) - top) as u32 + tile[1],
            ],
        })
    }

    /// The tiles that overlap `rect`, by index.
    pub fn overlapping(&self, rect: Rect) -> impl Iterator<Item = usize> + '_ {
        let [w, h] = self.tile;
        (self.origins.iter().enumerate())
            .filter(move |(_, [x, y])| {
                *x < rect.x + rect.width
                    && rect.x < x + w
                    && *y < rect.y + rect.height
                    && rect.y < y + h
            })
            .map(|(index, _)| index)
    }

    /// The whole mosaic, shrunk to fit `max_side` pixels on its longer side
    /// if it's larger. Each tile is had once, shrunk, and blended in.
    pub fn preview(&self, max_side: u32, tiles: &mut Tiles) -> Result<RgbImage, MosaicError> {
        let longest = self.size[0].max(self.size[1]);
        let scale = (f64::from(max_side) / f64::from(longest)).min(1.0);
        let at = |n: u32| (f64::from(n) * scale).round() as u32;
        let scaled = |n: u32| at(n).max(1);
        let rect = Rect {
            x: 0,
            y: 0,
            width: scaled(self.size[0]),
            height: scaled(self.size[1]),
        };
        let mut canvas = Canvas::new(rect);
        for (index, origin) in self.origins.iter().enumerate() {
            let tile = self.get(index, tiles)?;
            let small = imageops::resize(
                &tile,
                scaled(self.tile[0]),
                scaled(self.tile[1]),
                FilterType::Triangle,
            );
            canvas.add(&small, origin.map(|v| i64::from(at(v))));
        }
        Ok(canvas.finish())
    }

    /// Tile `index`, checked to be the layout's size.
    pub(crate) fn get(&self, index: usize, tiles: &mut Tiles) -> Result<RgbImage, MosaicError> {
        let tile = tiles(index).map_err(|why| MosaicError::Tile(index, why))?;
        let size = [tile.width(), tile.height()];
        if size != self.tile {
            return Err(MosaicError::TileSize(index, size));
        }
        Ok(tile)
    }
}

/// Weighted sums of colour over a rectangle of the mosaic, for blending:
/// each tile weighs most at its middle and least at its edges, so seams
/// fade rather than show.
pub(crate) struct Canvas {
    rect: Rect,
    sums: Vec<[f32; 4]>,
}

impl Canvas {
    pub(crate) fn new(rect: Rect) -> Canvas {
        let pixels = rect.width as usize * rect.height as usize;
        Canvas {
            rect,
            sums: vec![[0.0; 4]; pixels],
        }
    }

    /// Adds `image`, its top-left corner at `origin` in the mosaic.
    pub(crate) fn add(&mut self, image: &RgbImage, origin: [i64; 2]) {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.rect;
        let [ox, oy] = origin;
        let (ramp_x, ramp_y) = (ramp(image.width()), ramp(image.height()));
        let left = ox.max(i64::from(x));
        let right = (ox + i64::from(image.width())).min(i64::from(x + width));
        let top = oy.max(i64::from(y));
        let bottom = (oy + i64::from(image.height())).min(i64::from(y + height));
        for my in top..bottom {
            let ty = (my - oy) as u32;
            let row = (my - i64::from(y)) as usize * width as usize;
            for mx in left..right {
                let tx = (mx - ox) as u32;
                let weight = ramp_x[tx as usize] * ramp_y[ty as usize];
                let Rgb(colour) = *image.get_pixel(tx, ty);
                let sum = &mut self.sums[row + (mx - i64::from(x)) as usize];
                for (total, channel) in sum.iter_mut().zip(colour) {
                    *total += weight * f32::from(channel);
                }
                sum[3] += weight;
            }
        }
    }

    /// The blended rectangle: the background where no tile is.
    pub(crate) fn finish(self) -> RgbImage {
        let width = self.rect.width as usize;
        RgbImage::from_fn(self.rect.width, self.rect.height, |x, y| {
            let [r, g, b, weight] = self.sums[y as usize * width + x as usize];
            if weight == 0.0 {
                return Rgb(BACKGROUND);
            }
            Rgb([r, g, b].map(|c| (c / weight).round() as u8))
        })
    }
}

/// A tile's weights along a side of `n` pixels: 1 at each end, rising by 1
/// a pixel to the middle.
fn ramp(n: u32) -> Vec<f32> {
    (0..n).map(|i| (i.min(n - 1 - i) + 1) as f32).collect()
}
