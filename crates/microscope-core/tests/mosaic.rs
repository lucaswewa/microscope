//! Mosaics (P38): tiles cut from a known image, at stage positions through a
//! turned and scaled mapping, reassemble into it, at full size, in a
//! reduced preview, and through a deep-zoom pyramid of the right shape,
//! made within bounded memory.

use std::collections::HashMap;

use image::{GenericImageView, Rgb, RgbImage};
use microscope_core::dzi::{self, DziFile, TILE_SIZE, level_size, top_level};
use microscope_core::mapping::Matrix2;
use microscope_core::mosaic::{Layout, MosaicError};

/// The tiles' size, and the frames' the mapping was calibrated on: half.
const TILE: [u32; 2] = [200, 150];
const CALIBRATED: [u32; 2] = [100, 75];
/// The tiles are cut this far apart, so neighbours overlap.
const STEP: [u32; 2] = [160, 120];
/// The grid of tiles: columns and rows.
const GRID: [u32; 2] = [5, 4];
/// A quarter turn, two stage steps to a calibrated pixel.
const MATRIX: Matrix2 = [[0.0, 2.0], [-2.0, 0.0]];

/// A smooth picture with some detail, the mosaic's size.
fn known() -> RgbImage {
    let size = [0, 1].map(|i| TILE[i] + (GRID[i] - 1) * STEP[i]);
    RgbImage::from_fn(size[0], size[1], |x, y| {
        let (fx, fy) = (f64::from(x), f64::from(y));
        let wave = 128.0 + 100.0 * (fx / 17.0).sin() * (fy / 23.0).cos();
        Rgb([
            (x * 255 / size[0]) as u8,
            (y * 255 / size[1]) as u8,
            wave as u8,
        ])
    })
}

/// The tiles cut from `image`, row by row, and the stage's position for
/// each, from an arbitrary start.
fn cut(image: &RgbImage) -> (Vec<RgbImage>, Vec<[i64; 2]>) {
    let mut tiles = Vec::new();
    let mut positions = Vec::new();
    for j in 0..GRID[1] {
        for i in 0..GRID[0] {
            let (x, y) = (i * STEP[0], j * STEP[1]);
            tiles.push(image.view(x, y, TILE[0], TILE[1]).to_image());
            // A tile's pixels are half a calibrated frame's.
            let (px, py) = (f64::from(x) / 2.0, f64::from(y) / 2.0);
            let stage = [0, 1].map(|r| MATRIX[r][0] * px + MATRIX[r][1] * py);
            positions.push([1000 + stage[0] as i64, -500 + stage[1] as i64]);
        }
    }
    (tiles, positions)
}

fn layout(positions: &[[i64; 2]]) -> Layout {
    Layout::new(positions, &MATRIX, CALIBRATED, TILE).expect("a layout")
}

/// The largest difference of any channel, and the mean.
fn difference(a: &RgbImage, b: &RgbImage) -> (u8, f64) {
    assert_eq!(a.dimensions(), b.dimensions());
    let pairs = a.as_raw().iter().zip(b.as_raw());
    let diffs: Vec<u8> = pairs.map(|(p, q)| p.abs_diff(*q)).collect();
    let mean = diffs.iter().map(|&d| f64::from(d)).sum::<f64>() / diffs.len() as f64;
    (diffs.iter().copied().max().unwrap_or(0), mean)
}

#[test]
fn tiles_are_placed_by_the_stage_through_the_mapping() {
    let (_, positions) = cut(&known());
    let layout = layout(&positions);
    let expected: Vec<[u32; 2]> = (0..GRID[1])
        .flat_map(|j| (0..GRID[0]).map(move |i| [i * STEP[0], j * STEP[1]]))
        .collect();
    assert_eq!(layout.origins, expected);
    assert_eq!(layout.size, [known().width(), known().height()]);
    // From the last tile first, the others are left of it and above.
    let backwards: Vec<[i64; 2]> = positions.iter().rev().copied().collect();
    let reversed = Layout::new(&backwards, &MATRIX, CALIBRATED, TILE).expect("a layout");
    assert!(reversed.origins.iter().eq(expected.iter().rev()));
    assert_eq!(reversed.size, layout.size);
    let singular = [[1.0, 2.0], [2.0, 4.0]];
    assert_eq!(
        Layout::new(&positions, &singular, CALIBRATED, TILE),
        Err(MosaicError::Singular)
    );
    assert_eq!(
        Layout::new(&[], &MATRIX, CALIBRATED, TILE),
        Err(MosaicError::NoTiles)
    );
}

