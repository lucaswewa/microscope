//! Telling sample from empty background (P27): colours in CIE L*u*v*, and
//! the two ways of comparing an image with a background, by each pixel's
//! colour and by each region's texture. The behaviour spec opens the P27
//! notes.

/// An image's colours in CIE 1976 L*u*v*: three values per pixel, by rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Luv {
    /// The width, in pixels.
    pub width: usize,
    /// The height, in pixels.
    pub height: usize,
    /// L*, u* and v* for each pixel, by rows.
    pub values: Vec<[f64; 3]>,
}

/// The D65 white point's chromaticity, u′ and v′.
const WHITE: (f64, f64) = (0.197_839_8, 0.468_336_0);

impl Luv {
    /// An sRGB image's colours, from its interleaved 8-bit samples.
    pub fn from_rgb8(width: usize, height: usize, rgb: &[u8]) -> Self {
        // sRGB's decoding, once for each of the 256 levels.
        let linear: Vec<f64> = (0..=255)
            .map(|v| {
                let c = f64::from(v) / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            })
            .collect();
        let values = rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| {
                let [r, g, b] = [0, 1, 2].map(|c| linear[usize::from(p[c])]);
                let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
                let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
                let z = 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b;
                let l = if y > 0.008_856 {
                    116.0 * y.cbrt() - 16.0
                } else {
                    903.3 * y
                };
                let d = x + 15.0 * y + 3.0 * z;
                let (u, v) = if d > 0.0 {
                    (4.0 * x / d, 9.0 * y / d)
                } else {
                    WHITE
                };
                [l, 13.0 * l * (u - WHITE.0), 13.0 * l * (v - WHITE.1)]
            })
            .collect();
        Luv {
            width,
            height,
            values,
        }
    }
}

/// The smallest deviation a background's channel is given, so that a very
/// clean background doesn't make faint noise look like sample.
pub const DEVIATION_FLOOR: f64 = 0.5;

/// The side of the grid that texture is compared over.
pub const GRID: usize = 8;

/// Less than this, a deviation is none: the channel doesn't vary.
const BLANK: f64 = 1e-9;

/// What went wrong setting a background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankImage;

/// A background's colour: each channel's mean and standard deviation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourStatistics {
    /// The mean of L*, u* and v*.
    pub means: [f64; 3],
    /// Their standard deviations, at least [`DEVIATION_FLOOR`].
    pub deviations: [f64; 3],
}

impl ColourStatistics {
    /// The statistics of `background`, unless a channel doesn't vary.
    pub fn of(background: &Luv) -> Result<Self, BlankImage> {
        let (means, deviations) = statistics(&background.values);
        let deviations = floor(deviations)?;
        Ok(ColourStatistics { means, deviations })
    }

    /// The percentage of `image`'s pixels that aren't background: whose u*
    /// or v* is `tolerance` deviations or more from the background's.
    pub fn sample_coverage(&self, image: &Luv, tolerance: f64) -> f64 {
        let background = |p: &&[f64; 3]| {
            (1..3).all(|c| (p[c] - self.means[c]).abs() < tolerance * self.deviations[c])
        };
        let n = image.values.len().max(1) as f64;
        100.0 * (1.0 - image.values.iter().filter(background).count() as f64 / n)
    }
}

/// A background's texture: the median, over a grid, of each channel's
/// standard deviation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureStatistics {
    /// The median deviations of L*, u* and v*, at least [`DEVIATION_FLOOR`].
    pub deviations: [f64; 3],
}

impl TextureStatistics {
    /// The statistics of `background`, unless a channel doesn't vary
    /// anywhere.
    pub fn of(background: &Luv) -> Result<Self, BlankImage> {
        let cells = cell_deviations(background);
        let medians = std::array::from_fn(|c| {
            let mut channel: Vec<f64> = cells.iter().map(|cell| cell[c]).collect();
            channel.sort_by(f64::total_cmp);
            // Blank only if it varies nowhere; else at least the floor.
            match channel.last() {
                Some(most) if *most >= BLANK => channel[channel.len() / 2].max(DEVIATION_FLOOR),
                _ => 0.0,
            }
        });
        Ok(TextureStatistics {
            deviations: floor(medians)?,
        })
    }

    /// The percentage of the grid's cells that hold sample: where any
    /// channel's deviation is more than `tolerance` times the background's.
    pub fn sample_coverage(&self, image: &Luv, tolerance: f64) -> f64 {
        let cells = cell_deviations(image);
        let sample = cells
            .iter()
            .filter(|cell| (0..3).any(|c| cell[c] > tolerance * self.deviations[c]))
            .count();
        100.0 * sample as f64 / cells.len() as f64
    }
}

/// Each channel's mean and standard deviation over `values`.
fn statistics(values: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let n = values.len().max(1) as f64;
    let means: [f64; 3] = std::array::from_fn(|c| values.iter().map(|p| p[c]).sum::<f64>() / n);
    let deviations = std::array::from_fn(|c| {
        let squares: f64 = values.iter().map(|p| (p[c] - means[c]).powi(2)).sum();
        (squares / n).sqrt()
    });
    (means, deviations)
}

