//! The `camera` Thing in simulation: `SimulatedCamera`, mirroring
//! OpenFlexure's camera, which shows the sample where the simulated stage
//! really is (ADR-0020).
//!
//! - **Preview:** a thread, started in `on_start` and stopped in `on_stop`,
//!   draws a frame every 100 ms, or as often as drawing allows, and
//!   publishes it to `mjpeg_stream` and, at 320 × 240, `lores_mjpeg_stream`.
//! - **Streaming modes:** `default` streams 820 × 616 frames;
//!   `full_resolution` streams 1640 × 1232, the size of a capture, more
//!   slowly.
//! - **Settings** choose the objective, the sample, the noise, and the
//!   exposure and gain, and the optics' imperfections (P25). They apply from
//!   the next frame. `remove_sample` and `load_sample` take the slide out and
//!   put it back.
//! - **Samples** (P25): coloured blobs (their density and colours, and
//!   whether they repeat), H&E-like tissue (its coverage), or a picture from
//!   the data folder (its scale).
//! - **Optics** (P25): a focal surface that tilts and waves, vignetting, and
//!   uneven lighting, all off until set.
//! - **`manual_camera_settings`**, a custom endpoint, describes these
//!   settings for the web app's settings panel (ADR-0024).
//! - **Background detection** (P27): the camera uses one of the server's
//!   background detectors, named by `background_detector_name`.
//!   `set_background` and `image_is_sample` give it a frame shrunk to the
//!   low-resolution stream's size; `full_auto_calibrate` sets the
//!   background from the empty slide. `calibration_required` says until
//!   then, and two panels hold the calibration actions.
//! - **Light:** while an `illumination` Thing's LED is off, frames are black.
//! - **Captures** are drawn at 1640 × 1232, or at the stream's size.
//!   `capture` saves one to the data folder's `captures`, with a sidecar of
//!   how it was taken, and returns it to download (P26, ADR-0025).
//!   `capture_to_memory` keeps them in memory, and `save_from_memory` saves
//!   them to the data folder later.
//! - **Settling:** after a move, `settle` waits `settling_time`, then
//!   `discard_frames` makes sure the next frame is drawn after it.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, ImageResult, RgbImage};
use microscope_sim::{
    BlobConfig, BlobSpecimen, Effects, Extent, FocalSurface, ImageSpecimen, Objective, Sensor,
    SimState, Specimen, TissueConfig, TissueSpecimen, render_frame,
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use teta_wot::blob::Jpeg;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

use crate::capture;
use crate::data;
use crate::hardware::{
    BackgroundDetector, CameraApi, Capture, CaptureBuffer, CaptureMetadata, IlluminationApi,
    LORES_SIZE, PreviewFrames, StageApi, StreamInfo,
};
use crate::stage::SimulatedStage;
use crate::ui::{Layout, OwnEntry, Success, ThingUi, UiElement};

/// How often the preview draws a frame, when drawing is fast enough.
const FRAME_INTERVAL: Duration = Duration::from_millis(100);

/// The exposure, in µs, at which the sample is drawn as bright as it is.
const REFERENCE_EXPOSURE_US: f64 = 10_000.0;

/// The repeating sample's period, in µm.
const REPEAT_PERIOD_UM: f64 = 2_000.0;

/// The background detector the camera uses unless told otherwise, if the
/// server has it.
const DEFAULT_DETECTOR: &str = "bg_channel_deviations_luv";

/// The colours of `BlobConfig::default()`.
const DEFAULT_COLOURS: &str = "#965aaa; #d278a0; #6e78be; #be96c8";

/// How the simulated camera is turned relative to the stage, which makes
/// camera–stage mapping more than a scale (P23).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Orientation {
    /// Turned by this many degrees.
    pub rotation_deg: f64,
    /// Mirrored left to right.
    pub flipped: bool,
}

impl Orientation {
    /// Where a point of the stage (x and y, in µm) is in the camera's view:
    /// mirrored if flipped, then turned.
    pub fn apply(self, [x, y]: [f64; 2]) -> [f64; 2] {
        let x = if self.flipped { -x } else { x };
        let (sin, cos) = self.rotation_deg.to_radians().sin_cos();
        [cos * x - sin * y, sin * x + cos * y]
    }
}

