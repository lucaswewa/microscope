//! Stage measurement (P30) against the simulator, with its travel limits on:
//! the range of motion is found from the image to within a few steps, even
//! past patches without detail; the focus's shift at the ends is measured
//! with autofocus; and recentring makes the middle of the range the stage's
//! zero.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use microscope_sim::{BlobConfig, BlobSpecimen, Point, Specimen};

use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::Autofocus;
use microscope_things::camera::SimulatedCamera;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::mapping::CameraStageMapping;
use microscope_things::stage::SimulatedStage;
use microscope_things::stage_measure::StageMeasure;

/// A quick stage that ends at `x` and `y` (min, max), and a calibrated
/// mapping; with autofocus if asked, when z moves at its usual speed. Also
/// the data folder.
async fn start(x: [i64; 2], y: [i64; 2], autofocus: bool) -> (Harness<StageMeasure>, PathBuf) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data = std::env::temp_dir().join(format!("microscope-measure-{}-{n}", std::process::id()));
    let mut builder = Harness::builder("stage_measure", StageMeasure::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("camera_stage_mapping", CameraStageMapping::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true);
    if autofocus {
        builder = builder.thing("autofocus", Autofocus::default());
    }
    let harness = builder.start().await.expect("the harness starts");
    let client = harness.client();
    let mut simulation = client.get("/stage/simulation").await.json();
    let z_speed = if autofocus { 500.0 } else { 1e6 };
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": z_speed});
    simulation["limits"] = json!(true);
    let travel = |[min, max]: [i64; 2]| json!({"min": min, "max": max});
    simulation["travel"] = json!({"x": travel(x), "y": travel(y), "z": travel([-20_000, 20_000])});
    put(client, "/stage/simulation", simulation).await;
    put(client, "/camera/blob_density", json!(10_000.0)).await;
    let calibrated = run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
    assert_eq!(calibrated["status"], "completed", "{calibrated}");
    (harness, data)
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

/// Runs an action over HTTP: its final record.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let response = client.post_json(action, Some(&input)).await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let id = response.json()["id"].as_str().expect("an id").to_owned();
    loop {
        let record = client
            .get(&format!("/action_invocations/{id}"))
            .await
            .json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

fn assert_near(found: &Value, expected: i64, within: i64) {
    let found = found.as_i64().expect("a number");
    assert!(
        (found - expected).abs() <= within,
        "{found} against {expected}"
    );
}

#[tokio::test]
async fn finds_the_range_from_the_image_and_recentres_on_it() {
    let (harness, _) = start([-2500, 3500], [-3000, 2000], false).await;
    let client = harness.client();
    let early = run(client, "/stage_measure/perform_recentre", json!({})).await;
    assert!(
        early["error"].to_string().contains("measure the range"),
        "{early}"
    );
    // Not with frames of another size than the mapping's.
    let full = json!({"mode": "full_resolution"});
    run(client, "/camera/change_streaming_mode", full).await;
    let other = run(client, "/stage_measure/perform_rom_test", json!({})).await;
    assert!(
        other["error"].to_string().contains("calibrated at"),
        "{other}"
    );
    let default = json!({"mode": "default"});
    run(client, "/camera/change_streaming_mode", default).await;

    let record = run(client, "/stage_measure/perform_rom_test", json!({})).await;
    assert_eq!(record["status"], "completed", "{record}");
    let range = &record["output"];
    for (axis, [min, max]) in [("x", [-2500, 3500]), ("y", [-3000, 2000])] {
        assert_near(&range[axis]["min"], min, 3);
        assert_near(&range[axis]["max"], max, 3);
        assert_eq!(range[axis]["focus_shift"], Value::Null);
    }
    assert_eq!(
        &client.get("/stage_measure/calibrated_range").await.json(),
        range
    );
    // It went back to where it started.
    assert_eq!(
        client.get("/stage/position").await.json(),
        json!({"x": 0, "y": 0, "z": 0})
    );

    let record = run(client, "/stage_measure/perform_recentre", json!({})).await;
    assert_eq!(record["status"], "completed", "{record}");
    assert_near(&record["output"]["x"], 500, 3);
    assert_near(&record["output"]["y"], -500, 3);
    assert_eq!(
        client.get("/stage/position").await.json(),
        json!({"x": 0, "y": 0, "z": 0})
    );
    let centred = client.get("/stage_measure/calibrated_range").await.json();
    assert_eq!(
        centred["x"]["min"].as_i64().unwrap() + centred["x"]["max"].as_i64().unwrap(),
        range["x"]["min"].as_i64().unwrap() + range["x"]["max"].as_i64().unwrap()
            - 2 * record["output"]["x"].as_i64().unwrap(),
    );
    // The stage's zero is now the middle: x ends 3,000 steps away.
    let to_end = run(client, "/stage/move_absolute", json!({"x": 2990})).await;
    assert_eq!(to_end["status"], "completed", "{to_end}");
    let past = run(client, "/stage/move_relative", json!({"x": 20})).await;
    assert_eq!(past["status"], "error", "{past}");
    harness.stop().await;
}

#[tokio::test]
async fn measures_how_far_the_focus_moves_at_the_ends() {
    // From x 2000, made the zero, the stage goes 3000 steps either way.
    let (harness, _) = start([-1000, 5000], [-1000, 1000], true).await;
    let client = harness.client();
    run(client, "/stage/move_absolute", json!({"x": 2000})).await;
    run(client, "/stage/set_zero_position", json!({})).await;
    // The sample's surface rises 60 µm per mm in x, so 0.3 mm along, the
    // stage must come down 360 z steps more to keep it in focus. Autofocus
    // finds focus to within a few µm (about 80 steps on a slow machine).
    put(client, "/camera/focal_tilt_x", json!(60.0)).await;
    let record = run(client, "/stage_measure/perform_rom_test", json!({})).await;
    assert_eq!(record["status"], "completed", "{record}");
    let range = &record["output"];
    assert_near(&range["x"]["focus_shift"][0], 360, 120);
    assert_near(&range["x"]["focus_shift"][1], -360, 120);
    assert_near(&range["y"]["focus_shift"][0], 0, 120);
    assert_near(&range["y"]["focus_shift"][1], 0, 120);
    assert_near(&range["x"]["min"], -3000, 5);
    assert_near(&range["x"]["max"], 3000, 5);
    harness.stop().await;
}

/// Dense blobs 640 × 300 µm, at 0.5 µm a pixel, with an empty patch on each
/// side of the origin along x, from 165 to 240 µm out and 70 µm high.
fn save_holed_picture(path: &Path) {
    let blobs = BlobSpecimen::new(BlobConfig {
        density_per_mm2: 10_000.0,
        ..BlobConfig::default()
    });
    let mut picture = blobs.render(Point::new(-320.0, -150.0), 0.5, 1280, 600);
    for (i, j, pixel) in picture.enumerate_pixels_mut() {
        let (x, y) = (f64::from(i) * 0.5 - 320.0, f64::from(j) * 0.5 - 150.0);
        if (165.0..240.0).contains(&x.abs()) && y.abs() < 35.0 {
            *pixel = image::Rgb([246, 244, 240]);
        }
    }
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("created");
    picture.save(path).expect("saved");
}

#[tokio::test]
async fn steps_aside_where_the_image_has_no_detail() {
    let (harness, data) = start([-2000, 2000], [-1000, 1000], false).await;
    let client = harness.client();
    // Both ends of x (200 µm out) are in empty patches. The stage starts at
    // the top of y, so the first step aside, up, goes nowhere.
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["travel"]["y"]["max"] = json!(0);
    put(client, "/stage/simulation", simulation).await;
    save_holed_picture(&data.join("pictures/holed.png"));
    put(client, "/camera/sample_type", json!("image")).await;
    put(client, "/camera/sample_image_scale", json!(0.5)).await;
    put(client, "/camera/sample_image", json!("pictures/holed.png")).await;
    let record = run(client, "/stage_measure/perform_rom_test", json!({})).await;
    assert_eq!(record["status"], "completed", "{record}");
    let range = &record["output"];
    for (axis, [min, max]) in [("x", [-2000, 2000]), ("y", [-1000, 0])] {
        assert_near(&range[axis]["min"], min, 3);
        assert_near(&range[axis]["max"], max, 3);
    }
    assert_eq!(
        client.get("/stage/position").await.json(),
        json!({"x": 0, "y": 0, "z": 0})
    );
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
