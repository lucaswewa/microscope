//! The `camera` Thing on an SDK's camera (P07, ADR-0044): `SdkCamera`, for
//! Basler's cameras through pylon, and Allied Vision's, through Vimba X, in
//! P08.
//!
//! - **The SDK** is behind [`CameraSdk`], so the Thing is the same for each.
//!   The configuration names the SDK, and the camera by its serial number,
//!   or else the first one found.
//! - **Frames** go through P06's [`FramePipeline`] to both streams, to
//!   autofocus and the other Things, and to captures at full resolution.
//! - **Settings** are the standard features ([`STANDARD_FEATURES`]), read
//!   from the camera and written to it; the panel shows those it has. If it
//!   sends pixels in a format that can't be converted, a format that can is
//!   chosen when it starts.
//! - **Calibration:** "Full auto-calibrate" runs the camera's own automatic
//!   exposure and white balance once, where it has them, then sets the
//!   background detector's background. An automatic setting that doesn't
//!   settle within 10 s is turned off, and logged.
//! - **Disconnects:** a watchdog notices when frames stop for
//!   [`STALLED_AFTER`], says so, and opens the camera again every few
//!   seconds until it's back. `reconnect` does so at once.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use image::{DynamicImage, ImageFormat, RgbImage};
use microscope_core::pixels::{PixelFormat, RawFrame};
use microscope_sdk::pylon::{Grabbing, Pylon, PylonCamera};
use serde::Deserialize;
use serde_json::{Value, json};
use teta_wot::blob::Jpeg;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;
use tokio::task::JoinHandle;

use crate::camera::{DEFAULT_DETECTOR, SampleCheck};
use crate::drivers::device_error;
use crate::fakes::FakeCameraSdk;
use crate::hardware::{
    BackgroundDetector, CameraApi, Capture, CaptureBuffer, CaptureMetadata, CaptureMode, Feature,
    FeatureKind, FramePipeline, LORES_SIZE, STANDARD_FEATURES, StageApi, StreamInfo,
    settings_panel,
};
use crate::ui::{OwnEntry, Success, UiElement};
use crate::{capture, data};

/// How long without a frame before the camera is opened again.
pub const STALLED_AFTER: Duration = Duration::from_secs(5);
/// How long an automatic setting may take to settle.
const AUTO_WAIT: Duration = Duration::from_secs(10);

/// What `SdkCamera` asks of a camera's SDK. Features are by their GenICam
/// names, and their values text, as the SDKs read and write them.
pub trait CameraSdk {
    /// Which camera it is, for the log.
    fn describe(&self) -> String;
    /// Whether the camera has the feature `name`.
    fn has(&mut self, name: &str) -> bool;
    /// Which of the standard features it has, and how.
    fn features(&mut self) -> Vec<Feature>;
    /// The feature `name`'s value.
    fn get(&mut self, name: &str) -> Result<String, String>;
    /// Sets the feature `name` to `value`.
    fn set(&mut self, name: &str, value: &str) -> Result<(), String>;
    /// Starts grabbing, giving each frame to `deliver`.
    fn start(&mut self, deliver: Deliver) -> Result<(), String>;
    /// Stops grabbing.
    fn stop(&mut self);
}

/// Where an SDK delivers its frames, on its own thread.
pub type Deliver = Box<dyn FnMut(&RawFrame, DateTime<Utc>) + Send>;

/// The SDK behind a camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sdk {
    /// Basler's pylon.
    Pylon,
}

/// How an `SdkCamera` is configured.
#[derive(Deserialize)]
pub struct SdkCameraConfig {
    /// Its SDK.
    pub sdk: Sdk,
    /// The camera's serial number, as `--list-devices` shows it; without
    /// one, the first camera the SDK finds.
    #[serde(default)]
    pub serial_number: Option<String>,
    /// A fake SDK in place of the real one, for tests.
    #[serde(skip)]
    pub fake: Option<FakeCameraSdk>,
}

/// The camera's SDK, as a `teta-wot` driver on the device's thread.
pub struct SdkDriver(Box<dyn CameraSdk>);

impl Driver for SdkDriver {
    fn close(&mut self) -> anyhow::Result<()> {
        self.0.stop();
        Ok(())
    }
}

