//! An SDK camera's pixels as 8-bit RGB, which the other Things use
//! (ADR-0043): mono as grey, Bayer mosaics demosaicked, and pixels deeper
//! than 8 bits scaled down.

use image::RgbImage;

/// How a frame stores its pixels. GenICam's names are in brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// A byte a pixel (`Mono8`).
    Mono8,
    /// Two bytes a pixel, little-endian, of which `bits` are used
    /// (`Mono10`, `Mono12`, `Mono16`).
    Mono16 {
        /// The bits used, from 9 to 16.
        bits: u8,
    },
    /// Three bytes a pixel, red first (`RGB8`).
    Rgb8,
    /// Three bytes a pixel, blue first (`BGR8`).
    Bgr8,
    /// A Bayer mosaic, a byte a pixel (`BayerRG8`, …).
    Bayer8(Mosaic),
    /// A Bayer mosaic, two bytes a pixel, little-endian, of which `bits` are
    /// used (`BayerRG12`, …).
    Bayer16(Mosaic, u8),
}

/// A Bayer mosaic, by the colours of its first row's first two pixels and
/// the second row's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mosaic {
    /// Red, green; green, blue.
    Rggb,
    /// Green, red; blue, green.
    Grbg,
    /// Green, blue; red, green.
    Gbrg,
    /// Blue, green; green, red.
    Bggr,
}

impl Mosaic {
    /// The channel (0 red, 1 green, 2 blue) at `(x, y)`.
    fn channel(self, x: u32, y: u32) -> usize {
        let first = match self {
            Mosaic::Rggb => [0, 1, 1, 2],
            Mosaic::Grbg => [1, 0, 2, 1],
            Mosaic::Gbrg => [1, 2, 0, 1],
            Mosaic::Bggr => [2, 1, 1, 0],
        };
        first[(2 * (y % 2) + x % 2) as usize]
    }
}

impl PixelFormat {
    /// The format GenICam calls `name`, if it's one of these. Packed
    /// formats, such as `Mono12p`, aren't.
    pub fn from_genicam(name: &str) -> Option<PixelFormat> {
        let mosaic = |rest: &str| match rest {
            "RG" => Some(Mosaic::Rggb),
            "GR" => Some(Mosaic::Grbg),
            "GB" => Some(Mosaic::Gbrg),
            "BG" => Some(Mosaic::Bggr),
            _ => None,
        };
        let deep = |bits: &str| bits.parse().ok().filter(|bits| (9..=16).contains(bits));
        Some(match name {
            "Mono8" => PixelFormat::Mono8,
            "RGB8" => PixelFormat::Rgb8,
            "BGR8" => PixelFormat::Bgr8,
            _ => match (name.strip_prefix("Mono"), name.strip_prefix("Bayer")) {
                (Some(bits), _) => PixelFormat::Mono16 { bits: deep(bits)? },
                (_, Some(rest)) if rest.len() > 2 => {
                    let (pattern, bits) = rest.split_at(2);
                    match bits {
                        "8" => PixelFormat::Bayer8(mosaic(pattern)?),
                        bits => PixelFormat::Bayer16(mosaic(pattern)?, deep(bits)?),
                    }
                }
                _ => return None,
            },
        })
    }

    /// The bytes a pixel takes.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Mono8 | PixelFormat::Bayer8(_) => 1,
            PixelFormat::Mono16 { .. } | PixelFormat::Bayer16(..) => 2,
            PixelFormat::Rgb8 | PixelFormat::Bgr8 => 3,
        }
    }
}

/// A frame as an SDK delivers it.
#[derive(Debug, Clone, Copy)]
pub struct RawFrame<'a> {
    /// Its width, in pixels.
    pub width: u32,
    /// Its height, in pixels.
    pub height: u32,
    /// How its pixels are stored.
    pub format: PixelFormat,
    /// Its pixels, row by row, with no padding.
    pub data: &'a [u8],
}

/// `frame` as 8-bit RGB, or why it can't be.
pub fn to_rgb8(frame: &RawFrame) -> Result<RgbImage, String> {
    let RawFrame {
        width,
        height,
        format,
        data,
    } = *frame;
    let needed = width as usize * height as usize * format.bytes_per_pixel();
    if data.len() < needed {
        return Err(format!(
            "a {width} × {height} {format:?} frame needs {needed} bytes, not {}",
            data.len()
        ));
    }
    // A pixel's value, scaled to 8 bits.
    let value = |i: usize| match format {
        PixelFormat::Mono16 { bits } | PixelFormat::Bayer16(_, bits) => {
            (u16::from_le_bytes([data[2 * i], data[2 * i + 1]]) >> (bits - 8)).min(255) as u8
        }
        _ => data[i],
    };
    let at = |x: u32, y: u32| value(y as usize * width as usize + x as usize);
    Ok(match format {
        PixelFormat::Mono8 | PixelFormat::Mono16 { .. } => {
            RgbImage::from_fn(width, height, |x, y| image::Rgb([at(x, y); 3]))
        }
        PixelFormat::Rgb8 | PixelFormat::Bgr8 => {
            let mut pixels = data[..needed].to_vec();
            if format == PixelFormat::Bgr8 {
                pixels
                    .as_chunks_mut::<3>()
                    .0
                    .iter_mut()
                    .for_each(|pixel| pixel.swap(0, 2));
            }
            RgbImage::from_raw(width, height, pixels).expect("the size was checked")
        }
        PixelFormat::Bayer8(mosaic) | PixelFormat::Bayer16(mosaic, _) => {
            RgbImage::from_fn(width, height, |x, y| {
                demosaic(mosaic, (width, height), x, y, at)
            })
        }
    })
}

