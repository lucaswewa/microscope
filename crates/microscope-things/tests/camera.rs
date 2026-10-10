//! The simulated camera and illumination (P17): frames that follow the
//! stage, the focus and the light; the streams; the settings; and
//! `CameraApi`. Then captures held in memory and saved, and settling (P17b).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use image::RgbImage;
use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::camera::{SimulatedCamera, SimulatedCameraActions};
use microscope_things::hardware::{CameraApi, Position, StageApi};
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

/// A camera with a fast stage and a light, and no noise, so the same view
/// draws the same frame. The global lock is on, as in the shipped
/// configuration.
async fn start() -> Harness<SimulatedCamera> {
    start_with(json!({})).await
}

/// The same, with an `application_config`.
async fn start_with(application_config: Value) -> Harness<SimulatedCamera> {
    let harness = Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .application_config(application_config)
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    put(client, "/camera/noise_level", json!(0.0)).await;
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 1e6});
    put(client, "/stage/simulation", simulation).await;
    harness
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

/// Runs an action over HTTP: its record.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let response = client.post_json(action, Some(&input)).await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let invocation = format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    );
    for _ in 0..1000 {
        let record = client.get(&invocation).await.json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{action} didn't end");
}

fn stage(harness: &Harness<SimulatedCamera>) -> ThingRef<SimulatedStage> {
    let runtime = harness.client().runtime();
    runtime
        .thing_ref::<SimulatedStage>("stage")
        .expect("configured")
}

/// A frame drawn after everything before the call: the first frame may
/// have been started before it.
async fn fresh_frame(harness: &Harness<SimulatedCamera>) -> RgbImage {
    let camera = harness.thing();
    camera.grab_frame().await.expect("a frame");
    camera.grab_frame().await.expect("a frame")
}

/// The mean squared difference between neighbouring pixels: it falls as a
/// frame blurs.
fn sharpness(frame: &RgbImage) -> f64 {
    let (width, height) = frame.dimensions();
    let mut total = 0.0;
    for y in 0..height {
        for x in 1..width {
            let d = f64::from(frame.get_pixel(x, y)[1]) - f64::from(frame.get_pixel(x - 1, y)[1]);
            total += d * d;
        }
    }
    total / f64::from((width - 1) * height)
}

/// The mean absolute difference between `a` moved by (`dx`, `dy`) pixels
/// and `b`, where they overlap.
fn shifted_difference(a: &RgbImage, b: &RgbImage, dx: u32, dy: u32) -> f64 {
    let (width, height) = a.dimensions();
    let mut total = 0.0;
    for y in 0..height - dy {
        for x in 0..width - dx {
            let (p, q) = (a.get_pixel(x + dx, y + dy), b.get_pixel(x, y));
            total += (0..3).map(|c| f64::from(p[c].abs_diff(q[c]))).sum::<f64>();
        }
    }
    total / f64::from(3 * (width - dx) * (height - dy))
}

fn mean(frame: &RgbImage) -> f64 {
    frame.iter().map(|&v| f64::from(v)).sum::<f64>() / frame.len() as f64
}

#[tokio::test]
async fn frames_follow_the_stage_in_x_and_y() {
    let harness = start().await;
    let stage = stage(&harness);
    let origin = fresh_frame(&harness).await;
    assert_eq!(origin.dimensions(), (820, 616));
    assert!(sharpness(&origin) > 2.0, "the sample shows");

    // 28 µm is 250 pixels at 40× (0.112 µm per pixel).
    for (step, dx, dy) in [
        (Position::new(280, 0, 0), 250, 0),
        (Position::new(0, 280, 0), 0, 250),
    ] {
        stage.move_absolute(step).await.expect("moved");
        let moved = fresh_frame(&harness).await;
        assert!(shifted_difference(&origin, &moved, 0, 0) > 10.0);
        let shifted = shifted_difference(&origin, &moved, dx, dy);
        assert!(shifted < 0.5, "the view moved {dx}, {dy} pixels: {shifted}");
    }
    stage
        .move_absolute(Position::new(0, 0, 0))
        .await
        .expect("moved");
    assert_eq!(fresh_frame(&harness).await, origin);
    harness.stop().await;
}

