//! Test fakes: a Thing for each hardware interface, simple enough to reason
//! about, for testing the Things that use hardware. They are never in the
//! server's [`registry()`](crate::registry()); tests add them with [`register`].
//!
//! - [`FakeStage`] moves at once and remembers every move;
//! - [`FakeCamera`] returns plain grey frames of the size it is configured
//!   with, and counts its captures;
//! - [`FakeIllumination`] remembers whether it is on.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use image::{Rgb, RgbImage};
use schemars::JsonSchema;
use serde::Deserialize;
use teta_wot::prelude::*;
use teta_wot::server::ThingRegistry;

use crate::hardware::{
    AxisScale, CameraApi, Capture, CaptureMetadata, IlluminationApi, Position, StageApi, StreamInfo,
};

/// `registry` with the fakes added, under `microscope.fakes:<Type>`.
pub fn register(registry: ThingRegistry) -> ThingRegistry {
    registry
        .register::<FakeStage>("microscope.fakes:FakeStage")
        .register::<FakeCamera>("microscope.fakes:FakeCamera")
        .register::<FakeIllumination>("microscope.fakes:FakeIllumination")
}

/// A stage for tests: it moves at once, 0.1 µm per step, and remembers every
/// target it was sent to.
#[derive(Thing)]
#[thing(interfaces(StageApi))]
pub struct FakeStage {
    /// Where the stage is.
    #[property(default = Position::default(), readonly)]
    position: Prop<Position>,
    targets: Mutex<Vec<Position>>,
}

#[thing_impl]
impl FakeStage {}

impl FakeStage {
    /// Every target it was sent to, in order.
    pub fn targets(&self) -> Vec<Position> {
        self.targets.lock().expect("not poisoned").clone()
    }

    fn go(&self, target: Position) -> Result<Position, ActionError> {
        self.position.set(target).map_err(ActionError::handled)?;
        self.targets.lock().expect("not poisoned").push(target);
        Ok(target)
    }
}

impl StageApi for ThingRef<FakeStage> {
    fn position(&self) -> BoxFuture<'_, Result<Position, ActionError>> {
        let position = ThingRef::thing(self).position.get();
        Box::pin(async move { Ok(position) })
    }

    fn move_relative(
        &self,
        displacement: Position,
    ) -> BoxFuture<'_, Result<Position, ActionError>> {
        let stage = ThingRef::thing(self);
        let result = stage.go(stage.position.get() + displacement);
        Box::pin(async move { result })
    }

    fn move_absolute(&self, target: Position) -> BoxFuture<'_, Result<Position, ActionError>> {
        let result = ThingRef::thing(self).go(target);
        Box::pin(async move { result })
    }

    fn stop(&self) -> BoxFuture<'_, Result<(), ActionError>> {
        Box::pin(async { Ok(()) })
    }

    fn axis_scale(&self) -> AxisScale {
        AxisScale::uniform(0.1)
    }
}

/// How big a [`FakeCamera`]'s frames are.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FakeCameraConfig {
    /// The frames' width, in pixels.
    #[serde(default = "default_width")]
    pub width: u32,
    /// The frames' height, in pixels.
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 {
    64
}

fn default_height() -> u32 {
    48
}

/// A camera for tests: plain grey frames of its configured size (64 × 48
/// unless configured), and captures twice that size.
#[derive(Thing)]
#[thing(config = FakeCameraConfig, interfaces(CameraApi))]
pub struct FakeCamera {
    #[thing(init = config.width)]
    width: u32,
    #[thing(init = config.height)]
    height: u32,
    captures: AtomicU32,
    /// Never sends a frame.
    lores: MjpegStream,
}

#[thing_impl]
impl FakeCamera {}

impl FakeCamera {
    /// How many captures it has taken.
    pub fn captures(&self) -> u32 {
        self.captures.load(Ordering::SeqCst)
    }

    fn frame(&self, scale: u32) -> RgbImage {
        RgbImage::from_pixel(
            self.width * scale,
            self.height * scale,
            Rgb([128, 128, 128]),
        )
    }
}

impl CameraApi for ThingRef<FakeCamera> {
    fn grab_frame(&self) -> BoxFuture<'_, Result<RgbImage, ActionError>> {
        let frame = ThingRef::thing(self).frame(1);
        Box::pin(async move { Ok(frame) })
    }

    fn grab_jpeg(&self) -> BoxFuture<'_, Result<Vec<u8>, ActionError>> {
        // Not a real JPEG: just its start-of-image and end-of-image markers.
        Box::pin(async { Ok(vec![0xFF, 0xD8, 0xFF, 0xD9]) })
    }

    fn frame_size(&self) -> (u32, u32) {
        let camera = ThingRef::thing(self);
        (camera.width, camera.height)
    }

    fn capture_to_memory(&self) -> BoxFuture<'_, Result<Capture, ActionError>> {
        let camera = ThingRef::thing(self);
        camera.captures.fetch_add(1, Ordering::SeqCst);
        let image = camera.frame(2);
        let metadata = CaptureMetadata {
            acquired: "2026-01-01T00:00:00Z".to_owned(),
            width: image.width(),
            height: image.height(),
            ..CaptureMetadata::default()
        };
        Box::pin(async move { Ok(Capture { image, metadata }) })
    }

    fn stream_info(&self) -> StreamInfo {
        let (width, height) = self.frame_size();
        StreamInfo {
            affordance: "mjpeg_stream".to_owned(),
            width,
            height,
        }
    }

    fn lores_stream(&self) -> MjpegStream {
        ThingRef::thing(self).lores.clone()
    }
}

/// A light for tests: it remembers whether it is on.
#[derive(Thing)]
#[thing(interfaces(IlluminationApi))]
pub struct FakeIllumination {
    on: AtomicBool,
}

#[thing_impl]
impl FakeIllumination {}

impl IlluminationApi for ThingRef<FakeIllumination> {
    fn set_led(&self, on: bool) -> BoxFuture<'_, Result<(), ActionError>> {
        ThingRef::thing(self).on.store(on, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }

    fn led_on(&self) -> bool {
        ThingRef::thing(self).on.load(Ordering::SeqCst)
    }
}