#[test]
fn a_preview_reassembles_the_image_and_shrinks_to_fit() {
    let image = known();
    let (tiles, positions) = cut(&image);
    let layout = layout(&positions);
    let mut get = |index: usize| Ok(tiles[index].clone());
    let full = layout.preview(10_000, &mut get).expect("a preview");
    assert!(difference(&full, &image).0 <= 1);
    let small = layout.preview(210, &mut get).expect("a preview");
    assert_eq!(small.dimensions(), (210, 128));
    let shrunk = image::imageops::resize(&image, 210, 128, image::imageops::FilterType::Triangle);
    let (_, mean) = difference(&small, &shrunk);
    assert!(mean < 3.0, "{mean}");
}

#[test]
fn seams_fade_across_the_overlap() {
    // Two flat tiles, overlapping by 50 pixels, the second brighter.
    let tiles = [100, 200].map(|v| RgbImage::from_pixel(100, 10, Rgb([v; 3])));
    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let layout =
        Layout::new(&[[0, 0], [50, 0]], &identity, [100, 10], [100, 10]).expect("a layout");
    let mosaic = layout
        .preview(1000, &mut |index| Ok(tiles[index].clone()))
        .expect("a preview");
    let row: Vec<u8> = (0..150).map(|x| mosaic.get_pixel(x, 5).0[0]).collect();
    assert!(row[..50].iter().all(|&v| v == 100), "{row:?}");
    assert!(row[100..].iter().all(|&v| v == 200), "{row:?}");
    assert!(
        row[50..100].windows(2).all(|pair| pair[0] <= pair[1]),
        "{row:?}"
    );
    assert!(row[60] < 130 && row[89] > 170, "{row:?}");
    // Apart, with the empty slide's white between.
    let apart =
        Layout::new(&[[0, 0], [150, 0]], &identity, [100, 10], [100, 10]).expect("a layout");
    let mosaic = apart
        .preview(1000, &mut |index| Ok(tiles[index].clone()))
        .expect("a preview");
    let gap: Vec<u8> = (100..150).map(|x| mosaic.get_pixel(x, 5).0[1]).collect();
    assert!(gap.iter().all(|&v| v == 255), "{gap:?}");
}