fn connect(config: &SdkCameraConfig) -> anyhow::Result<SdkDriver> {
    let sdk: Box<dyn CameraSdk> = match (&config.fake, config.sdk) {
        (Some(fake), _) => Box::new(fake.clone()),
        (None, Sdk::Pylon) => Box::new(PylonSdk::open(config.serial_number.as_deref())?),
    };
    tracing::info!("the camera is {}", sdk.describe());
    Ok(SdkDriver(sdk))
}

/// A Basler camera, through pylon.
struct PylonSdk {
    camera: PylonCamera,
    grabbing: Option<Grabbing>,
    /// Where frames go: kept, to grab again after a change of format.
    deliver: Option<Arc<Mutex<Deliver>>>,
}

impl PylonSdk {
    fn open(serial_number: Option<&str>) -> Result<PylonSdk, microscope_sdk::SdkError> {
        let camera = Pylon::load()?.open(serial_number)?;
        Ok(PylonSdk {
            camera,
            grabbing: None,
            deliver: None,
        })
    }

    /// The camera's own name for `name`: older cameras call the exposure
    /// `ExposureTimeAbs`.
    fn named<'a>(&self, name: &'a str) -> &'a str {
        match name {
            "ExposureTime" if !self.camera.has(name) => "ExposureTimeAbs",
            name => name,
        }
    }

    fn grab(&mut self) -> Result<(), String> {
        let Some(deliver) = self.deliver.clone() else {
            return Ok(());
        };
        let grabbing = self.camera.grab(move |grabbed| {
            let Some(format) = PixelFormat::from_genicam(grabbed.pixel_format) else {
                return;
            };
            let frame = RawFrame {
                width: grabbed.width,
                height: grabbed.height,
                format,
                data: grabbed.data,
            };
            (deliver.lock().expect("not poisoned"))(&frame, Utc::now());
        });
        self.grabbing = Some(grabbing.map_err(|error| error.to_string())?);
        Ok(())
    }
}

impl CameraSdk for PylonSdk {
    fn describe(&self) -> String {
        let info = self.camera.info();
        format!(
            "a Basler {}, serial number {}",
            info.model, info.serial_number
        )
    }

    fn has(&mut self, name: &str) -> bool {
        self.camera.has(self.named(name))
    }

    fn features(&mut self) -> Vec<Feature> {
        let names: Vec<&str> = STANDARD_FEATURES.iter().map(|(name, ..)| *name).collect();
        let names: Vec<&str> = names.into_iter().filter(|name| self.has(name)).collect();
        (names.iter())
            .map(|name| {
                let kind = match *name {
                    "PixelFormat" | "BalanceWhiteAuto" => {
                        FeatureKind::Choice(self.camera.choices(name).unwrap_or_default())
                    }
                    _ => FeatureKind::Number { step: None },
                };
                let name = (*name).to_owned();
                Feature { name, kind }
            })
            .collect()
    }

    fn get(&mut self, name: &str) -> Result<String, String> {
        (self.camera.get(self.named(name))).map_err(|error| error.to_string())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        // The frame's size and format can't change while grabbing.
        let restart = matches!(name, "PixelFormat" | "BinningHorizontal");
        if restart {
            drop(self.grabbing.take());
        }
        let mut set = (self.camera.set(self.named(name), value)).map_err(|error| error.to_string());
        if set.is_ok() && name == "BinningHorizontal" && self.camera.has("BinningVertical") {
            set = (self.camera.set("BinningVertical", value)).map_err(|error| error.to_string());
        }
        if restart {
            self.grab()?;
        }
        set
    }

    fn start(&mut self, deliver: Deliver) -> Result<(), String> {
        self.deliver = Some(Arc::new(Mutex::new(deliver)));
        self.grab()
    }

    fn stop(&mut self) {
        drop(self.grabbing.take());
    }
}

/// A camera through its SDK.
#[derive(Thing)]
#[thing(config = SdkCameraConfig, interfaces(CameraApi))]
pub struct SdkCamera {
    /// The stage, for where a capture was taken, if there is one.
    #[slot]
    stage: OptSlot<dyn StageApi>,
    /// The background detectors the camera can use, by name.
    #[slot]
    background_detectors: SlotMap<dyn BackgroundDetector>,
    /// The live preview.
    #[stream]
    mjpeg_stream: MjpegStream,
    /// The live preview, at 320 × 240.
    #[stream]
    lores_mjpeg_stream: MjpegStream,
    #[device(init = connect(config))]
    device: Device<SdkDriver>,
    pipeline: Arc<FramePipeline>,
    captures: CaptureBuffer,
    /// The background detector named in the setting, if one is.
    detector_name: Mutex<Option<String>>,
    watchdog: Mutex<Option<JoinHandle<()>>>,
    entry: OwnEntry,
}