/// Whether an image holds sample, as the background detector judges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SampleCheck {
    /// Whether it holds sample.
    pub sample: bool,
    /// The percentage of it that does.
    pub coverage: f64,
    /// Both, in words: "62.5% sample", or "only 3.1% sample".
    pub message: String,
}

/// A picture loaded for the sample: from which file, at which scale, and
/// the picture, unless it couldn't be loaded.
struct LoadedPicture {
    path: PathBuf,
    scale: f64,
    picture: Option<Arc<ImageSpecimen>>,
}

/// What's on the slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SampleType {
    /// Coloured blobs.
    #[default]
    Blobs,
    /// H&E-like tissue.
    Tissue,
    /// A picture from the data folder.
    Image,
}

/// How a capture is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    /// At full resolution: 1640 × 1232.
    #[default]
    Standard,
    /// At the stream's size, without changing the stream.
    Quick,
}

/// A camera's streaming mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StreamingMode {
    /// 820 × 616 frames, quick to draw.
    #[default]
    Default,
    /// 1640 × 1232 frames, the size of a capture.
    FullResolution,
}

impl StreamingMode {
    const ALL: [StreamingMode; 2] = [StreamingMode::Default, StreamingMode::FullResolution];

    /// The sensor its frames are drawn for.
    pub fn sensor(self) -> Sensor {
        match self {
            StreamingMode::Default => Sensor::PREVIEW,
            StreamingMode::FullResolution => Sensor::CAPTURE,
        }
    }
}

/// A streaming mode, as `streaming_modes` lists it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StreamingModeInfo {
    /// What the mode is for.
    pub description: String,
    /// The frames' width, in pixels.
    pub width: u32,
    /// The frames' height, in pixels.
    pub height: u32,
}

/// One of the camera's two streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StreamName {
    /// `mjpeg_stream`.
    #[default]
    Main,
    /// `lores_mjpeg_stream`.
    Lores,
}

/// An objective's magnification: 4, 10, 20, 40, 60 or 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Magnification(u32);

impl Magnification {
    /// The objective with this magnification.
    pub fn objective(self) -> Objective {
        Objective::with_magnification(f64::from(self.0)).expect("checked when made")
    }
}

impl TryFrom<u32> for Magnification {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, String> {
        match Objective::with_magnification(f64::from(value)) {
            Some(_) => Ok(Self(value)),
            None => Err("the objective must be 4, 10, 20, 40, 60 or 100".to_owned()),
        }
    }
}

impl From<Magnification> for u32 {
    fn from(magnification: Magnification) -> u32 {
        magnification.0
    }
}

impl JsonSchema for Magnification {
    fn schema_name() -> Cow<'static, str> {
        "Magnification".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type": "integer", "enum": [4, 10, 20, 40, 60, 100]})
    }
}

