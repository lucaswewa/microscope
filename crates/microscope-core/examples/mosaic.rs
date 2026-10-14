//! Writes a deep-zoom mosaic of a made-up scan to a folder, to look at and
//! to time (P38):
//!
//! ```text
//! cargo run -p microscope-core --release --example mosaic -- <folder> [columns] [rows]
//! ```
//!
//! The scan is `columns` × `rows` tiles (10 × 8 unless given) of 1640 ×
//! 1232 pixels, overlapping by a third, each drawn when it's asked for from
//! a pattern of rings. The folder gets `mosaic.dzi` and `mosaic_files/`,
//! which the gallery's viewer can show, and `overview.jpeg` and
//! `thumbnail.jpeg`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{Rgb, RgbImage};
use microscope_core::dzi::{self, DziFile};
use microscope_core::mosaic::Layout;

const TILE: [u32; 2] = [1640, 1232];
const STEP: [u32; 2] = [1100, 820];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().ok_or("give a folder to write to")?);
    let columns: u32 = args.next().map_or(Ok(10), |n| n.parse())?;
    let rows: u32 = args.next().map_or(Ok(8), |n| n.parse())?;
    // Stage steps are pixels here, through the identity mapping.
    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let positions: Vec<[i64; 2]> = (0..rows)
        .flat_map(|j| (0..columns).map(move |i| [i64::from(i * STEP[0]), i64::from(j * STEP[1])]))
        .collect();
    let layout = Layout::new(&positions, &identity, TILE, TILE)?;
    let started = Instant::now();
    let pyramid = dzi::write(
        &layout,
        "mosaic",
        &mut |index| Ok(tile(layout.origins[index])),
        &mut |file| save(&folder, file),
    )?;
    let [width, height] = pyramid.size;
    println!(
        "{width} × {height} pixels from {} tiles, {} levels, in {:.1} s, holding at most {} tiles",
        layout.origins.len(),
        pyramid.levels,
        started.elapsed().as_secs_f64(),
        pyramid.most_tiles_held
    );
    std::fs::write(folder.join("overview.jpeg"), dzi::jpeg(&pyramid.overview)?)?;
    std::fs::write(
        folder.join("thumbnail.jpeg"),
        dzi::jpeg(&pyramid.thumbnail)?,
    )?;
    Ok(())
}

/// The tile whose top-left corner is at `origin`: rings about the middle,
/// on a gradient, as if they were the sample.
fn tile(origin: [u32; 2]) -> RgbImage {
    RgbImage::from_fn(TILE[0], TILE[1], |x, y| {
        let (fx, fy) = (f64::from(origin[0] + x), f64::from(origin[1] + y));
        let ring = ((fx - 5000.0).hypot(fy - 3500.0) / 40.0).sin();
        let shade = (fx / 80.0).sin() * (fy / 60.0).cos();
        Rgb([
            (200.0 + 50.0 * ring) as u8,
            (150.0 + 60.0 * shade) as u8,
            (180.0 - 40.0 * ring * shade) as u8,
        ])
    })
}

fn save(folder: &Path, file: DziFile) -> Result<(), String> {
    let path = folder.join(&file.path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, &file.bytes).map_err(|error| error.to_string())
}