#[tokio::test]
async fn sharpness_falls_as_the_stage_leaves_focus() {
    let harness = start().await;
    let stage = stage(&harness);
    // z is 0.05 µm per step: 0, 2, 5, 10 and 20 µm from focus.
    let mut previous = f64::INFINITY;
    for z in [0, 40, 100, 200, 400] {
        stage
            .move_absolute(Position::new(0, 0, z))
            .await
            .expect("moved");
        let now = sharpness(&fresh_frame(&harness).await);
        assert!(now < previous, "{z} steps: {now} after {previous}");
        previous = now;
    }
    // Below focus blurs as much as above.
    stage
        .move_absolute(Position::new(0, 0, 100))
        .await
        .expect("moved");
    let above = sharpness(&fresh_frame(&harness).await);
    stage
        .move_absolute(Position::new(0, 0, -100))
        .await
        .expect("moved");
    let below = sharpness(&fresh_frame(&harness).await);
    assert!((above - below).abs() < 0.05 * above, "{above} and {below}");
    harness.stop().await;
}

#[tokio::test]
async fn the_led_lights_the_frames() {
    let harness = start().await;
    let client = harness.client();
    run(client, "/illumination/set_led", json!({"led_on": false})).await;
    assert_eq!(
        client.get("/illumination/led_on").await.json(),
        json!(false)
    );
    assert!(fresh_frame(&harness).await.iter().all(|&v| v == 0), "black");
    run(client, "/illumination/set_led", json!({})).await;
    assert!(mean(&fresh_frame(&harness).await) > 100.0);
    harness.stop().await;

    // Without an illumination Thing, the light is always on.
    let harness = Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .start()
        .await
        .expect("the harness starts");
    assert!(mean(&fresh_frame(&harness).await) > 100.0);
    harness.stop().await;
}

#[tokio::test]
async fn flashing_ends_with_the_led_on() {
    let harness = start().await;
    let client = harness.client();
    let response = client
        .post_json(
            "/illumination/flash",
            Some(&json!({"number_of_flashes": 3, "dt": 0.05})),
        )
        .await;
    let invocation = format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    );
    let mut seen_off = false;
    while matches!(
        client.get(&invocation).await.json()["status"].as_str(),
        Some("pending" | "running")
    ) {
        seen_off |= client.get("/illumination/led_on").await.json() == json!(false);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(seen_off);
    assert_eq!(client.get("/illumination/led_on").await.json(), json!(true));

    // Cancelled while off, it still ends on.
    let response = client
        .post_json("/illumination/flash", Some(&json!({"dt": 10.0})))
        .await;
    let invocation = format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    );
    while client.get("/illumination/led_on").await.json() != json!(false) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(client.delete(&invocation).await.status.is_success());
    while client.get(&invocation).await.json()["status"] != "cancelled" {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(client.get("/illumination/led_on").await.json(), json!(true));
    let record = run(client, "/illumination/flash", json!({"dt": -1.0})).await;
    assert_eq!(record["status"], "error");
    harness.stop().await;
}

#[tokio::test]
async fn both_streams_serve_jpegs_and_stop_on_shutdown() {
    let harness = start().await;
    let client = harness.client();
    assert_eq!(
        client.get("/camera/stream_active").await.json(),
        json!(true)
    );
    for (stream, size) in [("main", (820, 616)), ("lores", (320, 240))] {
        let record = run(client, "/camera/grab_jpeg", json!({"stream_name": stream})).await;
        assert_eq!(record["status"], "completed", "{record}");
        let output = client
            .get(&format!(
                "/action_invocations/{}/output",
                record["id"].as_str().expect("an id")
            ))
            .await;
        assert_eq!(output.header("content-type"), Some("image/jpeg"));
        let jpeg = image::load_from_memory(&output.body).expect("a JPEG");
        assert_eq!((jpeg.width(), jpeg.height()), size);
        let record = run(
            client,
            "/camera/grab_jpeg_size",
            json!({"stream_name": stream}),
        )
        .await;
        assert!(record["output"].as_u64().expect("bytes") > 1000, "{record}");
    }

    let stream = ThingRef::thing(harness.thing()).mjpeg_stream().clone();
    let mut frames = stream.frames();
    assert!(frames.next().await.is_some());
    harness.stop().await;
    assert!(!stream.is_streaming());
    let ended = tokio::time::timeout(Duration::from_secs(5), async {
        while frames.next().await.is_some() {}
    });
    assert!(ended.await.is_ok(), "the stream ended");
}

#[tokio::test]
async fn the_full_resolution_mode_streams_capture_sized_frames() {
    let harness = start().await;
    let client = harness.client();
    let modes = client.get("/camera/streaming_modes").await.json();
    assert_eq!(modes["default"]["width"], 820);
    assert_eq!(modes["full_resolution"]["width"], 1640);
    assert_eq!(client.get("/camera/streaming_mode").await.json(), "default");

    run(
        client,
        "/camera/change_streaming_mode",
        json!({"mode": "full_resolution"}),
    )
    .await;
    assert_eq!(
        client.get("/camera/streaming_mode").await.json(),
        "full_resolution"
    );
    assert_eq!(harness.thing().frame_size(), (1640, 1232));
    assert_eq!(fresh_frame(&harness).await.dimensions(), (1640, 1232));
    let rejected = client
        .post_json(
            "/camera/change_streaming_mode",
            Some(&json!({"mode": "huge"})),
        )
        .await;
    assert_eq!(rejected.status.as_u16(), 422);
    run(client, "/camera/change_streaming_mode", json!({})).await;
    assert_eq!(harness.thing().frame_size(), (820, 616));
    harness.stop().await;
}