/// The preview thread, while it runs.
#[derive(Debug)]
struct PreviewThread {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// A simulated camera, mirroring OpenFlexure's camera.
#[derive(Thing)]
#[thing(interfaces(CameraApi))]
pub struct SimulatedCamera {
    /// The stage whose true position the frames show.
    #[slot(default = "stage")]
    stage: Slot<SimulatedStage>,
    /// The light, if there is one.
    #[slot]
    illumination: OptSlot<dyn IlluminationApi>,
    /// The background detectors the camera can use, by name.
    #[slot]
    background_detectors: SlotMap<dyn BackgroundDetector>,
    /// The live preview.
    #[stream]
    mjpeg_stream: MjpegStream,
    /// The live preview, at 320 × 240.
    #[stream]
    lores_mjpeg_stream: MjpegStream,
    /// The mode the camera streams in.
    #[property(default = StreamingMode::Default, readonly)]
    streaming_mode: Prop<StreamingMode>,
    /// Whether the sample is in place.
    #[property(default = true, readonly)]
    sample_loaded: Prop<bool>,
    /// The objective's magnification.
    #[setting(default = Magnification(40))]
    objective: Prop<Magnification>,
    /// How many blobs the sample has, per mm².
    #[setting(default = 1500.0, ge = 0.0, le = 100_000.0)]
    blob_density: Prop<f64>,
    /// The blobs' colours: HTML hex colours, separated by semicolons.
    #[setting(
        default = DEFAULT_COLOURS.to_owned(),
        pattern = r"^\s*#[0-9a-fA-F]{6}(\s*;\s*#[0-9a-fA-F]{6})*\s*;?\s*$"
    )]
    colour: Prop<String>,
    /// Whether the sample repeats every 2 mm, rather than ending at 20 mm
    /// across.
    #[setting(default = false)]
    repeating: Prop<bool>,
    /// What's on the slide: blobs, tissue or a picture.
    #[setting(default = SampleType::Blobs)]
    sample_type: Prop<SampleType>,
    /// The share of the slide the tissue covers.
    #[setting(default = 0.7, ge = 0.0, le = 1.0)]
    tissue_coverage: Prop<f64>,
    /// The picture on the slide: a PNG, JPEG or TIFF file in the data folder.
    #[setting(default = String::new())]
    sample_image: Prop<String>,
    /// The size of the picture's pixels on the slide, in µm.
    #[setting(default = 0.25, ge = 0.001, le = 100.0)]
    sample_image_scale: Prop<f64>,
    /// How much the focal surface rises in x, in µm per mm.
    #[setting(default = 0.0, ge = -100.0, le = 100.0)]
    focal_tilt_x: Prop<f64>,
    /// How much the focal surface rises in y, in µm per mm.
    #[setting(default = 0.0, ge = -100.0, le = 100.0)]
    focal_tilt_y: Prop<f64>,
    /// How far the focal surface waves up and down, in µm.
    #[setting(default = 0.0, ge = 0.0, le = 50.0)]
    focal_waviness: Prop<f64>,
    /// The share of the light lost in the frame's corners.
    #[setting(default = 0.0, ge = 0.0, le = 0.9)]
    vignetting: Prop<f64>,
    /// The share of the light lost at the frame's most dimly lit corner.
    #[setting(default = 0.0, ge = 0.0, le = 0.9)]
    uneven_illumination: Prop<f64>,
    /// The sensor's noise, as a standard deviation in grey levels.
    #[setting(default = 2.0, ge = 0.0, le = 50.0)]
    noise_level: Prop<f64>,
    /// How long each frame is exposed, in µs.
    #[setting(default = 10_000, ge = 100, le = 1_000_000)]
    exposure_time: Prop<u32>,
    /// The sensor's gain.
    #[setting(default = 1.0, ge = 1.0, le = 16.0)]
    analogue_gain: Prop<f64>,
    /// How the camera is turned relative to the stage.
    #[setting(default = Orientation::default())]
    orientation: Prop<Orientation>,
    /// How long `settle` waits, in seconds.
    #[setting(default = 0.2, ge = 0.0, le = 60.0)]
    settling_time: Prop<f64>,
    frames: PreviewFrames,
    captures: CaptureBuffer,
    preview: Mutex<Option<PreviewThread>>,
    /// The number of frames drawn, which seeds each one's noise.
    drawn: AtomicU64,
    /// The data folder, where the picture is.
    data_folder: OnceLock<PathBuf>,
    /// The picture last loaded.
    picture: Mutex<Option<LoadedPicture>>,
    /// The camera's registry entry, which its panels check names against.
    entry: OwnEntry,
    /// The background detector named in the setting, if one is.
    detector_name: Mutex<Option<String>>,
}

#[thing_impl]
impl SimulatedCamera {
    /// The modes the camera can stream in.
    #[property]
    async fn streaming_modes(&self) -> BTreeMap<StreamingMode, StreamingModeInfo> {
        StreamingMode::ALL
            .into_iter()
            .map(|mode| {
                let Sensor { width, height, .. } = mode.sensor();
                let description = match mode {
                    StreamingMode::Default => "The standard preview, quick to draw.",
                    StreamingMode::FullResolution => {
                        "Frames the size of a capture, drawn more slowly."
                    }
                };
                let description = description.to_owned();
                (
                    mode,
                    StreamingModeInfo {
                        description,
                        width,
                        height,
                    },
                )
            })
            .collect()
    }

    /// Whether the preview is running.
    #[property]
    async fn stream_active(&self) -> bool {
        let preview = self.preview.lock().expect("not poisoned");
        preview
            .as_ref()
            .is_some_and(|preview| !preview.thread.is_finished())
    }