#[test]
fn a_pyramid_has_every_level_and_reassembles_within_jpeg_error() {
    let image = known();
    let (tiles, positions) = cut(&image);
    let layout = layout(&positions);
    let mut asked = vec![0; tiles.len()];
    let mut files = HashMap::new();
    let pyramid = dzi::write(
        &layout,
        "scan",
        &mut |index| {
            asked[index] += 1;
            Ok(tiles[index].clone())
        },
        &mut |DziFile { path, bytes }| {
            files.insert(path, bytes);
            Ok(())
        },
    )
    .expect("written");
    assert!(asked.iter().all(|&n| n == 1), "each tile once: {asked:?}");
    let (width, height) = image.dimensions();
    assert_eq!(pyramid.levels, 11);
    let descriptor = String::from_utf8(files["scan.dzi"].clone()).expect("text");
    assert!(descriptor.contains(r#"Format="jpeg" Overlap="0" TileSize="256""#));
    assert!(descriptor.contains(&format!(r#"<Size Width="{width}" Height="{height}"/>"#)));

    // Each level's tiles, each the right size.
    let mut count = 0;
    for level in 0..pyramid.levels {
        let [w, h] = level_size([width, height], level);
        for column in 0..w.div_ceil(TILE_SIZE) {
            for row in 0..h.div_ceil(TILE_SIZE) {
                let tile = &files[&format!("scan_files/{level}/{column}_{row}.jpeg")];
                let tile = image::load_from_memory(tile).expect("a JPEG").to_rgb8();
                let expected =
                    [(w, column), (h, row)].map(|(n, i)| TILE_SIZE.min(n - i * TILE_SIZE));
                assert_eq!(
                    [tile.width(), tile.height()],
                    expected,
                    "{level}/{column}_{row}"
                );
                count += 1;
            }
        }
    }
    assert_eq!(files.len(), count + 1, "the tiles and the .dzi, no more");

    // The most detailed level, put back together.
    let top = top_level([width, height]);
    let mut whole = RgbImage::new(width, height);
    for (path, bytes) in &files {
        let Some(name) = path.strip_prefix(&format!("scan_files/{top}/")) else {
            continue;
        };
        let (column, row) = name.trim_end_matches(".jpeg").split_once('_').expect("c_r");
        let tile = image::load_from_memory(bytes).expect("a JPEG").to_rgb8();
        let (x, y) = [column, row]
            .map(|n| n.parse::<u32>().expect("a number") * TILE_SIZE)
            .into();
        image::imageops::replace(&mut whole, &tile, i64::from(x), i64::from(y));
    }
    let (most, mean) = difference(&whole, &image);
    assert!(most < 40 && mean < 2.0, "{most}, {mean}");
    // Small enough to be its own overview, before compression.
    assert!(difference(&pyramid.overview, &image).0 <= 1);
    assert_eq!(pyramid.thumbnail.dimensions(), (320, 194));
}

#[test]
fn a_tall_mosaic_holds_only_a_rows_worth_of_tiles() {
    // A column of 30 tiles, far taller than the overview may be.
    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let positions: Vec<[i64; 2]> = (0..30).map(|j| [0, j * 120]).collect();
    let layout = Layout::new(&positions, &identity, [200, 150], [200, 150]).expect("a layout");
    let tile = RgbImage::from_pixel(200, 150, Rgb([90, 120, 150]));
    let pyramid =
        dzi::write(&layout, "tall", &mut |_| Ok(tile.clone()), &mut |_| Ok(())).expect("written");
    assert_eq!(pyramid.size, [200, 29 * 120 + 150]);
    assert!(pyramid.most_tiles_held <= 4, "{}", pyramid.most_tiles_held);
    let [w, h] = [pyramid.overview.width(), pyramid.overview.height()];
    assert_eq!([w, h], [100, 1815], "the level below, as 3630 is too tall");
}

#[test]
fn levels_halve_down_to_a_pixel() {
    assert_eq!(top_level([600, 400]), 10);
    assert_eq!(top_level([1, 1]), 0);
    assert_eq!(top_level([512, 3]), 9);
    assert_eq!(top_level([3, 513]), 10);
    assert_eq!(level_size([600, 400], 9), [300, 200]);
    assert_eq!(level_size([600, 400], 1), [2, 1]);
    assert_eq!(level_size([600, 400], 0), [1, 1]);
}

#[test]
fn bad_tiles_and_failed_writes_are_errors() {
    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let layout = Layout::new(&[[0, 0], [10, 0]], &identity, [20, 20], [20, 20]).expect("a layout");
    let small = RgbImage::new(10, 20);
    let wrong = dzi::write(&layout, "x", &mut |_| Ok(small.clone()), &mut |_| Ok(()));
    assert_eq!(wrong.err(), Some(MosaicError::TileSize(0, [10, 20])));
    let missing = layout.preview(100, &mut |index| Err(format!("no file {index}")));
    assert_eq!(
        missing.err(),
        Some(MosaicError::Tile(0, "no file 0".to_owned()))
    );
    let tile = RgbImage::new(20, 20);
    let full = dzi::write(&layout, "x", &mut |_| Ok(tile.clone()), &mut |_| {
        Err("disk full".to_owned())
    });
    assert_eq!(full.err(), Some(MosaicError::Write("disk full".to_owned())));
}
