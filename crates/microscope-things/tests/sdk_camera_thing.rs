//! `SdkCamera` (P07): against a fake SDK, its streams, captures, settings,
//! calibration, a pixel format it can't show and a camera unplugged; and on
//! a Basler camera, where `MICROSCOPE_BASLER_SERIAL` names one (pylon's
//! camera emulator, `PYLON_CAMEMU=1`, is `0815-0000`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use teta_wot::FromConfig;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::background::ChannelDeviationDetector;
use microscope_things::drivers::hardware_device;
use microscope_things::fakes::FakeCameraSdk;
use microscope_things::hardware::{CameraApi, CaptureMode};
use microscope_things::sdk_camera::{CameraSdk, Sdk, SdkCamera, SdkCameraConfig};

/// A camera on `fake`, with a background detector, its data in a folder of
/// its own.
async fn start(fake: &FakeCameraSdk) -> (Harness<SdkCamera>, PathBuf) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data =
        std::env::temp_dir().join(format!("microscope-sdk-camera-{}-{n}", std::process::id()));
    let config = SdkCameraConfig {
        sdk: Sdk::Pylon,
        serial_number: None,
        fake: Some(fake.clone()),
    };
    let harness = Harness::builder("camera", SdkCamera::from_config(config))
        .thing(
            "bg_channel_deviations_luv",
            ChannelDeviationDetector::default(),
        )
        .application_config(json!({ "data_folder": data }))
        .start()
        .await
        .expect("started");
    (harness, data)
}

