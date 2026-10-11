//! Captures to the data folder (P26, ADR-0025): a JPEG with a sidecar of how
//! it was taken, the camera–stage mapping included once it's calibrated;
//! quick captures at the stream's size; captures only downloaded; and names
//! that never collide.

use std::path::{Path, PathBuf};

use image::RgbImage;
use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::camera::SimulatedCamera;
use microscope_things::capture::{self, Sidecar};
use microscope_things::hardware::{Capture, CaptureMetadata};
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::mapping::CameraStageMapping;
use microscope_things::stage::SimulatedStage;

fn data_folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("microscope-capture-{name}-{}", std::process::id()))
}

/// A camera without noise, a quick stage, and a camera–stage mapping, with
/// their data in `data`.
async fn start(data: &Path) -> Harness<SimulatedCamera> {
    let harness = Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("camera_stage_mapping", CameraStageMapping::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let response = client.put_json("/camera/noise_level", &json!(0.0)).await;
    assert!(response.status.is_success());
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 1e6});
    let response = client.put_json("/stage/simulation", &simulation).await;
    assert!(response.status.is_success());
    harness
}

/// Runs an action over HTTP: its output.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let response = client.post_json(action, Some(&input)).await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let invocation = format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    );
    loop {
        let record = client.get(&invocation).await.json();
        match record["status"].as_str() {
            Some("pending" | "running") => {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await
            }
            Some("completed") => return record["output"].clone(),
            _ => panic!("{action}: {record}"),
        }
    }
}

/// The bytes a Blob output links to.
async fn download(client: &TestClient, blob: &Value) -> Vec<u8> {
    let href = blob["href"].as_str().expect("a link");
    let path = &href[href.find("/blob/").expect("a blob")..];
    client.get(path).await.body.to_vec()
}

/// The files in `data`'s `captures`, sorted.
fn captures(data: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(data.join("captures")) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
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

#[tokio::test]
async fn a_capture_is_saved_with_how_it_was_taken() {
    let data = data_folder("saved");
    let harness = start(&data).await;
    let client = harness.client();
    run(
        client,
        "/stage/move_absolute",
        json!({"x": 1000, "y": -500, "z": 20}),
    )
    .await;

    let blob = run(client, "/camera/capture", json!({})).await;
    let names = captures(&data);
    assert_eq!(names.len(), 2, "{names:?}");
    let (image, sidecar) = (&names[0], &names[1]);
    assert!(
        image.starts_with("capture-") && image.ends_with(".jpeg"),
        "{image}"
    );
    assert_eq!(*sidecar, image.replace(".jpeg", ".json"));
    let saved = std::fs::read(data.join("captures").join(image)).expect("read");
    assert_eq!(download(client, &blob).await, saved);

    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(data.join("captures").join(name)).expect("read"))
            .expect("JSON")
    };
    let record = read(sidecar);
    let _: Sidecar = serde_json::from_value(record.clone()).expect("a sidecar");
    assert_eq!(record["format"], json!(1));
    assert_eq!(record["image"], json!(image));
    assert_eq!(
        (record["width"].clone(), record["height"].clone()),
        (json!(1640), json!(1232))
    );
    assert_eq!(record["um_per_pixel"], json!(2.24 / 40.0));
    assert_eq!(
        record["stage"]["steps"],
        json!({"x": 1000, "y": -500, "z": 20})
    );
    let um = &record["stage"]["um"];
    for (axis, expected) in [("x", 100.0), ("y", -50.0), ("z", 1.0)] {
        assert!(
            (um[axis].as_f64().expect("known") - expected).abs() < 1e-9,
            "{um}"
        );
    }
    assert_eq!(record["camera"]["objective"], json!(40));
    assert_eq!(record["camera"]["exposure_time"], json!(10_000));
    assert_eq!(record["camera"]["analogue_gain"], json!(1.0));
    assert_eq!(record["camera_stage_mapping"], Value::Null);
    assert_eq!(record["simulation"]["sample_type"], json!("blobs"));
    let true_position = &record["simulation"]["true_position_um"];
    assert!(
        (true_position[0].as_f64().expect("x") - 100.0).abs() < 1e-6,
        "{true_position}"
    );

    // Once the mapping's calibrated, captures record it.
    run(client, "/camera_stage_mapping/calibrate_xy", json!({})).await;
    run(client, "/camera/capture", json!({})).await;
    let mapping = read(&captures(&data)[3])["camera_stage_mapping"].clone();
    let matrix = &mapping["image_to_stage_displacement_matrix"];
    assert!(
        (matrix[0][0].as_f64().expect("a") - 1.12).abs() < 0.02,
        "{mapping}"
    );
    assert_eq!(mapping["image_resolution"], json!([820, 616]));
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn a_quick_capture_not_kept_is_only_downloaded_at_the_streams_size() {
    let data = data_folder("quick");
    let harness = start(&data).await;
    let client = harness.client();
    let input = json!({"capture_mode": "quick", "retain_image": false});
    let blob = run(client, "/camera/capture", input).await;
    assert_eq!(captures(&data), Vec::<String>::new());
    let image = image::load_from_memory(&download(client, &blob).await).expect("a JPEG");
    assert_eq!((image.width(), image.height()), (820, 616));
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn captures_in_the_same_second_get_names_of_their_own() {
    let data = data_folder("names");
    let capture = Capture {
        image: RgbImage::new(4, 2),
        metadata: CaptureMetadata {
            acquired: "2026-10-11T07:35:12.345Z".to_owned(),
            width: 4,
            height: 2,
            ..CaptureMetadata::default()
        },
    };
    let saved: Vec<PathBuf> = (0..3)
        .map(|_| capture::save(&data, &capture, None).expect("saved"))
        .collect();
    let stem = "capture-2026-10-11T07-35-12Z";
    for (path, name) in saved.iter().zip([
        format!("{stem}.jpeg"),
        format!("{stem}-2.jpeg"),
        format!("{stem}-3.jpeg"),
    ]) {
        assert_eq!(*path, data.join("captures").join(&name));
        let sidecar: Sidecar =
            serde_json::from_slice(&std::fs::read(path.with_extension("json")).expect("read"))
                .expect("a sidecar");
        assert_eq!(sidecar.image, name);
        assert_eq!(sidecar.stage, None);
    }
    std::fs::remove_dir_all(&data).expect("removed");
}