    /// Changes the mode the camera streams in, from the next frame.
    #[action]
    async fn change_streaming_mode(
        &self,
        #[param(default)] mode: StreamingMode,
    ) -> Result<(), ActionError> {
        if !self.stream_active().await {
            return Err(ActionError::handled("the camera isn't streaming"));
        }
        self.streaming_mode.set(mode).map_err(ActionError::handled)
    }

    /// The next frame of a stream, as a JPEG.
    #[action]
    async fn grab_jpeg(
        &self,
        #[param(default)] stream_name: StreamName,
    ) -> Result<Blob<Jpeg>, ActionError> {
        let jpeg = self.stream(stream_name).grab_frame().await;
        Ok(Blob::from_bytes(jpeg.map_err(ActionError::handled)?))
    }

    /// The size of a stream's next frame, in bytes: a JPEG's size grows with
    /// its detail, so it is a cheap measure of sharpness.
    #[action]
    async fn grab_jpeg_size(
        &self,
        #[param(default)] stream_name: StreamName,
    ) -> Result<usize, ActionError> {
        let size = self.stream(stream_name).next_frame_size().await;
        size.map_err(ActionError::handled)
    }

    /// Takes the sample out, leaving the empty slide.
    #[action]
    async fn remove_sample(&self) -> Result<(), ActionError> {
        if !self.sample_loaded.get() {
            return Err(ActionError::handled("the sample is already removed"));
        }
        self.sample_loaded.set(false).map_err(ActionError::handled)
    }

    /// Puts the sample back.
    #[action]
    async fn load_sample(&self) -> Result<(), ActionError> {
        if self.sample_loaded.get() {
            return Err(ActionError::handled("the sample is already in place"));
        }
        self.sample_loaded.set(true).map_err(ActionError::handled)
    }

    /// Captures an image at 1640 × 1232 into memory, returning its id, for
    /// `save_from_memory` to save. Memory keeps at most `buffer_max`
    /// captures: the oldest go first.
    #[action(blocking)]
    fn capture_to_memory(&self, #[param(default = 1)] buffer_max: usize) -> u64 {
        self.captures.add(self.take(Sensor::CAPTURE), buffer_max)
    }

    /// Captures an image: at full resolution (`standard`), or at the
    /// stream's size (`quick`). With `retain_image`, it's saved to the data
    /// folder's `captures`, with a sidecar of how it was taken (ADR-0025).
    /// Either way, it's returned, to download.
    #[action(blocking)]
    fn capture(
        &self,
        server: Server,
        #[param(default)] capture_mode: CaptureMode,
        #[param(default = true)] retain_image: bool,
    ) -> Result<Blob<Jpeg>, ActionError> {
        let sensor = match capture_mode {
            CaptureMode::Standard => Sensor::CAPTURE,
            CaptureMode::Quick => self.streaming_mode.get().sensor(),
        };
        let taken = self.take(sensor);
        if !retain_image {
            return Ok(Blob::from_bytes(capture::jpeg(&taken.image)?));
        }
        let folder = data::data_folder(&server);
        let path = capture::save(&folder, &taken, capture::mapping(&server))?;
        Blob::from_file(path).map_err(ActionError::handled)
    }

