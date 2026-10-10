//! A camera's live preview, shared by every camera driver (ADR-0020).
//!
//! Each frame goes, as a JPEG, to the camera's main MJPEG stream, and a
//! small copy to its low-resolution one. The frame itself is kept, so that
//! Things analysing the image get pixels, not a JPEG to decode.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, RgbImage};
use teta_wot::prelude::*;
use teta_wot::stream::StreamError;

/// The low-resolution stream's frame size: (width, height).
pub const LORES_SIZE: (u32, u32) = (320, 240);

/// The JPEG quality of streamed frames, from 1 to 100.
const QUALITY: u8 = 85;

/// The latest preview frame, kept as pixels.
#[derive(Debug, Default)]
pub struct PreviewFrames {
    latest: Mutex<Option<Arc<RgbImage>>>,
}

impl PreviewFrames {
    /// Publishes `frame`, taken at `taken`, to `main`, and a copy shrunk to
    /// [`LORES_SIZE`] to `lores`, keeping it as the latest. It works from any
    /// thread.
    pub fn publish(
        &self,
        frame: RgbImage,
        taken: DateTime<Utc>,
        main: &MjpegStream,
        lores: &MjpegStream,
    ) -> Result<(), StreamError> {
        let jpeg = encode_jpeg(&frame);
        // Through `DynamicImage`, whose methods aren't generic, so they run
        // as `image` was compiled: optimised, even in debug builds.
        let frame = DynamicImage::ImageRgb8(frame);
        let small = frame.thumbnail_exact(LORES_SIZE.0, LORES_SIZE.1);
        let small_jpeg = encode_jpeg(&small.into_rgb8());
        let frame = frame.into_rgb8();
        // Kept before it is added, so anyone woken by the new frame finds it.
        *self.latest.lock().expect("not poisoned") = Some(Arc::new(frame));
        main.add_frame_at(jpeg, taken)?;
        lores.add_frame_at(small_jpeg, taken)?;
        Ok(())
    }

    /// The next frame published to `main`, waiting for it: one that was
    /// drawn after the call, unless a frame was being drawn at the time.
    pub async fn next(&self, main: &MjpegStream) -> Result<Arc<RgbImage>, StreamError> {
        main.next_frame().await?;
        let latest = self.latest.lock().expect("not poisoned").clone();
        Ok(latest.expect("a frame is kept before it is added"))
    }
}

/// `image` as a JPEG, at the streams' quality.
pub fn encode_jpeg(image: &RgbImage) -> Vec<u8> {
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, QUALITY)
        .encode_image(image)
        .expect("an 8-bit RGB image of this size encodes");
    jpeg
}
