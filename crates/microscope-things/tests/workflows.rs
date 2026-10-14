//! The histo and C-Chip workflows (P37) on the simulator, over a tissue
//! island: a picture of a disc of tissue on the empty slide. A histo scan
//! spirals out over the island and stops at its edge; a C-Chip scan visits
//! every site of its pattern.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::Autofocus;
use microscope_things::background::ChannelDeviationDetector;
use microscope_things::c_chip::CChipWorkflow;
use microscope_things::camera::SimulatedCamera;
use microscope_things::histo::HistoScanWorkflow;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::mapping::CameraStageMapping;
use microscope_things::smart_scan::SmartScan;
use microscope_things::stage::SimulatedStage;

/// The island's radius, in µm. A frame is about 92 × 69 µm.
const ISLAND_UM: f64 = 70.0;
/// The picture's scale, in µm per pixel.
const PICTURE_UM: f64 = 0.5;
/// The stage's scale in x and y, in µm per step.
const STEP_UM: f64 = 0.1;

/// The workflows on the simulator, its data in a new folder, the island as
/// its sample, and the mapping calibrated at the island's centre.
async fn start() -> (Harness<SmartScan>, PathBuf) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data = std::env::temp_dir().join(format!("microscope-p37-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&data).expect("created");
    island().save(data.join("island.png")).expect("saved");
    let harness = Harness::builder("smart_scan", SmartScan::default())
        .thing("histo_scan_workflow", HistoScanWorkflow::default())
        .thing("c_chip_workflow", CChipWorkflow::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("autofocus", Autofocus::default())
        .thing("camera_stage_mapping", CameraStageMapping::default())
        .thing(
            "bg_channel_deviations_luv",
            ChannelDeviationDetector::default(),
        )
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 500});
    put(client, "/stage/simulation", simulation).await;
    // Not stitched, so nothing is still writing as a test removes its data.
    put(client, "/smart_scan/stitch_automatically", json!(false)).await;
    for (setting, value) in [
        ("sample_type", json!("image")),
        ("sample_image", json!("island.png")),
        ("sample_image_scale", json!(PICTURE_UM)),
    ] {
        put(client, &format!("/camera/{setting}"), value).await;
    }
    let calibrated = run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
    assert_eq!(calibrated["status"], "completed", "{calibrated}");
    (harness, data)
}

/// A disc of pink tissue with purple nuclei, `ISLAND_UM` across, centred
/// on a picture of the empty slide.
fn island() -> RgbImage {
    let side = (3.0 * ISLAND_UM / PICTURE_UM) as u32;
    let centre = f64::from(side) / 2.0;
    let from_centre = |x: f64, y: f64| (x - centre).hypot(y - centre) * PICTURE_UM;
    let mut picture = RgbImage::from_fn(side, side, |x, y| {
        let inside = from_centre(f64::from(x), f64::from(y)) < ISLAND_UM;
        Rgb(if inside {
            [232, 150, 190]
        } else {
            [246, 244, 240]
        })
    });
    let mut state = 1u64;
    let mut random = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 11) as f64 / (1u64 << 53) as f64 * f64::from(side)
    };
    for _ in 0..1500 {
        let (cx, cy) = (random(), random());
        if from_centre(cx, cy) > ISLAND_UM - 3.0 {
            continue;
        }
        for (x, y, pixel) in picture.enumerate_pixels_mut() {
            if (f64::from(x) - cx).hypot(f64::from(y) - cy) < 5.0 {
                *pixel = Rgb([90, 60, 150]);
            }
        }
    }
    picture
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

async fn move_to(client: &TestClient, x: i64, y: i64) {
    let moved = run(
        client,
        "/stage/move_absolute",
        json!({"x": x, "y": y, "z": 0}),
    )
    .await;
    assert_eq!(moved["status"], "completed", "{moved}");
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read")).expect("JSON")
}