    /// Saves a capture from memory to `path` in the data folder, as a JPEG
    /// or a PNG by its extension, and takes it out of memory. Without
    /// `buffer_id`, it saves the latest capture and empties memory.
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
        let capture = self.captures.take(buffer_id).ok_or_else(|| {
            ActionError::handled(match buffer_id {
                Some(id) => format!("there's no capture {id} in memory"),
                None => "there's no capture in memory".to_owned(),
            })
        })?;
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder).map_err(ActionError::handled)?;
        }
        save_image(&capture.image, &target, format).map_err(ActionError::handled)
    }

    /// Empties memory of captures.
    #[action]
    async fn clear_buffers(&self) {
        self.captures.clear();
    }

    /// Waits `settling_time` for the sample to settle after a move, then
    /// discards frames, so the next frame shows it settled.
    #[action]
    async fn settle(&self, ctx: ActionCtx) -> Result<(), ActionError> {
        ctx.sleep(Duration::from_secs_f64(self.settling_time.get()))
            .await?;
        self.discard_frames().await
    }

    /// Discards frames: waits for any frame being drawn, so that the next
    /// frame grabbed was drawn after the call.
    #[action]
    async fn discard_frames(&self) -> Result<(), ActionError> {
        let drawn = self.mjpeg_stream.next_frame().await;
        drawn.map(|_| ()).map_err(ActionError::handled)
    }

    /// Starts the preview.
    #[on_start]
    async fn start_preview(&self, ctx: ThingCtx) {
        let Some(camera) = ctx.server().thing_ref::<SimulatedCamera>(ctx.name()) else {
            return;
        };
        self.entry.keep::<Self>(&ctx);
        let _ = self.data_folder.set(data::data_folder(ctx.server()));
        let camera = Arc::clone(ThingRef::thing(&camera));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("camera preview".to_owned())
            .spawn(move || camera.run_preview(&stopped))
            .expect("the preview thread starts");
        *self.preview.lock().expect("not poisoned") = Some(PreviewThread { stop, thread });
    }

    /// Which background detector the camera uses, by its Thing's name:
    /// `bg_channel_deviations_luv` unless set, if the server has it, or
    /// else the first.
    #[setting]
    async fn background_detector_name(&self) -> Option<String> {
        self.detector().map(|(name, _)| name)
    }

    #[setter(background_detector_name)]
    async fn set_background_detector_name(
        &self,
        name: Option<String>,
    ) -> Result<(), PropertyError> {
        let detectors = self.background_detectors.get();
        if let Some(unknown) = name
            .as_ref()
            .filter(|n| !detectors.is_some_and(|d| d.contains_key(*n)))
        {
            let message = format!("no background detector is called {unknown}");
            let error =
                teta_wot::ValidationError::value_error(Vec::new(), &message, json!(unknown));
            return Err(PropertyError::Invalid(error));
        }
        *self.detector_name.lock().expect("not poisoned") = name;
        Ok(())
    }

    /// Whether the camera needs calibrating: until the background detector
    /// in use has its background.
    #[property]
    async fn calibration_required(&self) -> bool {
        self.detector()
            .is_none_or(|(_, detector)| !detector.ready())
    }

    /// Sets the background detector's background from a frame of what the
    /// camera sees now, which should be the empty slide.
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
        let coverage = detector.sample_coverage(&self.small_frame().await?)?;
        let sample = coverage > detector.min_sample_coverage();
        let only = if sample { "" } else { "only " };
        Ok(SampleCheck {
            sample,
            coverage,
            message: format!("{only}{coverage:.1}% sample"),
        })
    }

    /// Calibrates the camera. In the simulator: takes the sample out, sets
    /// the background from the empty slide, and puts the sample back, even
    /// if setting the background failed.
    #[action]
    async fn full_auto_calibrate(&self) -> Result<(), ActionError> {
        self.sample_loaded
            .set(false)
            .map_err(ActionError::handled)?;
        let background = self.set_background().await;
        self.sample_loaded.set(true).map_err(ActionError::handled)?;
        background
    }

    /// The calibration actions for the calibration wizard and the settings
    /// panel (ADR-0024).
    #[endpoint(get, "primary_calibration_actions")]
    async fn primary_calibration_actions(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| {
            let calibrate = ui.action("full_auto_calibrate", "Full auto-calibrate");
            let done = Success::Message("The camera is calibrated.".to_owned());
            vec![
                calibrate
                    .in_dialog(Some(ui_camera(ui)))
                    .on_success(done)
                    .into(),
            ]
        })
    }

    /// The calibration actions for the settings panel only (ADR-0024).
    #[endpoint(get, "secondary_calibration_actions")]
    async fn secondary_calibration_actions(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| {
            let question = "Is the camera looking at an empty part of the slide?";
            vec![
                ui.action("set_background", "Set background")
                    .confirm(question)
                    .secondary()
                    .into(),
                ui.action("remove_sample", "Remove sample")
                    .secondary()
                    .into(),
                ui.action("load_sample", "Load sample").secondary().into(),
            ]
        })
    }

    /// The camera's settings, described for the settings panel (ADR-0024).
    #[endpoint(get, "manual_camera_settings")]
    async fn manual_camera_settings(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(panel)
    }

    /// Stops the preview, and ends both streams.
    #[on_stop]
    async fn stop_preview(&self) {
        let preview = self.preview.lock().expect("not poisoned").take();
        if let Some(PreviewThread { stop, thread }) = preview {
            stop.store(true, Ordering::Relaxed);
            thread.thread().unpark();
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
        self.mjpeg_stream.stop();
        self.lores_mjpeg_stream.stop();
    }
}

