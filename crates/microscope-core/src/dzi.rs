//! Deep-zoom pyramids (P38, ADR-0030): a mosaic written as a DZI image, as
//! the gallery's viewer reads it (ADR-0027). It's `<name>.dzi`, with its
//! tiles at `<name>_files/<level>/<column>_<row>.jpeg`, 256 pixels square
//! without overlap. The files go to whatever the caller gives, such as a
//! folder.
//!
//! Memory is bounded by the mosaic's width, not its area. The most detailed
//! level is drawn a row of tiles at a time, from only the mosaic's tiles
//! that overlap the row, and each level's rows are halved into the next
//! level's as they come. The overview, the largest level that fits
//! [`OVERVIEW_SIDE`], is kept whole, and a thumbnail is made from it.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImage, GenericImageView, Rgb, RgbImage};

use crate::mosaic::{Canvas, Layout, MosaicError, Rect, Tiles};

/// The side of a pyramid's tiles, in pixels.
pub const TILE_SIZE: u32 = 256;
/// The JPEG quality tiles are saved at.
pub const QUALITY: u8 = 90;
/// The longer side of the overview, at most.
pub const OVERVIEW_SIDE: u32 = 2048;
/// The longer side of the thumbnail, at most.
pub const THUMBNAIL_SIDE: u32 = 320;

/// A file of a deep-zoom image: where it goes, and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DziFile {
    /// Its path among the others: `<name>.dzi`, or a tile's in
    /// `<name>_files`.
    pub path: String,
    /// What it holds.
    pub bytes: Vec<u8>,
}

/// A pyramid that was written.
#[derive(Debug, Clone)]
pub struct Pyramid {
    /// The image's size: width and height.
    pub size: [u32; 2],
    /// How many levels it has. The most detailed is the last, one less.
    pub levels: u32,
    /// The largest level whose longer side fits [`OVERVIEW_SIDE`].
    pub overview: RgbImage,
    /// The overview, shrunk to fit [`THUMBNAIL_SIDE`] if it's larger.
    pub thumbnail: RgbImage,
    /// The most of the mosaic's tiles held at once.
    pub most_tiles_held: usize,
}

/// The most detailed level's number, for an image of `size`. Each level
/// halves the one above it, rounding up, down to one pixel at level 0.
pub fn top_level(size: [u32; 2]) -> u32 {
    let longest = size[0].max(size[1]).max(1);
    u32::BITS - (longest - 1).leading_zeros()
}

/// `level`'s size, in an image of `size`.
pub fn level_size(size: [u32; 2], level: u32) -> [u32; 2] {
    let halvings = top_level(size) - level;
    size.map(|n| u64::from(n).div_ceil(1 << halvings) as u32)
}

/// `image` as a JPEG, at [`QUALITY`].
pub fn jpeg(image: &RgbImage) -> Result<Vec<u8>, MosaicError> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, QUALITY)
        .encode_image(image)
        .map_err(|error| MosaicError::Write(error.to_string()))?;
    Ok(bytes)
}

/// Writes `layout`'s mosaic as the deep-zoom image `name`: its tiles are
/// had from `tiles`, each once, and its files given to `out`.
pub fn write(
    layout: &Layout,
    name: &str,
    tiles: &mut Tiles,
    out: &mut dyn FnMut(DziFile) -> Result<(), String>,
) -> Result<Pyramid, MosaicError> {
    if layout.origins.is_empty() {
        return Err(MosaicError::NoTiles);
    }
    let [width, height] = layout.size;
    let top = top_level(layout.size);
    let overview_level = (0..=top)
        .rev()
        .find(|&level| {
            level_size(layout.size, level)
                .iter()
                .all(|&n| n <= OVERVIEW_SIDE)
        })
        .unwrap_or(0);
    let [ow, oh] = level_size(layout.size, overview_level);
    let mut writer = Writer {
        name,
        out,
        size: layout.size,
        pending: vec![None; top as usize + 1],
        rows: vec![0; top as usize + 1],
        overview_level,
        overview: RgbImage::new(ow, oh),
    };
    let mut held: HashMap<usize, RgbImage> = HashMap::new();
    let mut most_tiles_held = 0;
    for y in (0..height).step_by(TILE_SIZE as usize) {
        let row = Rect {
            x: 0,
            y,
            width,
            height: TILE_SIZE.min(height - y),
        };
        // The tiles that end above this row aren't needed again.
        held.retain(|&index, _| layout.origins[index][1] + layout.tile[1] > y);
        for index in layout.overlapping(row) {
            if let Entry::Vacant(slot) = held.entry(index) {
                slot.insert(layout.get(index, tiles)?);
            }
        }
        most_tiles_held = most_tiles_held.max(held.len());
        let mut band = RgbImage::new(width, row.height);
        for x in (0..width).step_by(TILE_SIZE as usize) {
            let cell = Rect {
                x,
                width: TILE_SIZE.min(width - x),
                ..row
            };
            let mut canvas = Canvas::new(cell);
            for index in layout.overlapping(cell) {
                let origin = layout.origins[index].map(i64::from);
                canvas.add(&held[&index], origin);
            }
            (band.copy_from(&canvas.finish(), x, 0)).expect("the cell is in the row");
        }
        writer.band(top, band)?;
    }
    writer.descriptor()?;
    let overview = writer.overview;
    let thumbnail = DynamicImage::ImageRgb8(overview.clone())
        .thumbnail(THUMBNAIL_SIDE.min(ow), THUMBNAIL_SIDE.min(oh))
        .into_rgb8();
    Ok(Pyramid {
        size: layout.size,
        levels: top + 1,
        overview,
        thumbnail,
        most_tiles_held,
    })
}

