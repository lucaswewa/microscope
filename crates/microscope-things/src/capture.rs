//! Captures saved to the data folder (ADR-0025): a JPEG in `captures/`,
//! named by when it was taken, and beside it a JSON sidecar of how. Every
//! camera driver saves its captures through here.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{BufWriter, ErrorKind};
use std::path::{Path, PathBuf};

use image::RgbImage;
use image::codecs::jpeg::JpegEncoder;
use microscope_core::mapping::Matrix2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teta_wot::prelude::*;

use crate::hardware::{AxisScale, Capture, Position};
use crate::mapping::CameraStageMapping;

/// The folder, in the data folder, that captures are saved in.
pub const CAPTURES: &str = "captures";
/// The sidecar's layout version.
pub const FORMAT: u32 = 1;
/// The quality captures are saved at.
const QUALITY: u8 = 95;

/// What a capture's sidecar records (ADR-0025).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Sidecar {
    /// The layout's version.
    pub format: u32,
    /// The image's file name, beside the sidecar.
    pub image: String,
    /// When it was taken: RFC 3339, in UTC.
    pub acquired: String,
    /// The image's width, in pixels.
    pub width: u32,
    /// The image's height, in pixels.
    pub height: u32,
    /// How much of the sample a pixel covers, in µm, if known.
    pub um_per_pixel: Option<f64>,
    /// Where the stage was, if the camera knows of one.
    pub stage: Option<StageRecord>,
    /// The camera's settings, by name.
    pub camera: BTreeMap<String, Value>,
    /// The camera–stage mapping, if it's calibrated.
    pub camera_stage_mapping: Option<MappingRecord>,
    /// What decided a simulated frame; null for a real camera.
    pub simulation: Option<Value>,
}

/// Where the stage was.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageRecord {
    /// In steps.
    pub steps: Position,
    /// In µm, on each axis whose scale is known.
    pub um: Microns,
    /// µm per step on each axis, where known.
    pub um_per_step: AxisScale,
}

/// A position in µm, on each axis where it's known.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Microns {
    /// In x.
    pub x: Option<f64>,
    /// In y.
    pub y: Option<f64>,
    /// In z.
    pub z: Option<f64>,
}

/// The camera–stage mapping, as calibrated (P23).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MappingRecord {
    /// Moving the stage by this times a displacement in pixels moves the
    /// field of view by that displacement.
    pub image_to_stage_displacement_matrix: Matrix2,
    /// The frames' size it was calibrated at: (width, height).
    pub image_resolution: [u32; 2],
}

impl Sidecar {
    /// The sidecar of `capture`, saved as `image`.
    pub fn of(capture: &Capture, image: &str, mapping: Option<MappingRecord>) -> Self {
        let metadata = &capture.metadata;
        let stage = metadata.position.map(|steps| {
            let scale = metadata.axis_scale.unwrap_or_default();
            let um = |steps: i64, scale: Option<f64>| scale.map(|s| steps as f64 * s);
            StageRecord {
                steps,
                um: Microns {
                    x: um(steps.x, scale.x),
                    y: um(steps.y, scale.y),
                    z: um(steps.z, scale.z),
                },
                um_per_step: scale,
            }
        });
        Sidecar {
            format: FORMAT,
            image: image.to_owned(),
            acquired: metadata.acquired.clone(),
            width: metadata.width,
            height: metadata.height,
            um_per_pixel: metadata.um_per_pixel,
            stage,
            camera: metadata.camera_settings.clone(),
            camera_stage_mapping: mapping,
            simulation: metadata.simulation.clone(),
        }
    }
}

/// Saves `capture` as a JPEG in `folder`'s `captures`, with its sidecar,
/// under a name no file there has yet: `capture-<time>.jpeg`, then `-2`,
/// `-3`, … for more in the same second. Returns the image's path.
pub fn save(
    folder: &Path,
    capture: &Capture,
    mapping: Option<MappingRecord>,
) -> Result<PathBuf, ActionError> {
    let captures = folder.join(CAPTURES);
    std::fs::create_dir_all(&captures).map_err(ActionError::handled)?;
    let stem = format!("capture-{}", name_time(&capture.metadata.acquired));
    let mut n = 1;
    let (path, name, file) = loop {
        let name = match n {
            1 => format!("{stem}.jpeg"),
            n => format!("{stem}-{n}.jpeg"),
        };
        let path = captures.join(&name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (path, name, file),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => n += 1,
            Err(error) => return Err(ActionError::handled(error)),
        }
    };
    let mut encoder = JpegEncoder::new_with_quality(BufWriter::new(file), QUALITY);
    encoder
        .encode_image(&capture.image)
        .map_err(ActionError::handled)?;
    let sidecar = serde_json::to_vec_pretty(&Sidecar::of(capture, &name, mapping));
    std::fs::write(
        path.with_extension("json"),
        sidecar.map_err(ActionError::handled)?,
    )
    .map_err(ActionError::handled)?;
    Ok(path)
}

/// `image` as a JPEG, at the quality captures are saved at.
pub fn jpeg(image: &RgbImage) -> Result<Vec<u8>, ActionError> {
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, QUALITY);
    encoder.encode_image(image).map_err(ActionError::handled)?;
    Ok(bytes)
}

/// The camera–stage mapping on `server`, if there's one and it's calibrated.
pub fn mapping(server: &Server) -> Option<MappingRecord> {
    let mapping = server
        .things_of_type::<CameraStageMapping>()
        .into_iter()
        .next()?;
    let (matrix, resolution) = ThingRef::thing(&mapping).calibrated()?;
    Some(MappingRecord {
        image_to_stage_displacement_matrix: matrix,
        image_resolution: resolution,
    })
}

/// `acquired` (RFC 3339) as a file name's time: UTC, to the second, with
/// dashes for colons, such as `2026-10-11T07-35-12Z`.
fn name_time(acquired: &str) -> String {
    let time = chrono::DateTime::parse_from_rfc3339(acquired).map_or_else(
        |_| chrono::Utc::now(),
        |time| time.with_timezone(&chrono::Utc),
    );
    time.format("%Y-%m-%dT%H-%M-%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_utc_to_the_second() {
        assert_eq!(
            name_time("2026-10-11T09:35:12.345+02:00"),
            "2026-10-11T07-35-12Z"
        );
    }
}
