//! Stitching scans (P39, ADR-0030): a scan's images, one a site, placed by
//! the stage positions and camera–stage mapping their sidecars record
//! (ADR-0025). While a scan runs, a small preview is blended again after
//! each site; once it's stitched, its `stitched/` folder holds a deep-zoom
//! image, `mosaic.dzi`, with `overview.jpeg` and `thumbnail.jpeg`. Both
//! record where each image went (P53, ADR-0036): the mosaic's in
//! `stitched/tiles.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image::RgbImage;
use image::imageops::{self, FilterType};
use microscope_core::dzi::{self, DziFile};
use microscope_core::mosaic::Layout;
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

/// A stitched mosaic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Stitched {
    /// Its size, in pixels: width and height.
    pub size: [u32; 2],
    /// Its deep-zoom levels.
    pub levels: u32,
    /// How many images it's made of: one a site.
    pub images: u32,
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

/// Where `sites` went in `layout`, numbered in the order they were taken.
fn placed(sites: &[Site], layout: &Layout) -> TileLayout {
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
        })
        .collect();
    TileLayout {
        size: layout.size,
        tiles,
    }
}

/// Stitches `scan` into its `stitched/` folder, replacing any it had. It's
/// written beside, then moved into place, so it's never seen half done.
pub fn stitch(scan: &Path) -> Result<Stitched, ActionError> {
    let sites = sites(scan)?;
    let layout = layout(&sites, sites[0].size)?;
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
        serde_json::to_vec_pretty(&placed(&sites, &layout)).map_err(ActionError::handled)?;
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
        Ok((jpeg.map_err(ActionError::handled)?, placed(&sites, &layout)))
    }
}