#[thing_impl]
impl SdkCamera {
    /// How long each frame is exposed, in µs.
    #[property]
    async fn exposure_time(&self) -> Result<f64, PropertyError> {
        self.number("ExposureTime").await
    }

    #[setter(exposure_time)]
    async fn set_exposure_time(&self, value: f64) -> Result<(), PropertyError> {
        self.write("ExposureTime", value.to_string()).await
    }

    /// The sensor's gain, in dB.
    #[property]
    async fn gain(&self) -> Result<f64, PropertyError> {
        self.number("Gain").await
    }

    #[setter(gain)]
    async fn set_gain(&self, value: f64) -> Result<(), PropertyError> {
        self.write("Gain", value.to_string()).await
    }

    /// How the camera sends its pixels, by GenICam's name.
    #[property]
    async fn pixel_format(&self) -> Result<String, PropertyError> {
        self.read("PixelFormat").await
    }

    #[setter(pixel_format)]
    async fn set_pixel_format(&self, value: String) -> Result<(), PropertyError> {
        if PixelFormat::from_genicam(&value).is_none() {
            let message = format!("frames in {value} can't be shown");
            return Err(PropertyError::Failed(anyhow::anyhow!(message)));
        }
        self.write("PixelFormat", value).await
    }

    /// The camera's automatic white balance: `Off`, `Once` or `Continuous`.
    #[property]
    async fn white_balance(&self) -> Result<String, PropertyError> {
        self.read("BalanceWhiteAuto").await
    }

    #[setter(white_balance)]
    async fn set_white_balance(&self, value: String) -> Result<(), PropertyError> {
        self.write("BalanceWhiteAuto", value).await
    }

    /// How many pixels each way the camera adds into one.
    #[property]
    async fn binning(&self) -> Result<i64, PropertyError> {
        let text = self.read("BinningHorizontal").await?;
        (text.parse()).map_err(|_| PropertyError::Failed(anyhow::anyhow!("binning is {text}")))
    }

    #[setter(binning)]
    async fn set_binning(&self, value: i64) -> Result<(), PropertyError> {
        self.write("BinningHorizontal", value.to_string()).await
    }

    /// Which background detector the camera uses, by its Thing's name:
    /// `bg_channel_deviations_luv` unless set, if the server has it, or
    /// else the first.
    #[property]
    async fn background_detector_name(&self) -> Option<String> {
        self.detector().map(|(name, _)| name)
    }

    #[setter(background_detector_name)]
    async fn set_background_detector_name(
        &self,
        name: Option<String>,
    ) -> Result<(), PropertyError> {
        let detectors = self.background_detectors.get();
        if let Some(unknown) =
            (name.as_ref()).filter(|n| !detectors.is_some_and(|d| d.contains_key(*n)))
        {
            let message = format!("no background detector is called {unknown}");
            return Err(PropertyError::Failed(anyhow::anyhow!(message)));
        }
        *self.detector_name.lock().expect("not poisoned") = name;
        Ok(())
    }

    /// Whether the camera needs calibrating: until the background detector
    /// in use has its background.
    #[property]
    async fn calibration_required(&self) -> bool {
        (self.detector()).is_none_or(|(_, detector)| !detector.ready())
    }

    /// Captures the camera's next frame: whole (`standard`), or at the
    /// preview's size (`quick`). With `retain_image`, it's saved to the
    /// data folder's `captures`, with a sidecar of how it was taken
    /// (ADR-0025). Either way, it's returned, to download.
    #[action]
    async fn capture(
        &self,
        server: Server,
        #[param(default)] capture_mode: CaptureMode,
        #[param(default = true)] retain_image: bool,
    ) -> Result<Blob<Jpeg>, ActionError> {
        let taken = self.take(capture_mode).await?;
        if !retain_image {
            return Ok(Blob::from_bytes(capture::jpeg(&taken.image)?));
        }
        let path = capture::save(
            &data::data_folder(&server),
            &taken,
            capture::mapping(&server),
        )?;
        Blob::from_file(path).map_err(ActionError::handled)
    }

    /// Captures the next frame whole, into memory, keeping at most
    /// `buffer_max` there. Returns its id.
    #[action]
    async fn capture_to_memory(
        &self,
        #[param(default = 1)] buffer_max: usize,
    ) -> Result<u64, ActionError> {
        let taken = self.take(CaptureMode::Standard).await?;
        Ok(self.captures.add(taken, buffer_max))
    }

