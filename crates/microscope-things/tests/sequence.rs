//! Sequences (P41) on the simulator: a timelapse captures at each time point
//! into its folder, with a record and a gallery card; points that can't run
//! in time are skipped; cancelling keeps the images already captured; and
//! only plain names, and one sequence at a time. The latest sequence's live
//! details follow each (P42).

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::Autofocus;
use microscope_things::camera::SimulatedCamera;
use microscope_things::gallery::Gallery;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::sequence::Sequence;
use microscope_things::stage::SimulatedStage;

async fn start() -> (Harness<Sequence>, PathBuf) {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data = std::env::temp_dir().join(format!("microscope-seq-{}-{n}", std::process::id()));
    let harness = Harness::builder("sequence", Sequence::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("autofocus", Autofocus::default())
        .thing("gallery", Gallery::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    (harness, data)
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

async fn invoke(client: &TestClient, action: &str, input: Value) -> String {
    let response = client.post_json(action, Some(&input)).await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    let id = response.json()["id"].as_str().expect("an id").to_owned();
    format!("/action_invocations/{id}")
}

async fn finish(client: &TestClient, invocation: &str) -> Value {
    loop {
        let record = client.get(invocation).await.json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let invocation = invoke(client, action, input).await;
    finish(client, &invocation).await
}

async fn details(client: &TestClient) -> Value {
    (client.get("/sequence/latest_sequence_live_details").await).json()
}

fn record(folder: &Path) -> Value {
    let bytes = std::fs::read(folder.join("sequence.json")).expect("read");
    serde_json::from_slice(&bytes).expect("JSON")
}

/// The files in `folder` but its record and thumbnail, sorted.
fn files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = (std::fs::read_dir(folder).expect("listed").flatten())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("image_"))
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_timelapse_captures_at_each_point_with_its_record_and_card() {
    let (harness, data) = start().await;
    let client = harness.client();
    // Its last point is at 2 s, before its end.
    for (setting, value) in [("dt", 1.0), ("duration", 2.5)] {
        put(client, &format!("/sequence/{setting}"), json!(value)).await;
    }
    let began = Instant::now();
    let done = run(client, "/sequence/run_sequence", json!({"name": "lapse"})).await;
    assert_eq!(done["status"], "completed", "{done}");
    assert_eq!(done["output"], "lapse");
    assert!(began.elapsed() >= Duration::from_secs(2));
    let folder = data.join("sequences/lapse");
    let images = ["image_00000", "image_00001", "image_00002"];
    let expected: Vec<String> = (images.iter())
        .flat_map(|image| [format!("{image}.jpeg"), format!("{image}.json")])
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(files(&folder), expected);
    // A second apart, by when each was taken.
    let acquired: Vec<chrono::DateTime<chrono::FixedOffset>> = (images.iter())
        .map(|image| {
            let sidecar = record_of(&folder.join(format!("{image}.json")));
            chrono::DateTime::parse_from_rfc3339(sidecar["acquired"].as_str().unwrap()).unwrap()
        })
        .collect();
    for pair in acquired.windows(2) {
        let gap = (pair[1] - pair[0]).num_milliseconds();
        assert!((700..1300).contains(&gap), "{gap} ms apart");
    }
    let saved = record(&folder);
    assert_eq!(
        saved["plan"],
        json!({"dt": 1.0, "duration": 2.5, "autofocus_enabled": false, "autofocus_dz": 2000})
    );
    assert_eq!(
        (saved["image_count"].as_u64(), saved["skipped"].as_u64()),
        (Some(3), Some(0))
    );
    assert_eq!(saved["result"], "success");
    let live = details(client).await;
    assert_eq!(
        (&live["name"], &live["points"], &live["image_count"]),
        (&json!("lapse"), &json!(3), &json!(3))
    );
    assert_eq!(
        (&live["skipped"], &live["result"]),
        (&json!(0), &json!("success"))
    );
    assert_eq!(live["last_point_seconds"], 2.0);
    assert_eq!(live["latest_image"], "sequences/lapse/image_00002.jpeg");
    assert!(live["duration_seconds"].as_f64().unwrap() >= 2.0, "{live}");

    // Its card, and its ZIP.
    let list = client.get("/gallery/list_data").await.json();
    let card = (list.as_array().expect("a list").iter())
        .find(|entry| entry["name"] == "lapse")
        .expect("a card")
        .clone();
    assert_eq!(card["card_type"], "Sequence");
    let paths: Vec<String> = images
        .iter()
        .map(|i| format!("sequences/lapse/{i}.jpeg"))
        .collect();
    assert_eq!(
        card["viewer"],
        json!({"type": "directory", "paths": paths, "start": 0})
    );
    assert_eq!(card["thumbnail"], "sequences/lapse/thumbnail.jpeg");
    assert_eq!(card["info"][0], json!({"label": "Images", "value": "3"}));
    assert_eq!(
        card["info"][1],
        json!({"label": "Interval", "value": "1 s"})
    );
    assert_eq!(
        card["download_buttons"][0]["input"],
        json!({"name": "lapse"})
    );
    assert_eq!(card["delete_endpoint"], "gallery/sequences/lapse");
    let zip = run(client, "/sequence/download_zip", json!({"name": "lapse"})).await;
    let blob = client
        .get(zip["output"]["href"].as_str().expect("a blob"))
        .await;
    let archive = zip::ZipArchive::new(Cursor::new(blob.body.to_vec())).expect("a ZIP");
    let mut zipped: Vec<&str> = archive.file_names().collect();
    zipped.sort_unstable();
    let mut wanted: Vec<&str> = expected.iter().map(String::as_str).collect();
    wanted.push("sequence.json");
    wanted.sort_unstable();
    assert_eq!(zipped, wanted);
    let deleted = client.delete("/gallery/sequences/lapse").await;
    assert_eq!(deleted.status.as_u16(), 204, "{}", deleted.text());
    assert!(!folder.exists());
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

fn record_of(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read")).expect("JSON")
}

#[tokio::test]
async fn points_that_cant_run_in_time_are_skipped() {
    let (harness, data) = start().await;
    let client = harness.client();
    // Each point's autofocus takes over 2 s; the points are half a second
    // apart, for 4 s: 9 of them. The first runs, then one about 2.5 s on,
    // and the points between and after are skipped.
    for (setting, value) in [
        ("dt", json!(0.5)),
        ("duration", json!(4.0)),
        ("autofocus_enabled", json!(true)),
        ("autofocus_dz", json!(400)),
    ] {
        put(client, &format!("/sequence/{setting}"), value).await;
    }
    let done = run(client, "/sequence/run_sequence", json!({"name": "slow"})).await;
    assert_eq!(done["status"], "completed", "{done}");
    let folder = data.join("sequences/slow");
    let images = files(&folder);
    assert_eq!(images.len(), 4, "{images:?}");
    assert_eq!(&images[..2], ["image_00000.jpeg", "image_00000.json"]);
    // Named for its time point, so the skipped points show.
    let later: u32 = images[2]["image_".len()..][..5].parse().expect("a number");
    assert!((4..9).contains(&later), "{images:?}");
    let saved = record(&folder);
    assert_eq!(
        (saved["image_count"].as_u64(), saved["skipped"].as_u64()),
        (Some(2), Some(7))
    );
    let live = details(client).await;
    assert_eq!(
        (&live["points"], &live["image_count"], &live["skipped"]),
        (&json!(9), &json!(2), &json!(7))
    );
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

#[tokio::test]
async fn cancelling_keeps_the_images_captured_and_one_runs_at_a_time() {
    let (harness, data) = start().await;
    let client = harness.client();
    put(client, "/sequence/dt", json!(0.5)).await;
    put(client, "/sequence/duration", json!(60.0)).await;
    let invocation = invoke(client, "/sequence/run_sequence", json!({"name": "long"})).await;
    let folder = data.join("sequences/long");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !folder.join("image_00001.jpeg").exists() {
        assert!(Instant::now() < deadline, "no second image in 30 s");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Its details while it runs.
    let live = details(client).await;
    assert_eq!(
        (&live["name"], &live["result"]),
        (&json!("long"), &Value::Null)
    );
    assert_eq!(
        (&live["points"], &live["last_point_seconds"]),
        (&json!(121), &json!(60.0))
    );
    assert!(live["image_count"].as_u64().unwrap() >= 1, "{live}");
    let latest = live["latest_image"].as_str().expect("an image");
    assert!(data.join(latest).is_file(), "{latest}");
    assert!(live["duration_seconds"].as_f64().unwrap() >= 0.5, "{live}");
    // Another is refused, and the gallery won't delete this one.
    let second = run(client, "/sequence/run_sequence", json!({})).await;
    assert_eq!(second["status"], "error", "{second}");
    let deleted = client.delete("/gallery/sequences/long").await;
    assert_eq!(deleted.status.as_u16(), 409, "{}", deleted.text());

    assert!(client.delete(&invocation).await.status.is_success());
    let record_ = finish(client, &invocation).await;
    assert_eq!(record_["status"], "cancelled", "{record_}");
    let saved = record(&folder);
    assert_eq!(saved["result"], "cancelled");
    let count = saved["image_count"].as_u64().unwrap();
    assert!(count >= 2, "{count} images");
    let live = details(client).await;
    assert_eq!(
        (&live["result"], &live["image_count"]),
        (&json!("cancelled"), &json!(count))
    );
    assert_eq!(
        files(&folder).len() as u64,
        2 * count,
        "each with its sidecar"
    );
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

#[tokio::test]
async fn only_plain_names() {
    let (harness, data) = start().await;
    let client = harness.client();
    assert_eq!(details(client).await, Value::Null);
    for name in ["../out", "a/b", ".hidden"] {
        let refused = run(client, "/sequence/run_sequence", json!({"name": name})).await;
        assert!(
            refused["error"].to_string().contains("a sequence's name"),
            "{name}: {refused}"
        );
        let zip = run(client, "/sequence/download_zip", json!({"name": name})).await;
        assert_eq!(zip["status"], "error", "{name}: {zip}");
    }
    assert!(!data.join("out").exists());
    // Refused before they began, they leave no details.
    assert_eq!(details(client).await, Value::Null);
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}