/// A scan's images, each as [x, y, z] from its name, sorted.
fn images(scan: &Path) -> Vec<[i64; 3]> {
    let mut sites: Vec<[i64; 3]> = std::fs::read_dir(scan.join("images"))
        .expect("listed")
        .filter_map(|entry| {
            let name = entry.expect("listed").file_name().into_string().ok()?;
            let stem = name.strip_prefix("img_")?.strip_suffix(".jpeg")?;
            let parts: Vec<i64> = stem
                .split('_')
                .map(|p| p.parse().expect("a number"))
                .collect();
            Some([parts[0], parts[1], parts[2]])
        })
        .collect();
    sites.sort_unstable();
    sites
}

#[tokio::test]
async fn a_histo_scan_spirals_over_a_tissue_island_and_stops_at_its_edge() {
    let (harness, data) = start().await;
    let client = harness.client();
    let workflow = "/histo_scan_workflow";
    // Not ready without a background, while it skips background.
    let ready = format!("{workflow}/ready");
    assert_eq!(client.get(&ready).await.json(), false);
    put(client, &format!("{workflow}/skip_background"), json!(false)).await;
    assert_eq!(client.get(&ready).await.json(), true);
    put(client, &format!("{workflow}/skip_background"), json!(true)).await;
    let refused = run(client, "/smart_scan/sample_scan", json!({})).await;
    assert!(
        refused["error"]
            .to_string()
            .contains("Histo scan isn't ready"),
        "{refused}"
    );
    // The background, set from the empty slide beyond the island.
    let beyond = (2.0 * ISLAND_UM / STEP_UM) as i64;
    move_to(client, beyond, 0).await;
    let set = run(client, &format!("{workflow}/set_background"), json!({})).await;
    assert_eq!(set["status"], "completed", "{set}");
    assert_eq!(client.get(&ready).await.json(), true);
    let empty = run(client, &format!("{workflow}/check_background"), json!({})).await;
    assert_eq!(empty["output"]["sample"], false, "{empty}");
    assert!(
        empty["output"]["message"]
            .as_str()
            .unwrap()
            .starts_with("only ")
    );
    // A scan won't start on background.
    let refused = run(client, "/smart_scan/sample_scan", json!({})).await;
    assert!(
        refused["error"]
            .to_string()
            .contains("move to the sample first"),
        "{refused}"
    );
    move_to(client, 0, 0).await;
    let tissue = run(client, &format!("{workflow}/check_background"), json!({})).await;
    assert_eq!(tissue["output"]["sample"], true, "{tissue}");

    // The default workflow.
    for (setting, value) in [
        ("autofocus_dz", json!(400)),
        ("capture_mode", json!("quick")),
        ("stack_min_images_to_test", json!(5)),
    ] {
        put(client, &format!("{workflow}/{setting}"), value).await;
    }
    let record = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "island"}),
    )
    .await;
    assert_eq!(record["status"], "completed", "{record}");
    let scan = data.join("scans/island");
    let log = std::fs::read_to_string(scan.join("scan.log")).expect("read");
    assert!(
        log.contains("The scanned area is surrounded by background"),
        "{log}"
    );
    let saved = read_json(&scan.join("scan.json"));
    assert_eq!(saved["workflow"], "histo_scan_workflow");
    assert_eq!(saved["settings"]["pattern"], "smart_spiral");
    // Images only where a frame meets the island; background around them
    // visited, but not imaged.
    let imaged = images(&scan);
    let visited = log.matches("Moving to").count();
    assert!(imaged.len() >= 5, "{imaged:?}");
    assert!(visited > imaged.len(), "{visited} sites, {imaged:?}");
    let reach = (ISLAND_UM + 46.0) / STEP_UM;
    for [x, y, _] in &imaged {
        let off = (*x as f64).hypot(*y as f64);
        assert!(
            off < reach,
            "an image {off} steps from the centre: {imaged:?}"
        );
    }
    assert_eq!(saved["image_count"], imaged.len());

    // Equal distances make the steps the same size; the range is as set.
    put(client, &format!("{workflow}/equal_distances"), json!(true)).await;
    put(client, &format!("{workflow}/max_range"), json!(2000)).await;
    let equal = json!({"scan_name": "equal"});
    let scan = invoke(client, "/smart_scan/sample_scan", equal).await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let details = "/smart_scan/latest_scan_live_details";
    while client.get(details).await.json()["scan_phase"] != "scanning" {
        assert!(
            std::time::Instant::now() < deadline,
            "not scanning in a minute"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(client.delete(&scan).await.status.is_success());
    assert_eq!(finish(client, &scan).await["status"], "cancelled");
    // The scan had no image, so its folder is gone; its details keep them.
    let settings = &client.get(details).await.json()["settings"];
    assert_eq!(
        (
            settings["equal_distances"].as_bool(),
            settings["max_range"].as_i64()
        ),
        (Some(true), Some(2000))
    );
    let [dx, dy] = ["dx", "dy"].map(|d| settings[d].as_i64().expect("a step").abs());
    assert_eq!(dx, dy, "{settings}");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn a_c_chip_scan_visits_every_site_with_a_stack_on_one_plane() {
    let (harness, data) = start().await;
    let client = harness.client();
    // What each does, for the Slide Scan tab (P40).
    let descriptions = client.get("/smart_scan/workflow_descriptions").await.json();
    let described = |name: &str| descriptions[name].as_str().unwrap_or_default().to_owned();
    assert!(described("c_chip_workflow").starts_with("For a C-Chip"));
    assert!(described("histo_scan_workflow").starts_with("For tissue sections"));
    put(
        client,
        "/smart_scan/workflow_name",
        json!("c_chip_workflow"),
    )
    .await;
    for (setting, value) in [
        ("autofocus_dz", json!(400)),
        ("capture_mode", json!("quick")),
        ("stack_images_to_save", json!(3)),
    ] {
        put(client, &format!("/c_chip_workflow/{setting}"), value).await;
    }
    let record = run(
        client,
        "/smart_scan/sample_scan",
        json!({"scan_name": "chip"}),
    )
    .await;
    assert_eq!(record["status"], "completed", "{record}");
    let scan = data.join("scans/chip");
    let saved = read_json(&scan.join("scan.json"));
    let settings = &saved["settings"];
    assert_eq!(settings["pattern"], "c_chip");
    let (dx, dy) = (
        settings["dx"].as_i64().unwrap(),
        settings["dy"].as_i64().unwrap(),
    );
    let dz = settings["stack_dz"].as_i64().unwrap();
    // Frames overlap by a tenth, through the mapping.
    let mapping = "/camera_stage_mapping";
    let matrix = (client.get(&format!("{mapping}/image_to_stage_displacement_matrix")))
        .await
        .json();
    let size = (client.get(&format!("{mapping}/image_resolution")))
        .await
        .json();
    let across = |i: usize| matrix[i][i].as_f64().unwrap() * size[i].as_f64().unwrap() * 0.9;
    assert_eq!([dx, dy], [0, 1].map(|i| across(i).round() as i64));
    // Three images at each of the 5 × 7 sites, on the same three planes.
    let imaged = images(&scan);
    assert_eq!(imaged.len(), 3 * 5 * 7, "{imaged:?}");
    assert_eq!(saved["image_count"], 3 * 5 * 7);
    let mut planes: Vec<i64> = imaged.iter().map(|[_, _, z]| *z).collect();
    planes.sort_unstable();
    planes.dedup();
    assert_eq!(planes.len(), 3, "{planes:?}");
    assert_eq!([planes[1] - planes[0], planes[2] - planes[1]], [dz, dz]);
    for i in 0..5 {
        for j in 0..7 {
            let (x, y) = (i * dx, j * dy);
            let here = imaged.iter().filter(|[ix, iy, _]| (*ix, *iy) == (x, y));
            assert_eq!(here.count(), 3, "at column {i}, row {j}");
        }
    }
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