    /// Saves a capture from memory to `path` in the data folder, as a JPEG
    /// or a PNG by its extension, and takes it out of memory: the latest,
    /// without `buffer_id`.
    #[action(blocking, global_lock = false)]
    fn save_from_memory(
        &self,
        server: Server,
        path: String,
        #[param(default)] buffer_id: Option<u64>,
    ) -> Result<(), ActionError> {
        let target = data::resolve(&data::data_folder(&server), &path);
        let target = target.map_err(ActionError::handled)?;
        let extension = target.extension().and_then(|e| e.to_str()).unwrap_or("");
        let format = match extension.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "png" => ImageFormat::Png,
            _ => return Err(ActionError::handled("save as .jpg, .jpeg or .png")),
        };
        let capture = (self.captures.take(buffer_id))
            .ok_or_else(|| ActionError::handled("there's no such capture in memory"))?;
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder).map_err(ActionError::handled)?;
        }
        (capture.image.save_with_format(&target, format)).map_err(ActionError::handled)
    }

    /// Empties memory of captures.
    #[action]
    async fn clear_buffers(&self) {
        self.captures.clear();
    }

    /// Sets the background detector's background from what the camera sees
    /// now, which should be the empty slide.
    #[action]
    async fn set_background(&self) -> Result<(), ActionError> {
        let (_, detector) = self.detector().ok_or_else(no_detector)?;
        detector.set_background(&self.small_frame().await?)
    }

    /// Whether what the camera sees now holds sample, as the background
    /// detector judges.
    #[action]
    async fn image_is_sample(&self) -> Result<SampleCheck, ActionError> {
        let (_, detector) = self.detector().ok_or_else(no_detector)?;
        SampleCheck::of(&*detector, &self.small_frame().await?)
    }

    /// Calibrates the camera, looking at the empty slide: its automatic
    /// exposure and white balance once, where it has them, then the
    /// background detector's background, if there is one.
    #[action]
    async fn full_auto_calibrate(&self, ctx: ActionCtx) -> Result<(), ActionError> {
        for feature in ["ExposureAuto", "BalanceWhiteAuto"] {
            let name = feature.to_owned();
            if !self.call(move |sdk| Ok(sdk.has(&name))).await? {
                continue;
            }
            self.call(move |sdk| sdk.set(feature, "Once")).await?;
            // The camera sets it back to Off once it's done; one that doesn't
            // settle is turned off, and the calibration goes on.
            let started = std::time::Instant::now();
            while self.call(move |sdk| sdk.get(feature)).await? == "Once" {
                if started.elapsed() > AUTO_WAIT {
                    tracing::warn!(
                        "the camera's {feature} didn't settle within {AUTO_WAIT:?}: it's off"
                    );
                    self.call(move |sdk| sdk.set(feature, "Off")).await?;
                    break;
                }
                ctx.sleep(Duration::from_millis(100)).await?;
            }
        }
        match self.detector() {
            Some(_) => self.set_background().await,
            None => Ok(()),
        }
    }

    /// Closes the camera and opens it again, and starts grabbing.
    #[action]
    async fn reconnect(&self) -> Result<(), ActionError> {
        self.device.reset().await.map_err(device_error)?;
        self.start_grabbing().await
    }

    /// The calibration actions for the calibration wizard and the settings
    /// panel (ADR-0024).
    #[endpoint(get, "primary_calibration_actions")]
    async fn primary_calibration_actions(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| {
            let calibrate = ui.action("full_auto_calibrate", "Full auto-calibrate");
            let done = Success::Message("The camera is calibrated.".to_owned());
            let stream = Some(ui.thing_name());
            vec![
                calibrate
                    .in_dialog(stream)
                    .on_success(done)
                    .refresh()
                    .into(),
            ]
        })
    }

    /// The calibration actions for the settings panel only (ADR-0024).
    #[endpoint(get, "secondary_calibration_actions")]
    async fn secondary_calibration_actions(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| {
            let question = "Is the camera looking at an empty part of the slide?";
            let set = ui.action("set_background", "Set background");
            vec![set.confirm(question).secondary().into()]
        })
    }

    /// The camera's settings, as its SDK reports them (ADR-0024, ADR-0043).
    #[endpoint(get, "manual_camera_settings")]
    async fn manual_camera_settings(&self) -> Json<Vec<UiElement>> {
        let features = self
            .call(|sdk| Ok(sdk.features()))
            .await
            .unwrap_or_default();
        let shown = features.into_iter().map(|mut feature| {
            if let (FeatureKind::Choice(choices), "PixelFormat") =
                (&mut feature.kind, feature.name.as_str())
            {
                choices.retain(|choice| PixelFormat::from_genicam(choice).is_some());
            }
            feature
        });
        let features: Vec<Feature> = shown.collect();
        self.entry.panel(|ui| settings_panel(ui, &features))
    }

    /// Starts grabbing, and the watchdog.
    #[on_start]
    async fn start_camera(&self, ctx: ThingCtx) -> anyhow::Result<()> {
        self.entry.keep::<Self>(&ctx);
        let started = async {
            self.convertible_format().await?;
            self.start_grabbing().await
        };
        started.await.map_err(|error| anyhow::anyhow!("{error}"))?;
        let Some(camera) = ctx.server().thing_ref::<SdkCamera>(ctx.name()) else {
            return Ok(());
        };
        let camera = Arc::clone(ThingRef::thing(&camera));
        let task = tokio::spawn(async move { camera.watch().await });
        *self.watchdog.lock().expect("not poisoned") = Some(task);
        Ok(())
    }

    /// Stops the watchdog; closing the device stops grabbing.
    #[on_stop]
    async fn stop_camera(&self) {
        if let Some(task) = self.watchdog.lock().expect("not poisoned").take() {
            task.abort();
        }
    }
}