/// The pixel at `(x, y)` of a mosaic: its own channel, and each other one
/// averaged over its neighbours that have it.
fn demosaic(
    mosaic: Mosaic,
    (width, height): (u32, u32),
    x: u32,
    y: u32,
    at: impl Fn(u32, u32) -> u8,
) -> image::Rgb<u8> {
    let (mut sums, mut counts) = ([0u32; 3], [0u32; 3]);
    for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let channel = mosaic.channel(nx, ny);
            sums[channel] += u32::from(at(nx, ny));
            counts[channel] += 1;
        }
    }
    let own = mosaic.channel(x, y);
    image::Rgb(std::array::from_fn(|channel| match channel {
        _ if channel == own => at(x, y),
        _ if counts[channel] == 0 => 0,
        _ => (sums[channel] / counts[channel]) as u8,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: u32, height: u32, format: PixelFormat, data: &[u8]) -> RgbImage {
        let raw = RawFrame {
            width,
            height,
            format,
            data,
        };
        to_rgb8(&raw).expect("converted")
    }

    #[test]
    fn genicam_names_give_their_formats() {
        let named = |name| PixelFormat::from_genicam(name);
        assert_eq!(named("Mono8"), Some(PixelFormat::Mono8));
        assert_eq!(named("Mono12"), Some(PixelFormat::Mono16 { bits: 12 }));
        assert_eq!(named("BGR8"), Some(PixelFormat::Bgr8));
        assert_eq!(named("BayerGB8"), Some(PixelFormat::Bayer8(Mosaic::Gbrg)));
        assert_eq!(
            named("BayerRG10"),
            Some(PixelFormat::Bayer16(Mosaic::Rggb, 10))
        );
        for unknown in [
            "Mono12p", "Mono8x", "BayerXY8", "YUV422_8", "Mono7", "Mono17",
        ] {
            assert_eq!(named(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn mono_is_grey_and_deep_pixels_are_scaled() {
        let image = frame(2, 1, PixelFormat::Mono8, &[10, 200]);
        assert_eq!(image.into_raw(), [10, 10, 10, 200, 200, 200]);
        // 12 bits: 4095 is white, 2048 mid-grey.
        let deep = [0xFF, 0x0F, 0x00, 0x08];
        let image = frame(2, 1, PixelFormat::Mono16 { bits: 12 }, &deep);
        assert_eq!(image.into_raw(), [255, 255, 255, 128, 128, 128]);
    }

    #[test]
    fn colour_is_reordered_where_it_comes_blue_first() {
        let pixel = [1, 2, 3];
        assert_eq!(frame(1, 1, PixelFormat::Rgb8, &pixel).into_raw(), [1, 2, 3]);
        assert_eq!(frame(1, 1, PixelFormat::Bgr8, &pixel).into_raw(), [3, 2, 1]);
    }

    #[test]
    fn a_bayer_mosaic_of_one_colour_comes_out_that_colour() {
        // An orange scene: red 240, green 120, blue 0, through each mosaic.
        for mosaic in [Mosaic::Rggb, Mosaic::Grbg, Mosaic::Gbrg, Mosaic::Bggr] {
            let data: Vec<u8> = (0..16)
                .map(|i| [240, 120, 0][mosaic.channel(i % 4, i / 4)])
                .collect();
            let image = frame(4, 4, PixelFormat::Bayer8(mosaic), &data);
            for pixel in image.pixels() {
                assert_eq!(pixel.0, [240, 120, 0], "{mosaic:?}");
            }
        }
        // And in 16 bits, of which 10 are used.
        let data: Vec<u8> = (0..4)
            .flat_map(|i| [1020_u16, 512, 512, 0][i].to_le_bytes())
            .collect();
        let image = frame(2, 2, PixelFormat::Bayer16(Mosaic::Rggb, 10), &data);
        assert_eq!(image.get_pixel(0, 0).0, [255, 128, 0]);
    }

    #[test]
    fn a_short_frame_is_refused() {
        let raw = RawFrame {
            width: 2,
            height: 2,
            format: PixelFormat::Mono16 { bits: 16 },
            data: &[0; 7],
        };
        let refused = to_rgb8(&raw).expect_err("too short");
        assert!(refused.contains("needs 8 bytes, not 7"), "{refused}");
    }
}
