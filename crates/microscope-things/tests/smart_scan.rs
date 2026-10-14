//! Scanning (P35) on the simulator: a scan visits its grid, saving a smart
//! stack's sharpest image (P36) at each site with its sidecar, its record and
//! its log in a folder of its own, and goes back to the start. A cancelled
//! scan, and one that fails part of the way, go back to the start too. Only
//! one scan runs at a time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

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

/// A microscope with both grid workflows and a calibrated mapping, its data
/// in a new folder, and a 2 × 2 snake chosen.
async fn start(calibrate: bool) -> (Harness<SmartScan>, PathBuf) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data = std::env::temp_dir().join(format!("microscope-scan-{}-{n}", std::process::id()));
    let workflow = |order| GridWorkflow::from_config(GridWorkflowConfig { order });
    let harness = Harness::builder("smart_scan", SmartScan::default())
        .thing("snake_workflow", workflow(GridOrder::Snake))
        .thing("raster_workflow", workflow(GridOrder::Raster))
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
    put(client, "/camera/blob_density", json!(10_000.0)).await;
    for (setting, value) in [
        ("x_count", json!(2)),
        ("y_count", json!(2)),
        ("autofocus_dz", json!(400)),
        ("capture_mode", json!("quick")),
        // Stacks test an odd number of images, so 5 (P36).
        ("stack_min_images_to_test", json!(4)),
    ] {
        put(client, &format!("/snake_workflow/{setting}"), value).await;
    }
    if calibrate {
        let calibrated = run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
        assert_eq!(calibrated["status"], "completed", "{calibrated}");
    }
    (harness, data)
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

/// Starts an action over HTTP: its invocation's path.
async fn invoke(client: &TestClient, action: &str, input: Value) -> String {
    let response = client.post_json(action, Some(&input)).await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    )
}

/// Runs an action over HTTP: its final record.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let invocation = invoke(client, action, input).await;
    finish(client, &invocation).await
}