impl SdkCamera {
    /// Runs `f` on the camera's SDK.
    async fn call<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut dyn CameraSdk) -> Result<R, String> + Send + 'static,
    ) -> Result<R, ActionError> {
        let call = move |driver: &mut SdkDriver, _: &CancelToken| {
            f(&mut *driver.0).map_err(anyhow::Error::msg)
        };
        self.device.try_call(call).await.map_err(device_error)
    }

    async fn read(&self, name: &'static str) -> Result<String, PropertyError> {
        let read = self.call(move |sdk| sdk.get(name)).await;
        read.map_err(|error| PropertyError::Failed(anyhow::anyhow!("{error}")))
    }

    async fn number(&self, name: &'static str) -> Result<f64, PropertyError> {
        let text = self.read(name).await?;
        (text.trim().parse())
            .map_err(|_| PropertyError::Failed(anyhow::anyhow!("{name} is {text}")))
    }

    async fn write(&self, name: &'static str, value: String) -> Result<(), PropertyError> {
        let write = self.call(move |sdk| sdk.set(name, &value)).await;
        write.map_err(|error| PropertyError::Failed(anyhow::anyhow!("{error}")))
    }

    /// Chooses a pixel format whose frames can be shown, if the camera's
    /// can't be.
    async fn convertible_format(&self) -> Result<(), ActionError> {
        let format = self.call(|sdk| sdk.get("PixelFormat")).await?;
        if PixelFormat::from_genicam(&format).is_some() {
            return Ok(());
        }
        let features = self.call(|sdk| Ok(sdk.features())).await?;
        let choices = features.into_iter().find_map(|feature| match feature.kind {
            FeatureKind::Choice(choices) if feature.name == "PixelFormat" => Some(choices),
            _ => None,
        });
        let choices = choices.unwrap_or_default();
        let Some(chosen) = choices
            .into_iter()
            .find(|choice| PixelFormat::from_genicam(choice).is_some())
        else {
            return Err(ActionError::handled(format!(
                "the camera's frames, in {format}, can't be shown"
            )));
        };
        tracing::info!("the camera sent {format}, which can't be shown: it sends {chosen} now");
        self.call(move |sdk| sdk.set("PixelFormat", &chosen)).await
    }

    async fn start_grabbing(&self) -> Result<(), ActionError> {
        let (pipeline, main, lores) = (
            Arc::clone(&self.pipeline),
            self.mjpeg_stream.clone(),
            self.lores_mjpeg_stream.clone(),
        );
        let warned = AtomicBool::new(false);
        let deliver: Deliver = Box::new(move |frame, taken| {
            if let Err(error) = pipeline.deliver(frame, taken, &main, &lores)
                && !warned.swap(true, Ordering::Relaxed)
            {
                tracing::warn!("the camera's frame couldn't be used: {error}");
            }
        });
        self.call(move |sdk| sdk.start(deliver)).await
    }

    /// Opens the camera again whenever its frames stop.
    async fn watch(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let last = self.pipeline.last_delivered();
            if last.is_some_and(|at| at.elapsed() < STALLED_AFTER) {
                continue;
            }
            tracing::warn!("the camera has sent no frame for {STALLED_AFTER:?}: opening it again");
            match self.reconnect().await {
                Ok(()) => tracing::info!("the camera is open again"),
                Err(error) => {
                    tracing::warn!("the camera couldn't be opened again: {error}");
                    tokio::time::sleep(STALLED_AFTER).await;
                }
            }
        }
    }

    /// A capture in `mode`, with where the stage is and the camera's
    /// settings.
    async fn take(&self, mode: CaptureMode) -> Result<Capture, ActionError> {
        let position = match self.stage.get() {
            Some(stage) => stage.position().await.ok(),
            None => None,
        };
        let axis_scale = self.stage.get().map(|stage| stage.axis_scale());
        let mut camera_settings = BTreeMap::new();
        for (name, property, _) in STANDARD_FEATURES {
            if let Ok(value) = self.call(move |sdk| sdk.get(name)).await {
                let value = value
                    .parse::<f64>()
                    .map_or_else(|_| json!(value), Value::from);
                camera_settings.insert(property.to_owned(), value);
            }
        }
        let metadata = CaptureMetadata {
            position,
            axis_scale,
            camera_settings,
            ..CaptureMetadata::default()
        };
        match mode {
            CaptureMode::Standard => self.pipeline.capture(metadata).await,
            CaptureMode::Quick => {
                let image = self.pipeline.grab_frame(&self.mjpeg_stream).await?;
                let (width, height) = image.dimensions();
                let acquired = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let metadata = CaptureMetadata {
                    acquired,
                    width,
                    height,
                    ..metadata
                };
                Ok(Capture { image, metadata })
            }
        }
    }

    /// The background detector in use, and its name.
    fn detector(&self) -> Option<(String, Arc<dyn BackgroundDetector>)> {
        let detectors = self.background_detectors.get()?;
        let named = self.detector_name.lock().expect("not poisoned").clone();
        let name = (named.filter(|name| detectors.contains_key(name)))
            .or_else(|| {
                (detectors.contains_key(DEFAULT_DETECTOR)).then(|| DEFAULT_DETECTOR.to_owned())
            })
            .or_else(|| detectors.keys().next().cloned())?;
        Some((name.clone(), Arc::clone(&detectors[&name])))
    }

    /// The next preview frame, shrunk to the low-resolution stream's size,
    /// for background detection.
    async fn small_frame(&self) -> Result<RgbImage, ActionError> {
        let frame = self.pipeline.grab_frame(&self.mjpeg_stream).await?;
        let (width, height) = LORES_SIZE;
        Ok(DynamicImage::ImageRgb8(frame)
            .thumbnail_exact(width, height)
            .into_rgb8())
    }
}