impl SimulatedCamera {
    /// The main stream: frames as JPEGs, as a browser's `<img>` shows them.
    pub fn mjpeg_stream(&self) -> &MjpegStream {
        &self.mjpeg_stream
    }

    fn stream(&self, name: StreamName) -> &MjpegStream {
        match name {
            StreamName::Main => &self.mjpeg_stream,
            StreamName::Lores => &self.lores_mjpeg_stream,
        }
    }

    /// The preview thread: draws and publishes frames until told to stop.
    fn run_preview(&self, stop: &AtomicBool) {
        while !stop.load(Ordering::Relaxed) {
            let started = Instant::now();
            // Stamped when the stage was read for it, not when it was ready.
            let taken = chrono::Utc::now();
            let frame = self.draw(self.streaming_mode.get().sensor());
            let published =
                self.frames
                    .publish(frame, taken, &self.mjpeg_stream, &self.lores_mjpeg_stream);
            if published.is_err() {
                break;
            }
            std::thread::park_timeout(FRAME_INTERVAL.saturating_sub(started.elapsed()));
        }
    }

    /// A frame for `sensor`: the sample where the stage really is, as the
    /// settings say, or black while the LED is off.
    fn draw(&self, sensor: Sensor) -> RgbImage {
        let number = self.drawn.fetch_add(1, Ordering::Relaxed);
        if self.illumination.get().is_some_and(|light| !light.led_on()) {
            return RgbImage::new(sensor.width, sensor.height);
        }
        render_frame(&SimState {
            specimen: self.specimen(),
            objective: self.objective.get().objective(),
            sensor,
            stage_um: {
                let [x, y, z] = ThingRef::thing(&self.stage).true_position_um();
                let [x, y] = self.orientation.get().apply([x, y]);
                [x, y, z]
            },
            focus_z_um: 0.0,
            noise: self.noise_level.get() as f32,
            frame: number,
            effects: Effects {
                focal_surface: FocalSurface {
                    tilt_um_per_mm: [self.focal_tilt_x.get(), self.focal_tilt_y.get()],
                    waviness_um: self.focal_waviness.get(),
                },
                vignetting: self.vignetting.get(),
                uneven_illumination: self.uneven_illumination.get(),
                exposure: f64::from(self.exposure_time.get()) / REFERENCE_EXPOSURE_US,
                gain: self.analogue_gain.get(),
            },
        })
    }

    /// The background detector in use, and its name: the one named, or
    /// `bg_channel_deviations_luv`, or the first.
    fn detector(&self) -> Option<(String, Arc<dyn BackgroundDetector>)> {
        let detectors = self.background_detectors.get()?;
        let named = self.detector_name.lock().expect("not poisoned").clone();
        let name = named
            .filter(|name| detectors.contains_key(name))
            .or_else(|| {
                detectors
                    .contains_key(DEFAULT_DETECTOR)
                    .then(|| DEFAULT_DETECTOR.to_owned())
            })
            .or_else(|| detectors.keys().next().cloned())?;
        let detector = Arc::clone(&detectors[&name]);
        Some((name, detector))
    }

    /// A frame drawn wholly after the call, shrunk to the low-resolution
    /// stream's size, for background detection.
    async fn small_frame(&self) -> Result<RgbImage, ActionError> {
        self.frames
            .next(&self.mjpeg_stream)
            .await
            .map_err(ActionError::handled)?;
        let frame = self.frames.next(&self.mjpeg_stream).await;
        let frame = RgbImage::clone(&*frame.map_err(ActionError::handled)?);
        let (width, height) = LORES_SIZE;
        Ok(DynamicImage::ImageRgb8(frame)
            .thumbnail_exact(width, height)
            .into_rgb8())
    }