/// Runs an action over HTTP, and returns its final record.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let started = client.post_json(action, Some(&input)).await;
    assert_eq!(started.status.as_u16(), 201, "{}", started.text());
    let invocation = format!(
        "/action_invocations/{}",
        started.json()["id"].as_str().expect("an id")
    );
    loop {
        let record = client.get(&invocation).await.json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn it_streams_and_captures_whole_frames_to_memory_and_the_data_folder() {
    let fake = FakeCameraSdk::new((1600, 1200));
    let (harness, data) = start(&fake).await;
    let camera = harness.thing();
    let frame = camera.grab_frame().await.expect("a frame");
    assert_eq!(frame.dimensions(), (1024, 768));
    assert_eq!(camera.stream_info().affordance, "mjpeg_stream");
    let capture = camera
        .capture_to_memory(CaptureMode::Standard)
        .await
        .expect("captured");
    assert_eq!(capture.image.dimensions(), (1600, 1200));
    assert_eq!(capture.metadata.camera_settings["exposure_time"], 1000.0);

    let client = harness.client();
    let saved = run(client, "/camera/capture", json!({"retain_image": true})).await;
    assert_eq!(saved["status"], "completed", "{saved}");
    let captures = std::fs::read_dir(data.join("captures")).expect("listed");
    let mut kinds: Vec<String> = (captures.flatten())
        .filter_map(|entry| Some(entry.path().extension()?.to_str()?.to_owned()))
        .collect();
    kinds.sort();
    assert_eq!(kinds, ["jpeg", "json"], "the image and its sidecar");
    let kept = run(client, "/camera/capture_to_memory", json!({})).await;
    assert_eq!(kept["status"], "completed", "{kept}");
    let png = run(
        client,
        "/camera/save_from_memory",
        json!({"path": "kept.png"}),
    )
    .await;
    assert_eq!(png["status"], "completed", "{png}");
    let kept = image::open(data.join("kept.png")).expect("a PNG");
    assert_eq!((kept.width(), kept.height()), (1600, 1200));
    harness.stop().await;
}

#[tokio::test]
async fn its_settings_are_the_cameras_and_its_panel_shows_what_can_be_shown() {
    let fake = FakeCameraSdk::new((640, 480));
    let (harness, _) = start(&fake).await;
    let client = harness.client();
    assert_eq!(client.get("/camera/exposure_time").await.json(), 1000.0);
    let set = client
        .put_json("/camera/exposure_time", &json!(2500.0))
        .await;
    assert!(set.status.is_success(), "{}", set.text());
    assert_eq!(fake.feature("ExposureTime").as_deref(), Some("2500"));
    let refused = client
        .put_json("/camera/pixel_format", &json!("Mono12p"))
        .await;
    assert!(!refused.status.is_success(), "a packed format");
    assert_eq!(fake.feature("PixelFormat").as_deref(), Some("Mono8"));

    let panel = client
        .get("/camera/manual_camera_settings")
        .await
        .json()
        .to_string();
    for shown in ["exposure_time", "gain", "pixel_format", "\"Mono12\""] {
        assert!(panel.contains(shown), "{shown}: {panel}");
    }
    assert!(!panel.contains("Mono12p"), "{panel}");
    harness.stop().await;
}

#[tokio::test]
async fn calibrating_runs_its_auto_exposure_then_sets_the_background() {
    let fake = FakeCameraSdk::new((640, 480));
    let (harness, _) = start(&fake).await;
    let client = harness.client();
    assert_eq!(
        client.get("/camera/calibration_required").await.json(),
        true
    );
    let calibrated = run(client, "/camera/full_auto_calibrate", json!({})).await;
    assert_eq!(calibrated["status"], "completed", "{calibrated}");
    assert_eq!(fake.feature("ExposureTime").as_deref(), Some("2000"));
    assert_eq!(
        client.get("/camera/calibration_required").await.json(),
        false
    );
    harness.stop().await;
}

#[tokio::test]
async fn a_camera_in_a_format_it_cant_show_is_switched_to_one_it_can() {
    let mut fake = FakeCameraSdk::new((640, 480));
    CameraSdk::set(&mut fake, "PixelFormat", "Mono12p").expect("set");
    let (harness, _) = start(&fake).await;
    assert_eq!(fake.feature("PixelFormat").as_deref(), Some("Mono8"));
    harness.thing().grab_frame().await.expect("a frame");
    harness.stop().await;
}

#[tokio::test]
async fn a_camera_unplugged_is_opened_again_once_its_frames_stop() {
    let fake = FakeCameraSdk::new((640, 480));
    let (harness, _) = start(&fake).await;
    harness.thing().grab_frame().await.expect("a frame");
    assert_eq!(fake.starts(), 1);
    fake.unplug();
    let deadline = Instant::now() + Duration::from_secs(15);
    while fake.starts() < 2 {
        assert!(Instant::now() < deadline, "not opened again");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    harness.thing().grab_frame().await.expect("a frame again");
    harness.stop().await;
}

/// On a Basler camera, or pylon's emulator: frames stream, a capture is the
/// camera's whole frame, and the exposure written reads back.
#[tokio::test]
async fn on_a_basler_camera_frames_stream_and_captures_are_whole() {
    let Some(serial) = hardware_device("MICROSCOPE_BASLER_SERIAL") else {
        return;
    };
    let config: SdkCameraConfig =
        serde_json::from_value(json!({"sdk": "pylon", "serial_number": serial})).expect("config");
    let harness = Harness::builder("camera", SdkCamera::from_config(config))
        .start()
        .await
        .expect("the camera opens");
    let camera = harness.thing();
    camera.grab_frame().await.expect("a frame");
    let client = harness.client();
    let before: f64 =
        serde_json::from_value(client.get("/camera/exposure_time").await.json()).expect("a number");
    assert!(before > 0.0);
    let set = client
        .put_json("/camera/exposure_time", &json!(5000.0))
        .await;
    assert!(set.status.is_success(), "{}", set.text());
    let exposure: f64 =
        serde_json::from_value(client.get("/camera/exposure_time").await.json()).expect("a number");
    assert!((exposure - 5000.0).abs() < 100.0, "{exposure}");
    let capture = camera
        .capture_to_memory(CaptureMode::Standard)
        .await
        .expect("captured");
    eprintln!("a {:?} capture", capture.image.dimensions());
    assert!(capture.image.width() >= camera.frame_size().0);
    harness.stop().await;
}