fn no_detector() -> ActionError {
    ActionError::handled("there's no background detector")
}

impl CameraApi for ThingRef<SdkCamera> {
    fn grab_frame(&self) -> BoxFuture<'_, Result<RgbImage, ActionError>> {
        let camera = ThingRef::thing(self);
        Box::pin(camera.pipeline.grab_frame(&camera.mjpeg_stream))
    }

    fn grab_jpeg(&self) -> BoxFuture<'_, Result<Vec<u8>, ActionError>> {
        let camera = ThingRef::thing(self);
        Box::pin(async move {
            let jpeg = camera.mjpeg_stream.grab_frame().await;
            Ok(jpeg.map_err(ActionError::handled)?.to_vec())
        })
    }

    fn frame_size(&self) -> (u32, u32) {
        ThingRef::thing(self)
            .pipeline
            .preview_size()
            .unwrap_or((0, 0))
    }

    fn capture_to_memory(&self, mode: CaptureMode) -> BoxFuture<'_, Result<Capture, ActionError>> {
        Box::pin(ThingRef::thing(self).take(mode))
    }

    fn stream_info(&self) -> StreamInfo {
        let (width, height) = self.frame_size();
        let affordance = "mjpeg_stream".to_owned();
        StreamInfo {
            affordance,
            width,
            height,
        }
    }

    fn lores_stream(&self) -> MjpegStream {
        ThingRef::thing(self).lores_mjpeg_stream.clone()
    }
}