async fn finish(client: &TestClient, invocation: &str) -> Value {
    loop {
        let record = client.get(invocation).await.json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read")).expect("JSON")
}

/// The files in a scan's `images/`, sorted.
fn images(scan: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(scan.join("images"))
        .expect("listed")
        .map(|entry| {
            entry
                .expect("listed")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// The moves a scan's log records, each [x, y, z].
fn moves(scan: &Path) -> Vec<Vec<i64>> {
    let log = std::fs::read_to_string(scan.join("scan.log")).expect("read");
    (log.lines())
        .filter_map(|line| line.split("Moving to x ").nth(1))
        .map(|rest| {
            (rest.split(|c: char| !c.is_ascii_digit() && c != '-'))
                .filter(|part| !part.is_empty())
                .map(|part| part.parse().expect("a number"))
                .collect()
        })
        .collect()
}

/// The z the image at (x, y) was taken at, from its name.
fn focus(scan: &Path, x: i64, y: i64) -> i64 {
    let prefix = format!("img_{x}_{y}_");
    let name = images(scan)
        .into_iter()
        .find(|f| f.starts_with(&prefix) && f.ends_with(".jpeg"));
    let name = name.expect("an image there");
    let z = name
        .trim_end_matches(".jpeg")
        .rsplit('_')
        .next()
        .expect("a z");
    z.parse().expect("a number")
}

#[tokio::test]
async fn a_scan_visits_its_grid_and_keeps_its_images_and_record() {
    let (harness, data) = start(true).await;
    let client = harness.client();
    assert_eq!(
        client
            .get("/smart_scan/workflow_display_names")
            .await
            .json(),
        json!({"snake_workflow": "Snake scan", "raster_workflow": "Raster scan"})
    );
    // A scan ends by removing folders without an image (P35b).
    std::fs::create_dir_all(data.join("scans/stale/images")).expect("created");
    let begin = client.get("/stage/position").await.json();
    let record = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "first"}),
    )
    .await;
    assert!(!data.join("scans/stale").exists());
    assert_eq!(record["status"], "completed", "{record}");
    assert_eq!(record["output"], "first");
    // Back at the start, z included.
    assert_eq!(client.get("/stage/position").await.json(), begin);

    let scan = data.join("scans/first");
    let files = images(&scan);
    assert_eq!(files.len(), 8, "{files:?}");
    let jpegs: Vec<&String> = files.iter().filter(|f| f.ends_with(".jpeg")).collect();
    assert_eq!(jpegs.len(), 4);
    // Each named for where it was taken, with its sidecar.
    let saved = read_json(&scan.join("scan.json"));
    let (dx, dy) = (
        saved["settings"]["dx"].as_i64().unwrap(),
        saved["settings"]["dy"].as_i64().unwrap(),
    );
    let (x0, y0) = (begin["x"].as_i64().unwrap(), begin["y"].as_i64().unwrap());
    for (x, y) in [(x0, y0), (x0 + dx, y0), (x0, y0 + dy), (x0 + dx, y0 + dy)] {
        let prefix = format!("img_{x}_{y}_");
        let jpeg = jpegs
            .iter()
            .find(|f| f.starts_with(&prefix))
            .expect("an image there");
        let sidecar = read_json(&scan.join("images").join(jpeg).with_extension("json"));
        assert_eq!(
            (
                sidecar["stage"]["steps"]["x"].as_i64(),
                sidecar["stage"]["steps"]["y"].as_i64()
            ),
            (Some(x), Some(y))
        );
    }
    assert_eq!(saved["name"], "first");
    assert_eq!(saved["workflow"], "snake_workflow");
    assert_eq!(saved["image_count"], 4);
    assert_eq!(saved["result"], "success");
    assert_eq!(saved["start"], begin);
    assert_eq!(saved["settings"]["order"], "snake");
    for (setting, value) in [("images_to_save", 1), ("min_images_to_test", 5), ("dz", 50)] {
        assert_eq!(saved["settings"][format!("stack_{setting}")], value);
    }
    let log = std::fs::read_to_string(scan.join("scan.log")).expect("read");
    assert_eq!(log.matches("Moving to").count(), 4, "{log}");
    assert!(
        log.contains("Every planned site was visited") && log.contains("Ended: success"),
        "{log}"
    );
    // Each move after the first is at the z the site before it focused at:
    // in a 2 × 2 snake, the nearest focused site.
    let moved = moves(&scan);
    assert_eq!(moved.len(), 4, "{moved:?}");
    for pair in moved.windows(2) {
        assert_eq!(
            pair[1][2],
            focus(&scan, pair[0][0], pair[0][1]),
            "{moved:?}"
        );
    }

    let details = client
        .get("/smart_scan/latest_scan_live_details")
        .await
        .json();
    assert_eq!(details["scan_phase"], "complete");
    assert_eq!(details["image_count"], 4);
    assert_eq!(details["result"], "success");

    // The same name again is made unique. In a 2 × 2 raster on a sample
    // tilted in x, the third site starts from the first site's focus, the
    // nearest, not the second's, where the stage is. Quick captures are the
    // stream's size.
    put(
        client,
        "/smart_scan/workflow_name",
        json!("raster_workflow"),
    )
    .await;
    for (setting, value) in [
        ("x_count", json!(2)),
        ("y_count", json!(2)),
        ("autofocus_dz", json!(400)),
        ("capture_mode", json!("quick")),
        ("stack_min_images_to_test", json!(5)),
    ] {
        put(client, &format!("/raster_workflow/{setting}"), value).await;
    }
    put(client, "/camera/focal_tilt_x", json!(100.0)).await;
    let again = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "first"}),
    )
    .await;
    assert_eq!(again["output"], "first-2", "{again}");
    let raster = data.join("scans/first-2");
    let moved = moves(&raster);
    assert_eq!(
        moved[2][2],
        focus(&raster, moved[0][0], moved[0][1]),
        "{moved:?}"
    );
    let sidecar = images(&raster).into_iter().find(|f| f.ends_with(".json"));
    let sidecar = read_json(&raster.join("images").join(sidecar.expect("a sidecar")));
    assert_eq!(sidecar["width"], 820);
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn a_cancelled_scan_stops_and_goes_back_to_the_start() {
    let (harness, data) = start(true).await;
    let client = harness.client();
    put(
        client,
        "/smart_scan/workflow_name",
        json!("raster_workflow"),
    )
    .await;
    for setting in ["x_count", "y_count"] {
        put(client, &format!("/raster_workflow/{setting}"), json!(4)).await;
    }
    put(client, "/raster_workflow/autofocus_dz", json!(400)).await;
    let begin = client.get("/stage/position").await.json();
    let scan = invoke(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "long"}),
    )
    .await;
    // Only one scan at a time: here the global lock refuses another first.
    let second = run(client, "/smart_scan/sample_scan", json!({})).await;
    assert_eq!(second["status"], "error", "{second}");
    // Cancelled once it has an image.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let details = "/smart_scan/latest_scan_live_details";
    while client.get(details).await.json()["image_count"] == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "no image within a minute"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    // The gallery won't delete it while it runs (P35b).
    let deleted = client.delete("/gallery/scans/long").await;
    assert_eq!(deleted.status.as_u16(), 409, "{}", deleted.text());
    assert!(client.delete(&scan).await.status.is_success());
    let record = finish(client, &scan).await;
    assert_eq!(record["status"], "cancelled", "{record}");
    assert_eq!(client.get("/stage/position").await.json(), begin);
    let saved = read_json(&data.join("scans/long/scan.json"));
    assert_eq!(saved["result"], "cancelled");
    let count = saved["image_count"].as_u64().unwrap();
    assert!((1..16).contains(&count), "{count} images");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn a_scan_that_fails_goes_back_to_the_start() {
    let (harness, data) = start(true).await;
    let client = harness.client();
    // The second column is beyond the stage's travel.
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["limits"] = json!(true);
    simulation["travel"]["x"] = json!({"min": -1000, "max": 200});
    put(client, "/stage/simulation", simulation).await;
    let begin = client.get("/stage/position").await.json();
    let record = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "edge"}),
    )
    .await;
    assert_eq!(record["status"], "error", "{record}");
    assert_eq!(client.get("/stage/position").await.json(), begin);
    let saved = read_json(&data.join("scans/edge/scan.json"));
    assert!(
        saved["result"].as_str().unwrap().starts_with("error:"),
        "{saved}"
    );
    assert_eq!(saved["image_count"], 1);
    let log = std::fs::read_to_string(data.join("scans/edge/scan.log")).expect("read");
    assert!(log.contains("Going back to the start"), "{log}");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn a_scan_needs_a_ready_workflow_and_a_plain_name() {
    let (harness, data) = start(false).await;
    let client = harness.client();
    let refused = run(client, "/smart_scan/sample_scan", json!({})).await;
    assert!(
        refused["error"]
            .to_string()
            .contains("Snake scan isn't ready"),
        "{refused}"
    );
    put(client, "/smart_scan/workflow_name", json!("histo_workflow")).await;
    let unknown = run(client, "/smart_scan/sample_scan", json!({})).await;
    assert!(
        unknown["error"]
            .to_string()
            .contains("no workflow `histo_workflow`"),
        "{unknown}"
    );
    // Nothing was scanned, so nothing was made.
    assert!(!data.join("scans").exists());
    let calibrated = run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
    assert_eq!(calibrated["status"], "completed", "{calibrated}");
    put(client, "/smart_scan/workflow_name", json!("snake_workflow")).await;
    for name in ["../out", "a/b", ".hidden", "c:d"] {
        let refused = run(
            client,
            "/smart_scan/sample_scan",
            json!({"scan_name": name}),
        )
        .await;
        assert!(
            refused["error"].to_string().contains("a scan's name"),
            "{name}: {refused}"
        );
    }
    assert!(!data.join("out").exists());
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}
