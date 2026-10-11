//! The simulated camera's samples and optics (P25): switching the sample
//! while it runs, a picture from the data folder, the optics' settings, and
//! the settings panel, whose controls all name settings the camera has.

use std::path::{Path, PathBuf};

use image::RgbImage;
use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::camera::SimulatedCamera;
use microscope_things::hardware::CameraApi;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

/// A camera without noise, and a quick stage, with its data in `data`.
async fn start(data: &Path) -> Harness<SimulatedCamera> {
    let harness = Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .application_config(json!({"data_folder": data}))
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    put(client, "noise_level", json!(0.0)).await;
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 1e6});
    let response = client.put_json("/stage/simulation", &simulation).await;
    assert!(response.status.is_success());
    harness
}

async fn put(client: &TestClient, setting: &str, value: Value) {
    let response = client.put_json(&format!("/camera/{setting}"), &value).await;
    assert!(
        response.status.is_success(),
        "{setting}: {}",
        response.text()
    );
}

/// A frame drawn after the call.
async fn frame(client: &TestClient) -> RgbImage {
    let camera = client.runtime().thing_ref::<SimulatedCamera>("camera");
    let camera = camera.expect("configured");
    camera.grab_frame().await.expect("a frame");
    camera.grab_frame().await.expect("a frame")
}

/// The mean of each channel over `region` of a frame: (x, y, width, height).
fn mean(frame: &RgbImage, (x0, y0, w, h): (u32, u32, u32, u32)) -> [f64; 3] {
    let n = f64::from(w * h);
    std::array::from_fn(|c| {
        let pixels = (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| (x, y)));
        pixels
            .map(|(x, y)| f64::from(frame.get_pixel(x, y).0[c]))
            .sum::<f64>()
            / n
    })
}

fn near(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    (0..3).all(|c| (a[c] - b[c]).abs() <= tolerance)
}

fn data_folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("microscope-samples-{name}-{}", std::process::id()))
}

const WHOLE: (u32, u32, u32, u32) = (0, 0, 820, 616);

#[tokio::test]
async fn the_sample_can_be_switched_while_the_camera_runs() {
    let data = data_folder("switch");
    std::fs::create_dir_all(data.join("pictures")).expect("made");
    RgbImage::from_pixel(400, 400, image::Rgb([40, 200, 60]))
        .save(data.join("pictures/green.png"))
        .expect("saved");
    let harness = start(&data).await;
    let client = harness.client();

    let blobs = mean(&frame(client).await, WHOLE);
    put(client, "sample_type", json!("tissue")).await;
    let tissue = mean(&frame(client).await, WHOLE);
    assert!(!near(blobs, tissue, 5.0), "{blobs:?} {tissue:?}");

    // A 40 µm square of green, smaller than the field of view at 40×; then
    // 400 µm, larger.
    put(client, "sample_type", json!("image")).await;
    put(client, "sample_image_scale", json!(0.1)).await;
    put(client, "sample_image", json!("pictures/green.png")).await;
    let small = mean(&frame(client).await, WHOLE);
    assert!(!near(small, [40.0, 200.0, 60.0], 20.0), "{small:?}");
    put(client, "sample_image_scale", json!(1.0)).await;
    let picture = mean(&frame(client).await, WHOLE);
    assert!(near(picture, [40.0, 200.0, 60.0], 2.0), "{picture:?}");

    // A picture that can't be loaded leaves the slide empty.
    put(client, "sample_image", json!("pictures/missing.png")).await;
    let empty = mean(&frame(client).await, WHOLE);
    assert!(near(empty, [246.0, 244.0, 240.0], 2.0), "{empty:?}");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn the_optics_settings_reach_the_frame() {
    let data = data_folder("optics");
    let harness = start(&data).await;
    let client = harness.client();
    let removed = client
        .post_json("/camera/remove_sample", Some(&json!({})))
        .await;
    assert!(removed.status.is_success());
    put(client, "vignetting", json!(0.5)).await;
    let vignetted = frame(client).await;
    let (centre, corner) = (
        mean(&vignetted, (400, 298, 20, 20)),
        mean(&vignetted, (0, 0, 20, 20)),
    );
    assert!(corner[1] / centre[1] < 0.6, "{centre:?} {corner:?}");

    // Tilted 20 µm per mm in x, the slide is out of focus 1 mm along.
    put(client, "vignetting", json!(0.0)).await;
    let _ = client
        .post_json("/camera/load_sample", Some(&json!({})))
        .await;
    put(client, "focal_tilt_x", json!(20.0)).await;
    let here = frame(client).await;
    let moved = client
        .post_json("/stage/move_absolute", Some(&json!({"x": 10_000})))
        .await;
    assert!(moved.status.is_success());
    while client.get("/stage/position").await.json()["x"] != json!(10_000) {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let there = frame(client).await;
    let spread = |frame: &RgbImage| {
        let values: Vec<f64> = frame.pixels().map(|p| f64::from(p.0[1])).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.iter().map(|v| (v - mean).abs()).sum::<f64>() / values.len() as f64
    };
    assert!(
        spread(&there) < spread(&here) / 2.0,
        "{} {}",
        spread(&here),
        spread(&there)
    );
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

#[tokio::test]
async fn the_settings_panel_names_only_settings_the_camera_has() {
    let data = data_folder("panel");
    let harness = start(&data).await;
    let response = harness.client().get("/camera/manual_camera_settings").await;
    assert!(response.status.is_success(), "{}", response.text());

    fn controls(elements: &Value, found: &mut Vec<String>) {
        for element in elements.as_array().expect("a list") {
            if element["kind"] == "property_control" {
                assert_eq!(element.get("broken"), None, "{element}");
                found.push(element["property"].as_str().expect("named").to_owned());
            }
            if let Some(children) = element.get("children") {
                controls(children, found);
            }
        }
    }
    let mut found = Vec::new();
    controls(&response.json(), &mut found);
    found.sort();
    let mut expected = [
        "analogue_gain",
        "blob_density",
        "colour",
        "exposure_time",
        "focal_tilt_x",
        "focal_tilt_y",
        "focal_waviness",
        "noise_level",
        "objective",
        "repeating",
        "sample_image",
        "sample_image_scale",
        "sample_type",
        "tissue_coverage",
        "uneven_illumination",
        "vignetting",
    ];
    expected.sort();
    assert_eq!(found, expected);
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}
