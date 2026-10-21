//! The SDK camera pipeline (P06), with a camera Thing put together as the
//! SDK drivers' will be: frames from a fake SDK's callback reach both
//! streams and captures, and its settings panel comes from its features.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use microscope_core::pixels::PixelFormat;
use serde_json::{Value, json};
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;
use teta_wot::testing::Harness;

use microscope_things::fakes::FakeSdkCamera;
use microscope_things::hardware::{
    CaptureMetadata, Feature, FeatureKind, FramePipeline, LORES_SIZE, settings_panel,
};
use microscope_things::ui::{OwnEntry, UiElement};

/// A camera on a fake SDK, delivering 2048 × 1536 frames in 12 bits.
#[derive(Thing)]
struct SdkCamera {
    #[stream]
    mjpeg_stream: MjpegStream,
    #[stream]
    lores_mjpeg_stream: MjpegStream,
    /// The exposure, in µs.
    #[property(default = 1000.0)]
    exposure_time: Prop<f64>,
    /// The pixel format.
    #[property(default = "Mono12".to_owned())]
    pixel_format: Prop<String>,
    pipeline: Arc<FramePipeline>,
    sdk: Mutex<Option<FakeSdkCamera>>,
    entry: OwnEntry,
}

/// What the fake SDK reports: two of the standard features, and gain,
/// which this Thing lacks a property for.
fn features() -> Vec<Feature> {
    let feature = |name: &str, kind| Feature {
        name: name.to_owned(),
        kind,
    };
    let formats = ["Mono8", "Mono12"].map(String::from).to_vec();
    vec![
        feature("ExposureTime", FeatureKind::Number { step: Some(10.0) }),
        feature("Gain", FeatureKind::Number { step: None }),
        feature("PixelFormat", FeatureKind::Choice(formats)),
        feature("DeviceTemperature", FeatureKind::Number { step: None }),
    ]
}

#[thing_impl]
impl SdkCamera {
    /// Its settings, from its features.
    #[endpoint(get, "settings_ui")]
    async fn settings_ui(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| settings_panel(ui, &features()))
    }

    #[on_start]
    async fn start_acquiring(&self, ctx: ThingCtx) {
        self.entry.keep::<Self>(&ctx);
        let pipeline = Arc::clone(&self.pipeline);
        let (main, lores) = (self.mjpeg_stream.clone(), self.lores_mjpeg_stream.clone());
        let format = PixelFormat::Mono16 { bits: 12 };
        let interval = Duration::from_millis(20);
        let sdk = FakeSdkCamera::start((2048, 1536), format, interval, move |frame, taken| {
            let _ = pipeline.deliver(frame, taken, &main, &lores);
        });
        *self.sdk.lock().expect("not poisoned") = Some(sdk);
    }

    #[on_stop]
    async fn stop_acquiring(&self) {
        drop(self.sdk.lock().expect("not poisoned").take());
    }
}

async fn start() -> Harness<SdkCamera> {
    (Harness::builder("camera", SdkCamera::default())
        .start()
        .await)
        .expect("started")
}

#[tokio::test]
async fn frames_from_the_sdk_reach_both_streams_and_captures() {
    let harness = start().await;
    let camera = ThingRef::thing(harness.thing());
    let preview = camera.pipeline.grab_frame(&camera.mjpeg_stream).await;
    assert_eq!(preview.expect("a preview frame").dimensions(), (1024, 768));
    camera
        .lores_mjpeg_stream
        .next_frame()
        .await
        .expect("a small frame");
    let small = camera
        .lores_mjpeg_stream
        .latest_frame()
        .expect("a small frame");
    let small = image::load_from_memory(&small.data).expect("a JPEG");
    assert_eq!((small.width(), small.height()), LORES_SIZE);

    let metadata = CaptureMetadata {
        camera_settings: [("exposure_time".to_owned(), json!(1000.0))].into(),
        ..CaptureMetadata::default()
    };
    let capture = camera.pipeline.capture(metadata).await.expect("captured");
    assert_eq!(capture.image.dimensions(), (2048, 1536));
    assert_eq!(capture.metadata.camera_settings["exposure_time"], 1000.0);
    // A frame's first two bytes are its number, n: 12 bits of n·257, as 8.
    let level = capture.image.get_pixel(0, 0).0[0];
    let from = |n: u16| ((n * 257) >> 4).min(255) as u8;
    assert!((0..=255).any(|n| from(n) == level), "{level}");
    harness.stop().await;
}

#[tokio::test]
async fn the_settings_panel_comes_from_the_features() {
    let harness = start().await;
    let panel = harness.client().get("/camera/settings_ui").await.json();
    let controls = panel[0]["children"].as_array().expect("controls").clone();
    let properties: Vec<&Value> = controls.iter().map(|c| &c["property"]).collect();
    assert_eq!(
        properties,
        [
            &json!("exposure_time"),
            &json!("gain"),
            &json!("pixel_format")
        ]
    );
    assert_eq!(controls[0]["step"], 10.0);
    assert_eq!(controls[0]["read_back_ms"], 300);
    // Gain is reported, but this Thing has no property for it.
    assert_eq!(controls[1]["broken"], true);
    assert_eq!(controls[2]["options"][1]["value"], "Mono12");
    harness.stop().await;
}