    /// What's on the slide, as the settings say: the empty slide while the
    /// sample is out, or if the picture can't be loaded.
    fn specimen(&self) -> Arc<dyn Specimen> {
        let blobs = BlobConfig::default();
        let loaded = self.sample_loaded.get();
        match self.sample_type.get() {
            SampleType::Tissue if loaded => Arc::new(TissueSpecimen::new(TissueConfig {
                coverage: self.tissue_coverage.get(),
                ..TissueConfig::default()
            })),
            SampleType::Image if loaded => match self.picture() {
                Some(picture) => picture,
                None => Arc::new(BlobSpecimen::new(BlobConfig {
                    density_per_mm2: 0.0,
                    ..blobs
                })),
            },
            _ => Arc::new(BlobSpecimen::new(BlobConfig {
                density_per_mm2: if loaded { self.blob_density.get() } else { 0.0 },
                colours: parse_colours(&self.colour.get()),
                extent: if self.repeating.get() {
                    Extent::Repeating {
                        period_um: REPEAT_PERIOD_UM,
                    }
                } else {
                    blobs.extent
                },
                ..blobs
            })),
        }
    }

    /// The picture the settings name, loaded once for each file and scale.
    fn picture(&self) -> Option<Arc<dyn Specimen>> {
        let folder = self.data_folder.get()?;
        let path = data::resolve(folder, &self.sample_image.get()).ok()?;
        let scale = self.sample_image_scale.get();
        let mut cached = self.picture.lock().expect("not poisoned");
        if !cached
            .as_ref()
            .is_some_and(|c| c.path == path && c.scale == scale)
        {
            let picture = ImageSpecimen::open(&path, scale);
            if let Err(error) = &picture {
                tracing::warn!(path = %path.display(), %error, "the sample's picture can't be loaded");
            }
            let picture = picture.ok().map(Arc::new);
            *cached = Some(LoadedPicture {
                path,
                scale,
                picture,
            });
        }
        let picture: Arc<dyn Specimen> = cached.as_ref()?.picture.clone()?;
        Some(picture)
    }

    /// A capture: a frame for `sensor`, and how it was taken. Drawing it
    /// takes a while, so it isn't for the async threads.
    fn take(&self, sensor: Sensor) -> Capture {
        let magnification = self.objective.get().objective().magnification;
        let metadata = CaptureMetadata {
            acquired: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            width: sensor.width,
            height: sensor.height,
            position: Some(ThingRef::thing(&self.stage).position_now()),
            axis_scale: Some(StageApi::axis_scale(&*self.stage)),
            um_per_pixel: Some(sensor.pixel_pitch_um / magnification),
            camera_settings: self.settings(),
            simulation: Some(self.simulation()),
        };
        Capture {
            image: self.draw(sensor),
            metadata,
        }
    }

    /// What decides a simulated frame, for a capture's sidecar.
    fn simulation(&self) -> Value {
        json!({
            "sample_type": self.sample_type.get(),
            "sample_loaded": self.sample_loaded.get(),
            "blob_density": self.blob_density.get(),
            "colour": self.colour.get(),
            "repeating": self.repeating.get(),
            "tissue_coverage": self.tissue_coverage.get(),
            "sample_image": self.sample_image.get(),
            "sample_image_scale": self.sample_image_scale.get(),
            "focal_tilt": [self.focal_tilt_x.get(), self.focal_tilt_y.get()],
            "focal_waviness": self.focal_waviness.get(),
            "vignetting": self.vignetting.get(),
            "uneven_illumination": self.uneven_illumination.get(),
            "orientation": self.orientation.get(),
            "true_position_um": ThingRef::thing(&self.stage).true_position_um(),
        })
    }

    /// What a capture records about the camera.
    fn settings(&self) -> BTreeMap<String, Value> {
        BTreeMap::from([
            ("objective".to_owned(), json!(self.objective.get())),
            ("exposure_time".to_owned(), json!(self.exposure_time.get())),
            ("analogue_gain".to_owned(), json!(self.analogue_gain.get())),
            ("noise_level".to_owned(), json!(self.noise_level.get())),
        ])
    }
}

fn no_detector() -> ActionError {
    ActionError::handled("there's no background detector")
}

/// The name of the camera whose panel `ui` builds.
fn ui_camera<'a>(ui: &'a ThingUi) -> &'a str {
    ui.thing_name()
}