#[tokio::test]
async fn settings_change_the_frames() {
    let harness = start().await;
    let client = harness.client();
    let original = fresh_frame(&harness).await;

    // Half the exposure at twice the gain is as bright as before.
    put(client, "/camera/exposure_time", json!(5000)).await;
    let darker = fresh_frame(&harness).await;
    assert!((mean(&darker) / mean(&original) - 0.5).abs() < 0.01);
    put(client, "/camera/analogue_gain", json!(2.0)).await;
    assert_eq!(fresh_frame(&harness).await, original);

    // Other objectives, colours and densities show the sample differently.
    for (setting, value) in [
        ("objective", json!(10)),
        ("colour", json!("#2050a0")),
        ("blob_density", json!(100.0)),
    ] {
        let path = format!("/camera/{setting}");
        let before = client.get(&path).await.json();
        put(client, &path, value).await;
        assert_ne!(fresh_frame(&harness).await, original, "{setting}");
        put(client, &path, before).await;
    }
    assert_eq!(fresh_frame(&harness).await, original);
    for (setting, value) in [
        ("objective", json!(7)),
        ("colour", json!("red")),
        ("noise_level", json!(-1)),
    ] {
        let response = client.put_json(&format!("/camera/{setting}"), &value).await;
        assert!(response.status.is_client_error(), "{setting} = {value}");
    }
    harness.stop().await;
}

#[tokio::test]
async fn the_sample_can_be_removed_and_repeated() {
    let harness = start().await;
    let client = harness.client();
    let original = fresh_frame(&harness).await;
    let uniform = |frame: &RgbImage| frame.pixels().all(|p| p == frame.get_pixel(0, 0));
    assert!(!uniform(&original));

    assert_eq!(
        run(client, "/camera/remove_sample", json!({})).await["status"],
        "completed"
    );
    assert_eq!(
        client.get("/camera/sample_loaded").await.json(),
        json!(false)
    );
    assert!(uniform(&fresh_frame(&harness).await), "the empty slide");
    assert_eq!(
        run(client, "/camera/remove_sample", json!({})).await["status"],
        "error"
    );
    run(client, "/camera/load_sample", json!({})).await;
    assert_eq!(fresh_frame(&harness).await, original);
    assert_eq!(
        run(client, "/camera/load_sample", json!({})).await["status"],
        "error"
    );

    // 15 mm out is past the sample's edge, unless it repeats.
    stage(&harness)
        .move_absolute(Position::new(150_000, 0, 0))
        .await
        .expect("moved");
    assert!(uniform(&fresh_frame(&harness).await));
    put(client, "/camera/repeating", json!(true)).await;
    assert!(!uniform(&fresh_frame(&harness).await));
    harness.stop().await;
}

#[tokio::test]
async fn other_things_use_it_through_camera_api() {
    let harness = start().await;
    let stage = stage(&harness);
    stage
        .move_absolute(Position::new(30, -20, 0))
        .await
        .expect("moved");
    let camera: &dyn CameraApi = harness.thing();
    assert_eq!(camera.frame_size(), (820, 616));
    let info = camera.stream_info();
    assert_eq!(
        (info.affordance.as_str(), info.width, info.height),
        ("mjpeg_stream", 820, 616)
    );
    let jpeg = camera.grab_jpeg().await.expect("a JPEG");
    assert_eq!(
        image::load_from_memory(&jpeg).expect("decodes").width(),
        820
    );

    let capture = camera.capture_to_memory().await.expect("a capture");
    assert_eq!(capture.image.dimensions(), (1640, 1232));
    let metadata = capture.metadata;
    assert_eq!((metadata.width, metadata.height), (1640, 1232));
    assert_eq!(metadata.position, Some(Position::new(30, -20, 0)));
    assert_eq!(metadata.axis_scale, Some(stage.axis_scale()));
    assert!((metadata.um_per_pixel.expect("known") - 0.056).abs() < 1e-12);
    assert_eq!(metadata.camera_settings["objective"], json!(40));
    assert!(metadata.acquired.ends_with('Z'), "{}", metadata.acquired);
    harness.stop().await;
}