/// Writes a pyramid's files a row of tiles at a time (see the module's
/// description).
struct Writer<'a> {
    name: &'a str,
    out: &'a mut dyn FnMut(DziFile) -> Result<(), String>,
    size: [u32; 2],
    /// For each level, a row waiting for the one below it, to be halved
    /// together into the next level's.
    pending: Vec<Option<RgbImage>>,
    /// For each level, its next row's number.
    rows: Vec<u32>,
    overview_level: u32,
    overview: RgbImage,
}

impl Writer<'_> {
    /// Writes `band`, `level`'s next row, and passes it down the levels.
    fn band(&mut self, level: u32, band: RgbImage) -> Result<(), MosaicError> {
        let i = level as usize;
        let row = self.rows[i];
        self.rows[i] += 1;
        for x in (0..band.width()).step_by(TILE_SIZE as usize) {
            let width = TILE_SIZE.min(band.width() - x);
            let tile = band.view(x, 0, width, band.height()).to_image();
            let column = x / TILE_SIZE;
            let path = format!("{}_files/{level}/{column}_{row}.jpeg", self.name);
            self.put(path, jpeg(&tile)?)?;
        }
        if level == self.overview_level {
            (self.overview.copy_from(&band, 0, row * TILE_SIZE))
                .expect("the row is in the overview");
        }
        if level == 0 {
            return Ok(());
        }
        let last = row + 1 == level_size(self.size, level)[1].div_ceil(TILE_SIZE);
        let joined = match self.pending[i].take() {
            None if !last => {
                self.pending[i] = Some(band);
                return Ok(());
            }
            None => band,
            Some(upper) => stack(&upper, &band),
        };
        self.band(level - 1, halve(&joined))
    }

    /// Writes the `.dzi` file, which describes the image.
    fn descriptor(&mut self) -> Result<(), MosaicError> {
        let [width, height] = self.size;
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <Image xmlns=\"http://schemas.microsoft.com/deepzoom/2008\" Format=\"jpeg\" \
             Overlap=\"0\" TileSize=\"{TILE_SIZE}\">\n  \
             <Size Width=\"{width}\" Height=\"{height}\"/>\n\
             </Image>\n"
        );
        self.put(format!("{}.dzi", self.name), xml.into_bytes())
    }

    fn put(&mut self, path: String, bytes: Vec<u8>) -> Result<(), MosaicError> {
        (self.out)(DziFile { path, bytes }).map_err(MosaicError::Write)
    }
}

/// `upper` with `lower` below it.
fn stack(upper: &RgbImage, lower: &RgbImage) -> RgbImage {
    let mut both = RgbImage::new(upper.width(), upper.height() + lower.height());
    (both.copy_from(upper, 0, 0)).expect("it fits");
    (both.copy_from(lower, 0, upper.height())).expect("it fits");
    both
}

/// `image` at half its size, rounding up: each pixel the mean of the two by
/// two it covers, or of those there are at the edges.
fn halve(image: &RgbImage) -> RgbImage {
    let (width, height) = image.dimensions();
    RgbImage::from_fn(width.div_ceil(2), height.div_ceil(2), |x, y| {
        let (mut sum, mut n) = ([0u32; 3], 0);
        for (sx, sy) in [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| (2 * x + dx, 2 * y + dy)) {
            if sx < width && sy < height {
                let Rgb(colour) = *image.get_pixel(sx, sy);
                for (total, channel) in sum.iter_mut().zip(colour) {
                    *total += u32::from(channel);
                }
                n += 1;
            }
        }
        Rgb(sum.map(|total| ((total + n / 2) / n) as u8))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(width: u32, values: &[u8]) -> RgbImage {
        let height = values.len() as u32 / width;
        RgbImage::from_fn(width, height, |x, y| {
            Rgb([values[(y * width + x) as usize]; 3])
        })
    }

    #[test]
    fn halving_averages_two_by_two_and_the_edges_alone() {
        let image = grey(3, &[0, 4, 9, 8, 12, 20, 30, 50, 70]);
        let half = halve(&image);
        let values: Vec<u8> = half.pixels().map(|p| p.0[0]).collect();
        // (0 + 4 + 8 + 12) / 4, (9 + 20) / 2, (30 + 50) / 2, and 70.
        assert_eq!(values, [6, 15, 40, 70]);
    }

    #[test]
    fn stacking_puts_one_below_the_other() {
        let both = stack(&grey(2, &[1, 2]), &grey(2, &[3, 4, 5, 6]));
        let values: Vec<u8> = both.pixels().map(|p| p.0[0]).collect();
        assert_eq!(values, [1, 2, 3, 4, 5, 6]);
    }
}
