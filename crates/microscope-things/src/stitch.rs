//! Stitching scans (P39, ADR-0030): a scan's images, one a site, placed by
//! the stage positions and camera–stage mapping their sidecars record
//! (ADR-0025). While a scan runs, a small preview is blended again after
//! each site; once it's stitched, its `stitched/` folder holds a deep-zoom
//! image, `mosaic.dzi`, with `overview.jpeg` and `thumbnail.jpeg`. Both
//! record where each image went (P53, ADR-0036): the mosaic's in
//! `stitched/tiles.json`.
//!
//! A final stitch can refine the stage's placement by correlation (P13,
//! ADR-0046): each image goes where its overlaps with its neighbours put it,
//! and `tiles.json` records where the stage put it too. The live preview
//! keeps the stage's positions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image::RgbImage;
use image::imageops::{self, FilterType};
use microscope_core::dzi::{self, DziFile};
use microscope_core::mosaic::Layout;
use microscope_core::placement::{self, PlacementOptions};
use microscope_core::registration::{Gray, Options};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;

use crate::capture::{MappingRecord, Sidecar};

/// The folder, in a scan's, of its stitched mosaic.
pub const STITCHED: &str = "stitched";
/// The mosaic's name: `mosaic.dzi`, with its tiles in `mosaic_files/`.
pub const MOSAIC: &str = "mosaic";
/// The file, in `stitched/`, of where each image went.
pub const TILES: &str = "tiles.json";
/// The longer side of the live preview, at most.
const PREVIEW_SIDE: u32 = 1024;
/// How many times smaller the live preview keeps each image.
const PREVIEW_SHRINK: u32 = 8;
/// About how wide, in pixels, images are shrunk to for correlation: to a
/// preview's detail, where a JPEG's 8-pixel blocks hardly show. Halved, a
/// full-resolution capture's blocks are a stronger match than a sparse
/// sample (P13).
const REGISTERED_WIDTH: u32 = 400;

/// A stitched mosaic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stitched {
    /// Its size, in pixels: width and height.
    pub size: [u32; 2],
    /// Its deep-zoom levels.
    pub levels: u32,
    /// How many images it's made of: one a site.
    pub images: u32,
    /// How far correlation moved its images, if it refined them.
    pub correlation: Option<Corrections>,
}

/// How far correlation moved a mosaic's images from where the stage put
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Corrections {
    /// How many it moved by a pixel or more.
    pub moved: u32,
    /// The furthest it moved one, in pixels.
    pub largest: f64,
}

impl Stitched {
    /// What a scan's log says of it.
    pub fn summary(&self) -> String {
        let ([width, height], images) = (self.size, self.images);
        match self.correlation {
            Some(Corrections { moved, largest }) => format!(
                "Stitched: {width} × {height} pixels; correlation moved {moved} of the \
                 {images} images, the furthest by {largest:.1} pixels"
            ),
            None => format!("Stitched: {width} × {height} pixels, where the stage put the images"),
        }
    }
}

/// Where each image of a mosaic went: the mosaic's size, and each image's
/// rectangle in it, in its pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TileLayout {
    /// The mosaic's size, in pixels: width and height.
    pub size: [u32; 2],
    /// Each image, in the order they were taken.
    pub tiles: Vec<PlacedImage>,
}

/// An image's place in a mosaic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlacedImage {
    /// Its file name, in the scan's `images/`.
    pub image: String,
    /// Its number in the scan: 1 for the first taken.
    pub number: u32,
    /// Its left edge, in the mosaic's pixels.
    pub x: u32,
    /// Its top edge, in the mosaic's pixels.
    pub y: u32,
    /// Its width, in the mosaic's pixels.
    pub width: u32,
    /// Its height, in the mosaic's pixels.
    pub height: u32,
    /// Where the stage put it, in the mosaic's pixels: its left edge. Where
    /// correlation moved it, this isn't `x`.
    pub stage_x: i64,
    /// Where the stage put its top edge.
    pub stage_y: i64,
}

/// An image to stitch, and where and when it was taken.
struct Site {
    path: PathBuf,
    at: [i64; 2],
    /// When: RFC 3339, in UTC, so in order as text.
    acquired: String,
    size: [u32; 2],
    mapping: Option<MappingRecord>,
}