/// The camera's settings panel: the sample, the camera's settings, and the
/// optics.
fn panel(ui: &ThingUi) -> Vec<UiElement> {
    let row = |children: Vec<UiElement>| UiElement::Container {
        layout: Layout::Row,
        children,
    };
    let objectives = [4, 10, 20, 40, 60, 100].map(|m| (format!("{m}×"), m));
    vec![
        ui.property("sample_type", "Sample")
            .options([
                ("Blobs", "blobs"),
                ("Tissue", "tissue"),
                ("Picture", "image"),
            ])
            .into(),
        row(vec![
            ui.property("blob_density", "Blobs per mm²").into(),
            ui.property("colour", "Blob colours").into(),
            ui.property("repeating", "Repeating blobs").into(),
        ]),
        ui.property("tissue_coverage", "Tissue coverage")
            .step(0.05)
            .into(),
        row(vec![
            ui.property("sample_image", "Picture, in the data folder")
                .into(),
            ui.property("sample_image_scale", "Picture's scale (µm per pixel)")
                .into(),
        ]),
        ui.property("objective", "Objective")
            .options(objectives.iter().map(|(label, m)| (label.as_str(), *m)))
            .into(),
        row(vec![
            ui.property("exposure_time", "Exposure (µs)")
                .step(100.0)
                .into(),
            ui.property("analogue_gain", "Gain").into(),
            ui.property("noise_level", "Noise").into(),
        ]),
        UiElement::Accordion {
            title: "Optics".to_owned(),
            children: vec![
                row(vec![
                    ui.property("focal_tilt_x", "Focal tilt in x (µm per mm)")
                        .into(),
                    ui.property("focal_tilt_y", "Focal tilt in y (µm per mm)")
                        .into(),
                ]),
                ui.property("focal_waviness", "Focal waviness (µm)").into(),
                row(vec![
                    ui.property("vignetting", "Vignetting").step(0.05).into(),
                    ui.property("uneven_illumination", "Uneven lighting")
                        .step(0.05)
                        .into(),
                ]),
            ],
        },
    ]
}

/// Saves `image` to `path` as `format`: a JPEG at quality 95, or a PNG.
fn save_image(image: &RgbImage, path: &Path, format: ImageFormat) -> ImageResult<()> {
    if format == ImageFormat::Jpeg {
        let file = BufWriter::new(File::create(path)?);
        JpegEncoder::new_with_quality(file, 95).encode_image(image)
    } else {
        image.save_with_format(path, format)
    }
}

/// The colours in `text`, which the `colour` setting's pattern has checked.
fn parse_colours(text: &str) -> Vec<[u8; 3]> {
    let colours: Vec<[u8; 3]> = text
        .split(';')
        .filter_map(|colour| {
            let hex = colour.trim().strip_prefix('#')?;
            let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
            Some([channel(0)?, channel(2)?, channel(4)?])
        })
        .collect();
    if colours.is_empty() {
        BlobConfig::default().colours
    } else {
        colours
    }
}

impl CameraApi for ThingRef<SimulatedCamera> {
    fn grab_frame(&self) -> BoxFuture<'_, Result<RgbImage, ActionError>> {
        Box::pin(async move {
            let camera = ThingRef::thing(self);
            let frame = camera.frames.next(&camera.mjpeg_stream).await;
            let frame = frame.map_err(ActionError::handled)?;
            Ok(RgbImage::clone(&frame))
        })
    }

    fn grab_jpeg(&self) -> BoxFuture<'_, Result<Vec<u8>, ActionError>> {
        Box::pin(async move {
            let jpeg = ThingRef::thing(self).mjpeg_stream.grab_frame().await;
            Ok(jpeg.map_err(ActionError::handled)?.to_vec())
        })
    }

    fn frame_size(&self) -> (u32, u32) {
        let sensor = ThingRef::thing(self).streaming_mode.get().sensor();
        (sensor.width, sensor.height)
    }

    fn capture_to_memory(&self) -> BoxFuture<'_, Result<Capture, ActionError>> {
        let camera = Arc::clone(ThingRef::thing(self));
        Box::pin(async move {
            let capture = tokio::task::spawn_blocking(move || camera.take(Sensor::CAPTURE)).await;
            capture.map_err(ActionError::handled)
        })
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
        ThingRef::thing(self).lores_mjpeg_stream.clone()
    }
}
