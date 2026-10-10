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
//! - **Settings** choose the objective, the sample (its density and colours,
//!   and whether it repeats), the noise, and the exposure and gain. They
//!   apply from the next frame. `remove_sample` and `load_sample` take the
//!   slide out and put it back.
//! - **Light:** while an `illumination` Thing's LED is off, frames are black.
//! - **Captures** are drawn at 1640 × 1232. `capture_to_memory` keeps them
//!   in memory, and `save_from_memory` saves them to the data folder later.
//! - **Settling:** after a move, `settle` waits `settling_time`, then
//!   `discard_frames` makes sure the next frame is drawn after it.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use image::codecs::jpeg::JpegEncoder;
use image::{ImageFormat, ImageResult, RgbImage};
use microscope_sim::{BlobConfig, BlobSpecimen, Extent, Objective, Sensor, SimState, render_frame};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use teta_wot::blob::Jpeg;
use teta_wot::prelude::*;

use crate::data;
use crate::hardware::{
    CameraApi, Capture, CaptureBuffer, CaptureMetadata, IlluminationApi, PreviewFrames, StageApi,
    StreamInfo,
};
use crate::stage::SimulatedStage;

/// How often the preview draws a frame, when drawing is fast enough.
const FRAME_INTERVAL: Duration = Duration::from_millis(100);

/// The exposure, in µs, at which the sample is drawn as bright as it is.
const REFERENCE_EXPOSURE_US: f64 = 10_000.0;

/// The repeating sample's period, in µm.
const REPEAT_PERIOD_UM: f64 = 2_000.0;

/// The colours of `BlobConfig::default()`.
const DEFAULT_COLOURS: &str = "#965aaa; #d278a0; #6e78be; #be96c8";

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
    /// The sensor's noise, as a standard deviation in grey levels.
    #[setting(default = 2.0, ge = 0.0, le = 50.0)]
    noise_level: Prop<f64>,
    /// How long each frame is exposed, in µs.
    #[setting(default = 10_000, ge = 100, le = 1_000_000)]
    exposure_time: Prop<u32>,
    /// The sensor's gain.
    #[setting(default = 1.0, ge = 1.0, le = 16.0)]
    analogue_gain: Prop<f64>,
    /// How long `settle` waits, in seconds.
    #[setting(default = 0.2, ge = 0.0, le = 60.0)]
    settling_time: Prop<f64>,
    frames: PreviewFrames,
    captures: CaptureBuffer,
    preview: Mutex<Option<PreviewThread>>,
    /// The number of frames drawn, which seeds each one's noise.
    drawn: AtomicU64,
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
        self.captures.add(self.capture(), buffer_max)
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
        let camera = Arc::clone(ThingRef::thing(&camera));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("camera preview".to_owned())
            .spawn(move || camera.run_preview(&stopped))
            .expect("the preview thread starts");
        *self.preview.lock().expect("not poisoned") = Some(PreviewThread { stop, thread });
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
        let defaults = BlobConfig::default();
        let specimen = BlobSpecimen::new(BlobConfig {
            density_per_mm2: if self.sample_loaded.get() {
                self.blob_density.get()
            } else {
                0.0
            },
            colours: parse_colours(&self.colour.get()),
            extent: if self.repeating.get() {
                Extent::Repeating {
                    period_um: REPEAT_PERIOD_UM,
                }
            } else {
                defaults.extent
            },
            ..defaults
        });
        let mut frame = render_frame(&SimState {
            specimen: Arc::new(specimen),
            objective: self.objective.get().objective(),
            sensor,
            stage_um: ThingRef::thing(&self.stage).true_position_um(),
            focus_z_um: 0.0,
            noise: self.noise_level.get() as f32,
            frame: number,
        });
        let brightness =
            f64::from(self.exposure_time.get()) / REFERENCE_EXPOSURE_US * self.analogue_gain.get();
        if brightness != 1.0 {
            for value in frame.iter_mut() {
                *value = (f64::from(*value) * brightness).round().min(255.0) as u8;
            }
        }
        frame
    }

    /// A capture: a frame at 1640 × 1232, and how it was taken. Drawing it
    /// takes a while, so it isn't for the async threads.
    fn capture(&self) -> Capture {
        let sensor = Sensor::CAPTURE;
        let magnification = self.objective.get().objective().magnification;
        let metadata = CaptureMetadata {
            acquired: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            width: sensor.width,
            height: sensor.height,
            position: Some(ThingRef::thing(&self.stage).position_now()),
            axis_scale: Some(StageApi::axis_scale(&*self.stage)),
            um_per_pixel: Some(sensor.pixel_pitch_um / magnification),
            camera_settings: self.settings(),
        };
        Capture {
            image: self.draw(sensor),
            metadata,
        }
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
            let capture = tokio::task::spawn_blocking(move || camera.capture()).await;
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