/// `scan`'s images to stitch, one a site: the middle of each site's stack,
/// by z.
fn sites(scan: &Path) -> Result<Vec<Site>, ActionError> {
    let images = scan.join("images");
    let mut stacks: BTreeMap<[i64; 2], Vec<(i64, Site)>> = BTreeMap::new();
    for file in std::fs::read_dir(&images)
        .map_err(ActionError::handled)?
        .flatten()
    {
        let path = file.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_default();
        let Ok(sidecar) = serde_json::from_slice::<Sidecar>(&bytes) else {
            continue;
        };
        let Some(steps) = sidecar.stage.as_ref().map(|stage| stage.steps) else {
            continue;
        };
        let at = [steps.x, steps.y];
        stacks.entry(at).or_default().push((
            steps.z,
            Site {
                path: images.join(&sidecar.image),
                at,
                acquired: sidecar.acquired,
                size: [sidecar.width, sidecar.height],
                mapping: sidecar.camera_stage_mapping,
            },
        ));
    }
    let sites: Vec<Site> = (stacks.into_values())
        .map(|mut stack| {
            stack.sort_by_key(|(z, _)| *z);
            stack.swap_remove(stack.len() / 2).1
        })
        .collect();
    if sites.is_empty() {
        return Err(ActionError::handled("the scan has no images to stitch"));
    }
    Ok(sites)
}

/// Where `sites` go, as images `size` pixels in size, through the first
/// camera–stage mapping they record.
fn layout(sites: &[Site], size: [u32; 2]) -> Result<Layout, ActionError> {
    let mapping = (sites.iter())
        .find_map(|site| site.mapping.as_ref())
        .ok_or_else(|| ActionError::handled("the images don't record the camera–stage mapping"))?;
    let at: Vec<[i64; 2]> = sites.iter().map(|site| site.at).collect();
    let matrix = &mapping.image_to_stage_displacement_matrix;
    Layout::new(&at, matrix, mapping.image_resolution, size).map_err(ActionError::handled)
}

/// Where `sites` go once correlation has refined `stage`, their placement
/// by the stage (ADR-0046); where the stage put them, in the same pixels;
/// and how far it moved them.
fn refined(
    sites: &[Site],
    stage: &Layout,
) -> Result<(Layout, Vec<[i64; 2]>, Corrections), ActionError> {
    let expected: Vec<[f64; 2]> = stage.origins.iter().map(|o| o.map(f64::from)).collect();
    let read = &mut |index: usize| gray(&sites[index].path, stage.tile);
    let size = stage.tile.map(|n| n as usize);
    let options = PlacementOptions {
        registration: Options {
            downsample: (stage.tile[0] / REGISTERED_WIDTH).max(1) as usize,
            ..Options::default()
        },
        ..PlacementOptions::default()
    };
    let placement =
        placement::place_with(&expected, size, read, &options).map_err(ActionError::handled)?;
    // The mosaic starts at the leftmost and topmost image, as placed.
    let low =
        |axis: usize| (placement.origins.iter()).fold(f64::INFINITY, |low, o| low.min(o[axis]));
    let corner = [low(0), low(1)];
    let from_corner = |o: &[f64; 2]| [0, 1].map(|axis| (o[axis] - corner[axis]).round() as i64);
    let origins: Vec<[u32; 2]> = (placement.origins.iter())
        .map(|o| from_corner(o).map(|n| n.max(0) as u32))
        .collect();
    let high = |axis: usize| origins.iter().map(|o| o[axis]).max().unwrap_or(0) + stage.tile[axis];
    let moves = placement.moves(&expected);
    let corrections = Corrections {
        moved: moves.iter().filter(|&&moved| moved >= 1.0).count() as u32,
        largest: moves.iter().copied().fold(0.0, f64::max),
    };
    let layout = Layout {
        size: [high(0), high(1)],
        origins,
        tile: stage.tile,
    };
    Ok((
        layout,
        expected.iter().map(from_corner).collect(),
        corrections,
    ))
}

/// The image at `path` in grey, for registering, if it's `size` pixels.
fn gray(path: &Path, size: [u32; 2]) -> Result<Gray, String> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let image = image::open(path).map_err(|error| format!("{name}: {error}"))?;
    let image = image.to_rgb8();
    if image.dimensions() != (size[0], size[1]) {
        return Err(format!("{name} isn't the size of the others"));
    }
    let (width, height) = (image.width() as usize, image.height() as usize);
    Ok(Gray::from_rgb8(width, height, image.as_raw()))
}

/// Where the stage put each of `layout`'s images, where it's their only
/// placement.
fn unrefined(layout: &Layout) -> Vec<[i64; 2]> {
    (layout.origins.iter()).map(|o| o.map(i64::from)).collect()
}

