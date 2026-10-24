//! Correlation in stitching (P13) on the simulator: a drifting stage's
//! scan is stitched where its images really are, and without correlation,
//! where the stage put them. It's a test target of its own, so its scan,
//! autofocused and captured whole, doesn't run beside `smart_scan.rs`'s and
//! slow their cameras.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use teta_wot::FromConfig;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::Autofocus;
use microscope_things::camera::SimulatedCamera;
use microscope_things::gallery::Gallery;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::mapping::CameraStageMapping;
use microscope_things::smart_scan::SmartScan;
use microscope_things::stage::SimulatedStage;
use microscope_things::workflow::{GridOrder, GridWorkflow, GridWorkflowConfig};

/// A microscope with a calibrated mapping and a 3 × 3 snake, its data in a
/// new folder.
async fn start() -> (Harness<SmartScan>, PathBuf) {
    let data = std::env::temp_dir().join(format!("microscope-correlation-{}", std::process::id()));
    let snake = GridWorkflow::from_config(GridWorkflowConfig {
        order: GridOrder::Snake,
    });
    let harness = Harness::builder("smart_scan", SmartScan::default())
        .thing("snake_workflow", snake)
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("autofocus", Autofocus::default())
        .thing("camera_stage_mapping", CameraStageMapping::default())
        .thing("gallery", Gallery::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 500});
    put(client, "/stage/simulation", simulation).await;
    for (setting, value) in [
        ("x_count", json!(3)),
        ("y_count", json!(3)),
        ("autofocus_dz", json!(400)),
        ("stack_min_images_to_test", json!(4)),
    ] {
        put(client, &format!("/snake_workflow/{setting}"), value).await;
    }
    let calibrated = run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
    assert_eq!(calibrated["status"], "completed", "{calibrated}");
    put(client, "/smart_scan/workflow_name", json!("snake_workflow")).await;
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

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read")).expect("JSON")
}

/// The largest distance, in pixels, between where `tiles.json` put each
/// image and where it really is, by its sidecar, once their common offset is
/// taken out.
fn off_by(scan: &Path) -> f64 {
    let tiles = read_json(&scan.join("stitched/tiles.json"));
    let tiles = tiles["tiles"].as_array().expect("tiles");
    let gaps: Vec<[f64; 2]> = (tiles.iter())
        .map(|tile| {
            let image = tile["image"].as_str().expect("a name");
            let sidecar = scan.join("images").join(image).with_extension("json");
            let sidecar = read_json(&sidecar);
            let (truth, um_per_px) = (
                &sidecar["simulation"]["true_position_um"],
                sidecar["um_per_pixel"].as_f64().expect("a scale"),
            );
            [0, 1].map(|axis| {
                let placed = tile[["x", "y"][axis]].as_f64().expect("placed");
                placed - truth[axis].as_f64().expect("a position") / um_per_px
            })
        })
        .collect();
    let mean = |axis: usize| gaps.iter().map(|gap| gap[axis]).sum::<f64>() / gaps.len() as f64;
    let (mx, my) = (mean(0), mean(1));
    (gaps.iter())
        .map(|gap| (gap[0] - mx).hypot(gap[1] - my))
        .fold(0.0, f64::max)
}

#[tokio::test]
async fn a_drifting_stage_s_scan_is_stitched_where_its_images_really_are() {
    let (harness, data) = start().await;
    let client = harness.client();
    // Autofocused and captured whole, as scans are by default, of the
    // default sample: at a capture's detail, the JPEG's blocks are what
    // correlation must not follow (P13).
    put(client, "/snake_workflow/capture_mode", json!("standard")).await;
    put(client, "/camera/blob_density", json!(1500.0)).await;
    // 2% of the steps lost, once the mapping is calibrated.
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["drift"] = json!(true);
    let none = json!({"x": 0.0, "y": 0.0, "z": 0.0});
    let lost = json!({"x": 0.02, "y": 0.02, "z": 0.0});
    simulation["drift_size"] = json!({"lost_steps": lost, "creep": none});
    put(client, "/stage/simulation", simulation).await;
    put(client, "/smart_scan/stitch_automatically", json!(true)).await;
    let scanned = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "drift"}),
    )
    .await;
    assert_eq!(scanned["status"], "completed", "{scanned}");

    // Stitched once it ends, refined by correlation, as it is by default.
    let scan = data.join("scans/drift");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let log = loop {
        let log = std::fs::read_to_string(scan.join("scan.log")).expect("read");
        if log.contains("Stitched") {
            break log;
        }
        assert!(std::time::Instant::now() < deadline, "not stitched: {log}");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    // Most of them moved: one near where they are on average may not have.
    let moved = log
        .split("correlation moved ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next());
    let moved: u32 = moved.and_then(|n| n.parse().ok()).expect("how many moved");
    assert!(
        moved >= 6 && log.contains(" of the 9 images, the furthest by "),
        "{log}"
    );
    // Within a pixel or two: a corner image of a sparse sample has the
    // fewest pairs to go by.
    let refined = off_by(&scan);
    assert!(refined < 2.5, "refined, off by {refined} pixels");
    // Where the stage put each image is recorded too.
    let tiles = read_json(&scan.join("stitched/tiles.json"));
    let moved = (tiles["tiles"].as_array().expect("tiles").iter())
        .filter(|tile| tile["x"] != tile["stage_x"] || tile["y"] != tile["stage_y"]);
    assert!(moved.count() >= 6, "{tiles}");

    // Without correlation, where the stage put them.
    put(client, "/smart_scan/refine_with_correlation", json!(false)).await;
    let stitched = run(
        client,
        "/smart_scan/stitch_scan",
        json!({"scan_name": "drift"}),
    )
    .await;
    assert_eq!(stitched["output"]["correlation"], Value::Null, "{stitched}");
    let unrefined = off_by(&scan);
    assert!(
        unrefined > 10.0,
        "unrefined, off by only {unrefined} pixels"
    );
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