/// `deviations` raised to [`DEVIATION_FLOOR`], unless a channel is zero:
/// a blank image.
fn floor(deviations: [f64; 3]) -> Result<[f64; 3], BlankImage> {
    if deviations.iter().any(|d| *d < BLANK) {
        return Err(BlankImage);
    }
    Ok(deviations.map(|d| d.max(DEVIATION_FLOOR)))
}

/// The standard deviation of each channel in each cell of an 8 × 8 grid
/// over `image`, by rows. Cells are whole pixels; any left over at the right
/// and bottom are left out.
fn cell_deviations(image: &Luv) -> Vec<[f64; 3]> {
    let (h, w) = (image.height / GRID, image.width / GRID);
    let mut cells = Vec::with_capacity(GRID * GRID);
    let mut cell = Vec::with_capacity(w * h);
    for row in 0..GRID {
        for col in 0..GRID {
            cell.clear();
            for y in row * h..(row + 1) * h {
                let start = y * image.width + col * w;
                cell.extend_from_slice(&image.values[start..start + w]);
            }
            cells.push(statistics(&cell).1);
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luv(rgb: [u8; 3]) -> [f64; 3] {
        Luv::from_rgb8(1, 1, &rgb).values[0]
    }

    #[test]
    fn converts_srgb_to_cie_luv() {
        let close = |a: [f64; 3], b: [f64; 3]| (0..3).all(|c| (a[c] - b[c]).abs() < 0.05);
        // White and black, and the primaries' published values.
        assert!(
            close(luv([255, 255, 255]), [100.0, 0.0, 0.0]),
            "{:?}",
            luv([255; 3])
        );
        assert!(close(luv([0, 0, 0]), [0.0, 0.0, 0.0]));
        assert!(
            close(luv([255, 0, 0]), [53.24, 175.01, 37.76]),
            "{:?}",
            luv([255, 0, 0])
        );
        assert!(
            close(luv([0, 255, 0]), [87.73, -83.07, 107.40]),
            "{:?}",
            luv([0, 255, 0])
        );
        assert!(
            close(luv([0, 0, 255]), [32.30, -9.40, -130.34]),
            "{:?}",
            luv([0, 0, 255])
        );
        // Greys are colourless; sRGB's mid-grey is half as light as white.
        let grey = luv([119, 119, 119]);
        assert!(grey[1].abs() < 1e-3 && grey[2].abs() < 1e-3, "{grey:?}");
        assert!((grey[0] - 50.05).abs() < 0.05, "{grey:?}");
    }

    /// A 64 × 64 image: `pixel(x, y)` for each pixel.
    fn image(pixel: impl Fn(usize, usize) -> [u8; 3]) -> Luv {
        let rgb: Vec<u8> = (0..64 * 64).flat_map(|i| pixel(i % 64, i / 64)).collect();
        Luv::from_rgb8(64, 64, &rgb)
    }

    /// The empty slide: near-white, with a little noise.
    fn slide(x: usize, y: usize) -> [u8; 3] {
        let noise = ((x * 7 + y * 13) % 5) as u8;
        [244 + noise, 242 + noise, 238 + noise]
    }

    #[test]
    fn colour_coverage_is_the_share_of_pixels_off_the_backgrounds_colour() {
        let background = ColourStatistics::of(&image(slide)).expect("varies");
        assert!(background.deviations.iter().all(|d| *d >= DEVIATION_FLOOR));
        // The left quarter stained pink; the rest the slide, but darker,
        // which only changes L*.
        let stained = image(|x, y| match x < 16 {
            true => [220, 120, 170],
            false => slide(x, y).map(|c| c - 40),
        });
        assert_eq!(background.sample_coverage(&stained, 7.0), 25.0);
        assert_eq!(background.sample_coverage(&image(slide), 7.0), 0.0);
    }

    #[test]
    fn texture_coverage_is_the_share_of_cells_more_varied_than_the_background() {
        let background = TextureStatistics::of(&image(slide)).expect("varies");
        // Coarse dark stripes in the top half's cells: 32 of the 64.
        let textured = image(|x, y| match y < 32 && x % 4 < 2 {
            true => [60, 40, 90],
            false => slide(x, y),
        });
        assert_eq!(background.sample_coverage(&textured, 7.0), 50.0);
        // A smooth change in brightness isn't texture.
        let shaded = image(|x, y| slide(x, y).map(|c| c - (x / 8) as u8));
        assert_eq!(background.sample_coverage(&shaded, 7.0), 0.0);
        // Texture in any one channel counts: here v* alone, in the left half.
        let mut tinted = image(slide);
        for (i, value) in tinted.values.iter_mut().enumerate() {
            if i % 64 < 32 {
                value[2] += if i % 2 == 0 { 20.0 } else { -20.0 };
            }
        }
        assert_eq!(background.sample_coverage(&tinted, 7.0), 50.0);
    }

    #[test]
    fn a_blank_image_cant_be_a_background() {
        let blank = Luv::from_rgb8(16, 16, &[200; 16 * 16 * 3]);
        assert_eq!(ColourStatistics::of(&blank), Err(BlankImage));
        assert_eq!(TextureStatistics::of(&blank), Err(BlankImage));
    }
}
