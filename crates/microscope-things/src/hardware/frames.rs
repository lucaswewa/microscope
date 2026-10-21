//! What every SDK camera shares (ADR-0043): its frames, from the SDK's
//! callback, to the preview and to captures.
//!
//! - **Frames** arrive on the SDK's thread, through
//!   [`FramePipeline::deliver`]. At most one every [`PREVIEW_INTERVAL`] is
//!   converted to 8-bit RGB for the preview, shrunk to fit [`PREVIEW_MAX`]:
//!   the main stream, the low-resolution one, and `grab_frame`, as with the
//!   simulator (ADR-0020).
//! - **A capture** takes the next frame whole, at the sensor's resolution,
//!   with the metadata the camera gives it: its settings, and where the
//!   stage was (ADR-0025).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, SecondsFormat, Utc};
use image::{DynamicImage, RgbImage};
use microscope_core::pixels::{RawFrame, to_rgb8};
use teta_wot::prelude::*;
use tokio::sync::oneshot;

use super::camera::{Capture, CaptureMetadata};
use super::preview::PreviewFrames;

/// The most often the preview takes a frame.
pub const PREVIEW_INTERVAL: Duration = Duration::from_millis(100);
/// The largest preview frame: a larger one is shrunk to fit, keeping its
/// shape.
pub const PREVIEW_MAX: (u32, u32) = (1024, 1024);
/// How long a capture waits for the camera's next frame.
pub const CAPTURE_WAIT: Duration = Duration::from_secs(5);

/// A whole frame, and when it was taken.
type Whole = (RgbImage, DateTime<Utc>);

/// An SDK camera's frames, on their way to the preview and to captures.
#[derive(Debug, Default)]
pub struct FramePipeline {
    preview: PreviewFrames,
    /// When the preview last took a frame, and its size.
    last_preview: Mutex<Option<(Instant, (u32, u32))>>,
    /// When the SDK last delivered a frame.
    delivered: Mutex<Option<Instant>>,
    /// The captures waiting for the next frame.
    waiting: Mutex<Vec<oneshot::Sender<Result<Whole, String>>>>,
}

impl FramePipeline {
    /// Takes `frame`, taken at `taken`, from the SDK's callback: for the
    /// captures waiting, and for the preview, `main` and `lores`, if it's
    /// due. A frame that can't be converted is an error, for them too.
    pub fn deliver(
        &self,
        frame: &RawFrame,
        taken: DateTime<Utc>,
        main: &MjpegStream,
        lores: &MjpegStream,
    ) -> Result<(), String> {
        *self.delivered.lock().expect("not poisoned") = Some(Instant::now());
        let waiting = std::mem::take(&mut *self.waiting.lock().expect("not poisoned"));
        let last = *self.last_preview.lock().expect("not poisoned");
        let due = last.is_none_or(|(at, _)| at.elapsed() >= PREVIEW_INTERVAL);
        if waiting.is_empty() && !due {
            return Ok(());
        }
        let image = match to_rgb8(frame) {
            Ok(image) => image,
            Err(error) => {
                waiting.into_iter().for_each(|capture| {
                    let _ = capture.send(Err(error.clone()));
                });
                return Err(error);
            }
        };
        for capture in waiting {
            let _ = capture.send(Ok((image.clone(), taken)));
        }
        if !due {
            return Ok(());
        }
        let preview = shrink(image, PREVIEW_MAX);
        let size = preview.dimensions();
        *self.last_preview.lock().expect("not poisoned") = Some((Instant::now(), size));
        (self.preview.publish(preview, taken, main, lores)).map_err(|error| error.to_string())
    }

    /// The next preview frame published to `main`, for analysis.
    pub async fn grab_frame(&self, main: &MjpegStream) -> Result<RgbImage, ActionError> {
        let frame = self
            .preview
            .next(main)
            .await
            .map_err(ActionError::handled)?;
        Ok(RgbImage::clone(&frame))
    }

    /// When the SDK last delivered a frame, if it has.
    pub fn last_delivered(&self) -> Option<Instant> {
        *self.delivered.lock().expect("not poisoned")
    }

    /// The preview's frame size, once it has had a frame.
    pub fn preview_size(&self) -> Option<(u32, u32)> {
        let last = *self.last_preview.lock().expect("not poisoned");
        last.map(|(_, size)| size)
    }

    /// A capture: the camera's next frame, whole, with `metadata`, whose
    /// time and size are the frame's.
    pub async fn capture(&self, metadata: CaptureMetadata) -> Result<Capture, ActionError> {
        let (sender, receiver) = oneshot::channel();
        self.waiting.lock().expect("not poisoned").push(sender);
        let next = tokio::time::timeout(CAPTURE_WAIT, receiver).await;
        let Ok(Ok(taken)) = next else {
            return Err(ActionError::handled(format!(
                "the camera gave no frame within {CAPTURE_WAIT:?}: is it acquiring?"
            )));
        };
        let (image, taken) = taken.map_err(ActionError::handled)?;
        let (width, height) = image.dimensions();
        let acquired = taken.to_rfc3339_opts(SecondsFormat::Millis, true);
        let metadata = CaptureMetadata {
            acquired,
            width,
            height,
            ..metadata
        };
        Ok(Capture { image, metadata })
    }
}