/// A data folder in the system's temporary folder, unique to this run.
fn data_folder() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_nanos();
    let name = format!("microscope-captures-{}-{nanos}", std::process::id());
    std::env::temp_dir().join(name).join("data")
}

#[tokio::test]
async fn captures_wait_in_memory_until_saved() {
    let data = data_folder();
    let harness = start_with(json!({"data_folder": data})).await;
    let client = harness.client();
    let capture = || {
        run(
            client,
            "/camera/capture_to_memory",
            json!({"buffer_max": 2}),
        )
    };
    for id in 1..=3 {
        assert_eq!(capture().await["output"], json!(id));
    }
    let save = |path: &str, id: Value| {
        let input = json!({"path": path, "buffer_id": id});
        async move { run(client, "/camera/save_from_memory", input).await["status"].clone() }
    };
    // Only the last two are held.
    assert_eq!(save("one.jpg", json!(1)).await, "error");
    assert_eq!(save("captures/two.jpg", json!(2)).await, "completed");
    let two = image::open(data.join("captures/two.jpg")).expect("a readable JPEG");
    assert_eq!((two.width(), two.height()), (1640, 1232));
    assert_eq!(save("again.jpg", json!(2)).await, "error", "taken out");

    // Paths outside the data folder, and other formats, are refused, and
    // leave the capture in memory.
    for refused in [
        "../outside.jpg",
        "C:/outside.jpg",
        "/outside.jpg",
        "three.bmp",
        "",
    ] {
        assert_eq!(save(refused, Value::Null).await, "error", "{refused}");
    }
    let outside = data.parent().expect("a parent").join("outside.jpg");
    assert!(!outside.exists());
    // With no id, the latest is saved and memory emptied.
    assert_eq!(save("three.PNG", Value::Null).await, "completed");
    assert_eq!(
        image::open(data.join("three.PNG")).expect("a PNG").width(),
        1640
    );
    assert_eq!(save("none.jpg", Value::Null).await, "error");

    capture().await;
    run(client, "/camera/clear_buffers", json!({})).await;
    assert_eq!(save("cleared.jpg", Value::Null).await, "error");
    harness.stop().await;
    std::fs::remove_dir_all(data.parent().expect("a parent")).expect("removed");
}

#[tokio::test]
async fn saving_needs_no_global_lock() {
    let data = data_folder();
    let harness = start_with(json!({"data_folder": data})).await;
    let client = harness.client();
    run(client, "/camera/capture_to_memory", json!({})).await;
    // A flash holds the global lock for a second.
    let flash = client
        .post_json(
            "/illumination/flash",
            Some(&json!({"number_of_flashes": 1, "dt": 0.5})),
        )
        .await;
    let flash = format!(
        "/action_invocations/{}",
        flash.json()["id"].as_str().expect("an id")
    );
    let record = run(
        client,
        "/camera/save_from_memory",
        json!({"path": "during.jpg"}),
    )
    .await;
    assert_eq!(record["status"], "completed");
    assert_eq!(client.get(&flash).await.json()["status"], "running");
    assert!(data.join("during.jpg").exists());
    harness.stop().await;
    std::fs::remove_dir_all(data.parent().expect("a parent")).expect("removed");
}

#[tokio::test]
async fn after_settling_the_next_frame_shows_where_the_stage_is() {
    let harness = start().await;
    let client = harness.client();
    let (stage, camera) = (stage(&harness), harness.thing());
    // Large frames, so one is nearly always being drawn during a move.
    run(
        client,
        "/camera/change_streaming_mode",
        json!({"mode": "full_resolution"}),
    )
    .await;
    let places = [Position::new(0, 0, 0), Position::new(280, 0, 0)];
    let mut views = Vec::new();
    for place in places {
        stage.move_absolute(place).await.expect("moved");
        views.push(fresh_frame(&harness).await);
    }
    put(client, "/camera/settling_time", json!(0.0)).await;
    for round in 0..4 {
        stage.move_absolute(places[round % 2]).await.expect("moved");
        camera.settle().await.expect("settled");
        let frame = camera.grab_frame().await.expect("a frame");
        assert!(
            frame == views[round % 2],
            "round {round}: a frame from before the move"
        );
    }

    // Back to small frames, so waiting for one is quick.
    run(client, "/camera/change_streaming_mode", json!({})).await;
    assert_eq!(fresh_frame(&harness).await.width(), 820);
    put(client, "/camera/settling_time", json!(0.5)).await;
    let started = Instant::now();
    assert_eq!(
        run(client, "/camera/settle", json!({})).await["status"],
        "completed"
    );
    assert!(started.elapsed() >= Duration::from_millis(500));
    harness.stop().await;
}