/// Where `sites` went in `layout`, numbered in the order they were taken,
/// and where the stage put them.
fn placed(sites: &[Site], layout: &Layout, stage: &[[i64; 2]]) -> TileLayout {
    let mut order: Vec<usize> = (0..sites.len()).collect();
    order.sort_by_key(|&i| (&sites[i].acquired, &sites[i].path));
    let tiles = (order.iter().zip(1..))
        .map(|(&i, number)| PlacedImage {
            image: (sites[i].path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            number,
            x: layout.origins[i][0],
            y: layout.origins[i][1],
            width: layout.tile[0],
            height: layout.tile[1],
            stage_x: stage[i][0],
            stage_y: stage[i][1],
        })
        .collect();
    TileLayout {
        size: layout.size,
        tiles,
    }
}

/// Stitches `scan` into its `stitched/` folder, replacing any it had, with
/// its images where correlation puts them if `refine`, or else where the
/// stage put them. It's written beside, then moved into place, so it's
/// never seen half done.
pub fn stitch(scan: &Path, refine: bool) -> Result<Stitched, ActionError> {
    let sites = sites(scan)?;
    let stage = layout(&sites, sites[0].size)?;
    let (layout, at, correlation) = if refine {
        let (layout, at, corrections) = refined(&sites, &stage)?;
        (layout, at, Some(corrections))
    } else {
        let at = unrefined(&stage);
        (stage, at, None)
    };
    let partial = scan.join(format!("{STITCHED}.partial"));
    let _ = std::fs::remove_dir_all(&partial);
    let pyramid = dzi::write(
        &layout,
        MOSAIC,
        &mut |index| match image::open(&sites[index].path) {
            Ok(image) => Ok(image.to_rgb8()),
            Err(error) => Err(error.to_string()),
        },
        &mut |file: DziFile| write(&partial.join(&file.path), &file.bytes),
    )
    .map_err(ActionError::handled)?;
    let tiles =
        serde_json::to_vec_pretty(&placed(&sites, &layout, &at)).map_err(ActionError::handled)?;
    write(&partial.join(TILES), &tiles).map_err(ActionError::handled)?;
    for (name, picture) in [
        ("overview.jpeg", &pyramid.overview),
        ("thumbnail.jpeg", &pyramid.thumbnail),
    ] {
        let jpeg = dzi::jpeg(picture).map_err(ActionError::handled)?;
        write(&partial.join(name), &jpeg).map_err(ActionError::handled)?;
    }
    let stitched = scan.join(STITCHED);
    if stitched.exists() {
        std::fs::remove_dir_all(&stitched).map_err(ActionError::handled)?;
    }
    std::fs::rename(&partial, &stitched).map_err(ActionError::handled)?;
    Ok(Stitched {
        size: pyramid.size,
        levels: pyramid.levels,
        images: sites.len() as u32,
        correlation,
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}

/// A scan's live preview: a small copy of each site's image, kept as the
/// scan goes, and blended again after each site.
#[derive(Default)]
pub struct Preview {
    /// Each site's image and its small copy, by where it was taken.
    small: BTreeMap<[i64; 2], (PathBuf, RgbImage)>,
}

impl Preview {
    /// Takes in `scan`'s new images, and draws the preview again: a JPEG
    /// whose longer side is at most 1024 pixels, and where each image went,
    /// in the pixels of the mosaic it was shrunk from.
    pub fn update(&mut self, scan: &Path) -> Result<(Vec<u8>, TileLayout), ActionError> {
        let sites = sites(scan)?;
        let small_size = sites[0].size.map(|n| (n / PREVIEW_SHRINK).max(1));
        for site in &sites {
            if (self.small.get(&site.at)).is_some_and(|(path, _)| *path == site.path) {
                continue;
            }
            let image = image::open(&site.path).map_err(ActionError::handled)?;
            let [width, height] = small_size;
            let small = imageops::resize(&image.to_rgb8(), width, height, FilterType::Triangle);
            self.small.insert(site.at, (site.path.clone(), small));
        }
        let layout = layout(&sites, small_size)?;
        let preview = layout.preview(PREVIEW_SIDE, &mut |index| {
            Ok(self.small[&sites[index].at].1.clone())
        });
        let jpeg = dzi::jpeg(&preview.map_err(ActionError::handled)?);
        let tiles = placed(&sites, &layout, &unrefined(&layout));
        Ok((jpeg.map_err(ActionError::handled)?, tiles))
    }
}
