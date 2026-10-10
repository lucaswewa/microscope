//! What other Things need from a camera.

use std::collections::BTreeMap;

use image::RgbImage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teta_wot::prelude::*;

use super::stage::{AxisScale, Position};

/// What a capture records about how it was taken, saved with the image.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct CaptureMetadata {
    /// When the image was taken: RFC 3339, in UTC.
    pub acquired: String,
    /// The image's width, in pixels.
    pub width: u32,
    /// The image's height, in pixels.
    pub height: u32,
    /// Where the stage was, if the camera knows of a stage.
    pub position: Option<Position>,
    /// The stage's scale at the time, if known.
    pub axis_scale: Option<AxisScale>,
    /// How much of the sample one pixel covers, in µm, if known.
    pub um_per_pixel: Option<f64>,
    /// The camera's settings at the time, by name: exposure, gain, and
    /// whatever else the driver reports.
    pub camera_settings: BTreeMap<String, Value>,
}

/// A full-resolution image held in memory, with its metadata.
#[derive(Debug, Clone)]
pub struct Capture {
    /// The image.
    pub image: RgbImage,
    /// How it was taken.
    pub metadata: CaptureMetadata,
}

/// What the camera's live stream carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StreamInfo {
    /// The affordance in the camera's Thing Description that serves it,
    /// such as `mjpeg_stream`.
    pub affordance: String,
    /// Its frames' width, in pixels.
    pub width: u32,
    /// Its frames' height, in pixels.
    pub height: u32,
}

/// What other Things need from a camera, whatever drives it: frames to
/// analyse, JPEGs to show or keep, and full-resolution captures.
#[teta_wot::interface]
pub trait CameraApi: Send + Sync {
    /// The latest preview frame, for analysis: sharpness, background
    /// detection, camera–stage mapping.
    fn grab_frame(&self) -> BoxFuture<'_, Result<RgbImage, ActionError>>;

    /// The latest preview frame, as a JPEG.
    fn grab_jpeg(&self) -> BoxFuture<'_, Result<Vec<u8>, ActionError>>;

    /// The size of the frames `grab_frame` will return next: (width, height).
    fn frame_size(&self) -> (u32, u32);

    /// A full-resolution image, kept in memory, with its metadata.
    fn capture_to_memory(&self) -> BoxFuture<'_, Result<Capture, ActionError>>;

    /// The live stream, as it is now.
    fn stream_info(&self) -> StreamInfo;
}