/// `image`, shrunk to fit `(width, height)` if it's larger.
fn shrink(image: RgbImage, (width, height): (u32, u32)) -> RgbImage {
    if image.width() <= width && image.height() <= height {
        return image;
    }
    // Through `DynamicImage`, which runs optimised even in debug builds.
    DynamicImage::ImageRgb8(image)
        .thumbnail(width, height)
        .into_rgb8()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use microscope_core::pixels::PixelFormat;

    use super::*;

    fn grey(width: u32, height: u32, level: u8) -> Vec<u8> {
        vec![level; (width * height) as usize]
    }

    fn mono<'a>(width: u32, height: u32, data: &'a [u8]) -> RawFrame<'a> {
        RawFrame {
            width,
            height,
            format: PixelFormat::Mono8,
            data,
        }
    }

    #[tokio::test]
    async fn the_preview_takes_a_frame_at_most_every_interval() {
        let (pipeline, main, lores) = (
            FramePipeline::default(),
            MjpegStream::new(),
            MjpegStream::new(),
        );
        // Small frames, which convert at once, whatever the build.
        let data = grey(64, 48, 90);
        for _ in 0..5 {
            (pipeline.deliver(&mono(64, 48, &data), Utc::now(), &main, &lores)).expect("taken");
        }
        assert_eq!(main.latest_frame().expect("a frame").index, 0);
        assert!(lores.latest_frame().is_some());
        std::thread::sleep(PREVIEW_INTERVAL);
        (pipeline.deliver(&mono(64, 48, &data), Utc::now(), &main, &lores)).expect("taken");
        assert_eq!(main.latest_frame().expect("a frame").index, 1);
    }

    #[tokio::test]
    async fn a_large_frame_is_shrunk_to_fit_the_preview() {
        let (pipeline, main, lores) = (
            FramePipeline::default(),
            MjpegStream::new(),
            MjpegStream::new(),
        );
        let data = grey(2048, 1536, 90);
        (pipeline.deliver(&mono(2048, 1536, &data), Utc::now(), &main, &lores)).expect("taken");
        assert_eq!(pipeline.preview_size(), Some((1024, 768)));
    }

    #[tokio::test]
    async fn a_capture_takes_the_next_frame_whole() {
        let pipeline = Arc::new(FramePipeline::default());
        let (main, lores) = (MjpegStream::new(), MjpegStream::new());
        let data = grey(2048, 1536, 200);
        let settings = [("exposure_time".to_owned(), serde_json::json!(5000.0))];
        let metadata = CaptureMetadata {
            camera_settings: settings.into_iter().collect(),
            ..CaptureMetadata::default()
        };
        let capturing = tokio::spawn({
            let pipeline = Arc::clone(&pipeline);
            async move { pipeline.capture(metadata).await }
        });
        let taken = Utc::now();
        while pipeline.waiting.lock().expect("ok").is_empty() {
            tokio::task::yield_now().await;
        }
        (pipeline.deliver(&mono(2048, 1536, &data), taken, &main, &lores)).expect("taken");
        let capture = capturing.await.expect("joined").expect("captured");
        assert_eq!(capture.image.dimensions(), (2048, 1536));
        assert_eq!(capture.image.get_pixel(0, 0).0, [200; 3]);
        let metadata = capture.metadata;
        assert_eq!((metadata.width, metadata.height), (2048, 1536));
        assert_eq!(
            metadata.acquired,
            taken.to_rfc3339_opts(SecondsFormat::Millis, true)
        );
        assert_eq!(metadata.camera_settings["exposure_time"], 5000.0);
    }

    #[tokio::test]
    async fn a_camera_without_frames_says_so_once_it_has_waited() {
        let refused = FramePipeline::default()
            .capture(CaptureMetadata::default())
            .await;
        let refused = refused.expect_err("no frame").to_string();
        assert!(refused.contains("is it acquiring?"), "{refused}");
    }

    #[tokio::test]
    async fn a_frame_that_cant_be_converted_fails_the_capture_too() {
        let pipeline = Arc::new(FramePipeline::default());
        let capturing = tokio::spawn({
            let pipeline = Arc::clone(&pipeline);
            async move { pipeline.capture(CaptureMetadata::default()).await }
        });
        while pipeline.waiting.lock().expect("ok").is_empty() {
            tokio::task::yield_now().await;
        }
        let (main, lores) = (MjpegStream::new(), MjpegStream::new());
        let short = mono(4, 4, &[0; 3]);
        assert!(pipeline.deliver(&short, Utc::now(), &main, &lores).is_err());
        let failed = capturing
            .await
            .expect("joined")
            .expect_err("failed")
            .to_string();
        assert!(failed.contains("needs 16 bytes"), "{failed}");
    }
}
