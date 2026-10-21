//! Test fakes: a Thing for each hardware interface, simple enough to reason
//! about, for testing the Things that use hardware. They are never in the
//! server's [`registry()`](crate::registry()); tests add them with [`register`].
//!
//! - [`FakeStage`] moves at once and remembers every move;
//! - [`FakeCamera`] returns plain grey frames of the size it is configured
//!   with, and counts its captures;
//! - [`FakeIllumination`] remembers whether it is on;
//! - [`FakeSdkCamera`], not a Thing, delivers frames to a callback, as an
//!   SDK does;
//! - [`FakeCameraSdk`], also not a Thing, is a camera's SDK for `SdkCamera`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use chrono::{DateTime, Utc};
use microscope_core::pixels::{PixelFormat, RawFrame};

use image::{Rgb, RgbImage};
use schemars::JsonSchema;
use serde::Deserialize;
use teta_wot::prelude::*;
use teta_wot::server::ThingRegistry;

use crate::hardware::{
    AxisScale, CameraApi, Capture, CaptureMetadata, CaptureMode, Feature, FeatureKind,
    IlluminationApi, Position, STANDARD_FEATURES, StageApi, StreamInfo,
};
use crate::sdk_camera::{CameraSdk, Deliver};

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

    fn set_zero(&self) -> BoxFuture<'_, Result<(), ActionError>> {
        let zeroed = ThingRef::thing(self).position.set(Position::default());
        Box::pin(async move { zeroed.map_err(ActionError::handled) })
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

    fn capture_to_memory(&self, _mode: CaptureMode) -> BoxFuture<'_, Result<Capture, ActionError>> {
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

/// A fake SDK camera, for tests: a thread that delivers frames to a
/// callback, as an SDK's does, until it's dropped. A frame's first bytes are
/// its number, from 0, so a test can tell frames apart, and each pair of
/// bytes after them rises by one, for 64 pairs, then starts again: a
/// texture, as a slide's image has.
pub struct FakeSdkCamera {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeSdkCamera {
    /// Delivers a `width` × `height` frame in `format` to `deliver` every
    /// `interval`.
    pub fn start(
        (width, height): (u32, u32),
        format: PixelFormat,
        interval: Duration,
        mut deliver: impl FnMut(&RawFrame, DateTime<Utc>) + Send + 'static,
    ) -> FakeSdkCamera {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::spawn({
            let stop = Arc::clone(&stop);
            move || {
                let size = width as usize * height as usize * format.bytes_per_pixel();
                for number in (0..=u8::MAX).cycle() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let data: Vec<u8> = (0..size)
                        .map(|i| number.wrapping_add(((i / 2) % 64) as u8))
                        .collect();
                    deliver(
                        &RawFrame {
                            width,
                            height,
                            format,
                            data: &data,
                        },
                        Utc::now(),
                    );
                    std::thread::sleep(interval);
                }
            }
        });
        let thread = Some(thread);
        FakeSdkCamera { stop, thread }
    }
}

impl Drop for FakeSdkCamera {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A fake camera SDK, for testing `SdkCamera` without a camera: a mono
/// camera, its features kept by name, sending frames from a
/// [`FakeSdkCamera`]. Its clones share it, so a test keeps one to look at it,
/// or to unplug it.
#[derive(Clone)]
pub struct FakeCameraSdk(Arc<Mutex<FakeSdkState>>);

struct FakeSdkState {
    size: (u32, u32),
    features: BTreeMap<String, String>,
    camera: Option<FakeSdkCamera>,
    deliver: Option<Arc<Mutex<Deliver>>>,
    starts: u32,
}

/// The pixel formats it offers: `Mono12p` is packed, which can't be shown.
const FAKE_FORMATS: [&str; 3] = ["Mono8", "Mono12", "Mono12p"];

impl FakeCameraSdk {
    /// A camera sending `size` frames in `Mono8`, exposed for 1000 µs at no
    /// gain, with automatic exposure, which sets 2000 µs.
    pub fn new(size: (u32, u32)) -> FakeCameraSdk {
        let features = [
            ("ExposureTime", "1000"),
            ("Gain", "0"),
            ("PixelFormat", "Mono8"),
            ("ExposureAuto", "Off"),
        ];
        let features = features.map(|(name, value)| (name.to_owned(), value.to_owned()));
        let state = FakeSdkState {
            size,
            features: features.into(),
            camera: None,
            deliver: None,
            starts: 0,
        };
        FakeCameraSdk(Arc::new(Mutex::new(state)))
    }

    /// The feature `name`'s value now.
    pub fn feature(&self, name: &str) -> Option<String> {
        self.state().features.get(name).cloned()
    }

    /// How many times it has started grabbing.
    pub fn starts(&self) -> u32 {
        self.state().starts
    }

    /// Stops its frames, as a camera unplugged does, until it next starts.
    pub fn unplug(&self) {
        drop(self.state().camera.take());
    }

    fn state(&self) -> std::sync::MutexGuard<'_, FakeSdkState> {
        self.0.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// Sends frames in its pixel format, if they can be.
    fn grab(state: &mut FakeSdkState) {
        drop(state.camera.take());
        let format = PixelFormat::from_genicam(&state.features["PixelFormat"]);
        let (Some(deliver), Some(format)) = (state.deliver.clone(), format) else {
            return;
        };
        let interval = Duration::from_millis(20);
        let camera = FakeSdkCamera::start(state.size, format, interval, move |frame, taken| {
            (deliver.lock().unwrap_or_else(|error| error.into_inner()))(frame, taken);
        });
        state.camera = Some(camera);
    }
}

impl CameraSdk for FakeCameraSdk {
    fn describe(&self) -> String {
        "a fake camera".to_owned()
    }

    fn has(&mut self, name: &str) -> bool {
        self.state().features.contains_key(name)
    }

    fn features(&mut self) -> Vec<Feature> {
        (STANDARD_FEATURES.iter())
            .filter(|(name, ..)| self.has(name))
            .map(|(name, ..)| Feature {
                name: (*name).to_owned(),
                kind: match *name {
                    "PixelFormat" => FeatureKind::Choice(FAKE_FORMATS.map(String::from).to_vec()),
                    _ => FeatureKind::Number { step: None },
                },
            })
            .collect()
    }

    fn get(&mut self, name: &str) -> Result<String, String> {
        let value = self.state().features.get(name).cloned();
        value.ok_or_else(|| format!("the camera has no {name}"))
    }

    fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        let mut state = self.state();
        if !state.features.contains_key(name) {
            return Err(format!("the camera has no {name}"));
        }
        if name == "PixelFormat" && !FAKE_FORMATS.contains(&value) {
            return Err(format!("the camera can't send {value}"));
        }
        let (name, value) = match (name, value) {
            ("ExposureAuto", "Once") => ("ExposureTime", "2000"),
            other => other,
        };
        state.features.insert(name.to_owned(), value.to_owned());
        if name == "PixelFormat" && state.camera.is_some() {
            FakeCameraSdk::grab(&mut state);
        }
        Ok(())
    }

    fn start(&mut self, deliver: Deliver) -> Result<(), String> {
        let mut state = self.state();
        state.starts += 1;
        state.deliver = Some(Arc::new(Mutex::new(deliver)));
        FakeCameraSdk::grab(&mut state);
        Ok(())
    }

    fn stop(&mut self) {
        drop(self.state().camera.take());
    }
}
