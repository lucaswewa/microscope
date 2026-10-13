//! The gallery (P31, ADR-0026): its index is built from the data folder and
//! follows every write, by a Thing or from outside the server; captures are
//! downloaded as a ZIP, deleted with all their files, or deleted all at once
//! by card type; and nothing outside the captures folder can be reached.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::RgbImage;
use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::camera::SimulatedCamera;
use microscope_things::gallery::Gallery;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

fn data_folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("microscope-gallery-{name}-{}", std::process::id()))
}

/// A 64 × 48 JPEG in `data`'s captures, called `name`.
fn image(data: &Path, name: &str) -> PathBuf {
    let path = data.join("captures").join(name);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("created");
    RgbImage::from_fn(64, 48, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 5) as u8, 90])
    })
    .save(&path)
    .expect("saved");
    path
}

/// The sidecar of a capture taken on New Year's Day 2026, beside `image`.
fn sidecar(image: &Path) {
    let sidecar = json!({
        "format": 1,
        "image": image.file_name().expect("a name").to_str(),
        "acquired": "2026-01-01T00:00:00Z",
        "width": 64,
        "height": 48,
        "um_per_pixel": null,
        "stage": {
            "steps": {"x": 10, "y": -20, "z": 10},
            "um": {"x": 1.0, "y": -2.0, "z": 0.5},
            "um_per_step": {"x": 0.1, "y": 0.1, "z": 0.05}
        },
        "camera": {"objective": 40},
        "camera_stage_mapping": null,
        "simulation": null
    });
    std::fs::write(image.with_extension("json"), sidecar.to_string()).expect("written");
}

async fn start(data: &Path) -> Harness<Gallery> {
    Harness::builder("gallery", Gallery::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts")
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

async fn entries(client: &TestClient) -> Vec<Value> {
    let list = client.get("/gallery/list_data").await.json();
    list.as_array().expect("a list").clone()
}

fn names(entries: &[Value]) -> Vec<&str> {
    entries
        .iter()
        .map(|e| e["name"].as_str().expect("a name"))
        .collect()
}

fn size(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).expect("an image")
}

#[tokio::test]
async fn the_index_is_built_from_the_data_folder_and_follows_writes() {
    let data = data_folder("index");
    let old = image(&data, "capture-2026-01-01T00-00-00Z.jpeg");
    sidecar(&old);
    image(&data, "orphan.jpg");
    std::fs::write(data.join("captures/notes.txt"), "not a capture").expect("written");
    let harness = start(&data).await;
    let client = harness.client();
    assert_eq!(
        client.get("/gallery/card_types").await.json(),
        json!(["Capture"])
    );
    assert_eq!(client.get("/gallery/bulk_actions").await.json(), json!([]));

    // The newest first: the orphan was written just now.
    let listed = entries(client).await;
    assert_eq!(names(&listed), ["orphan", "capture-2026-01-01T00-00-00Z"]);
    let name = "capture-2026-01-01T00-00-00Z.jpeg";
    let path = format!("captures/{name}");
    assert_eq!(
        listed[1],
        json!({
            "name": "capture-2026-01-01T00-00-00Z",
            "path": path,
            "card_type": "Capture",
            "created": "2026-01-01T00:00:00.000Z",
            "thumbnail": format!("captures/thumbnails/{name}"),
            "viewer": {"type": "image", "path": path},
            "info": [
                {"label": "Size", "value": "64 × 48 px"},
                {"label": "Position", "value": "(1.0, -2.0, 0.5) µm"},
                {"label": "Objective", "value": "40×"}
            ],
            "download_buttons": [
                {"label": "Image", "source": "data", "path": path, "filename": name},
                {
                    "label": "Image and metadata",
                    "source": "endpoint",
                    "thing": "gallery",
                    "path": format!("captures/{name}/zip"),
                    "filename": "capture-2026-01-01T00-00-00Z.zip",
                    "secondary": true
                }
            ],
            "card_actions": [],
            "card_messages": [],
            "delete_endpoint": format!("gallery/captures/{name}")
        })
    );
    // Thumbnails are made for captures that lack them, never larger.
    assert_eq!(size(&data.join("captures/thumbnails").join(name)), (64, 48));
    assert_eq!(listed[0]["info"], json!([]));
    assert_eq!(listed[0]["card_messages"][0]["level"], "warning");

    // A capture is listed, newest first, with its thumbnail.
    let quick = json!({"capture_mode": "quick"});
    let captured = run(client, "/camera/capture", quick).await;
    assert_eq!(captured["status"], "completed", "{captured}");
    let listed = entries(client).await;
    assert_eq!(listed.len(), 3);
    let thumbnail = listed[0]["thumbnail"].as_str().expect("a thumbnail");
    assert_eq!(size(&data.join(thumbnail)), (320, 240));

    // Files written and removed outside the server: an image whose sidecar
    // comes later, and the orphan.
    let late = image(&data, "late.jpeg");
    std::fs::remove_file(data.join("captures/orphan.jpg")).expect("removed");
    let listed = entries(client).await;
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0]["name"], "late");
    assert_eq!(listed[0]["card_messages"][0]["level"], "warning");
    sidecar(&late);
    let listed = entries(client).await;
    let late = listed.iter().find(|e| e["name"] == "late").expect("listed");
    assert_eq!(late["card_messages"], json!([]));
    assert_eq!(late["created"], "2026-01-01T00:00:00.000Z");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn captures_are_downloaded_and_deleted_with_their_files() {
    let data = data_folder("delete");
    let a = image(&data, "a b.jpeg");
    sidecar(&a);
    sidecar(&image(&data, "b.jpeg"));
    std::fs::write(data.join("secret.jpeg"), "outside the captures").expect("written");
    let harness = start(&data).await;
    let client = harness.client();
    let listed = entries(client).await;
    let entry = listed.iter().find(|e| e["name"] == "a b").expect("listed");
    assert_eq!(entry["delete_endpoint"], "gallery/captures/a%20b.jpeg");

    // The image and its sidecar, as a ZIP.
    let zip = client.get("/gallery/captures/a%20b.jpeg/zip").await;
    assert_eq!(zip.header("content-type"), Some("application/zip"));
    let mut archive = zip::ZipArchive::new(Cursor::new(zip.body.to_vec())).expect("a ZIP");
    let mut files: Vec<_> = archive.file_names().map(str::to_owned).collect();
    files.sort();
    assert_eq!(files, ["a b.jpeg", "a b.json"]);
    let mut image = Vec::new();
    std::io::copy(&mut archive.by_name("a b.jpeg").expect("there"), &mut image).expect("read");
    assert_eq!(image, std::fs::read(&a).expect("read"));

    // Nothing outside the captures folder, however it's written.
    for name in [
        "..%2Fsecret.jpeg",
        "..%5Csecret.jpeg",
        "%2e%2e",
        "thumbnails%2Fb.jpeg",
    ] {
        let deleted = client.delete(&format!("/gallery/captures/{name}")).await;
        assert_eq!(deleted.status.as_u16(), 404, "{name}");
        let zip = client.get(&format!("/gallery/captures/{name}/zip")).await;
        assert_eq!(zip.status.as_u16(), 404, "{name}");
    }
    assert!(data.join("secret.jpeg").is_file());
    assert!(data.join("captures/thumbnails/b.jpeg").is_file());

    let endpoint = entry["delete_endpoint"].as_str().expect("an endpoint");
    assert_eq!(
        client.delete(&format!("/{endpoint}")).await.status.as_u16(),
        204
    );
    for gone in ["a b.jpeg", "a b.json", "thumbnails/a b.jpeg"] {
        assert!(!data.join("captures").join(gone).exists(), "{gone}");
    }
    assert_eq!(names(&entries(client).await), ["b"]);
    assert_eq!(
        client.delete(&format!("/{endpoint}")).await.status.as_u16(),
        404
    );

    // All of a card type at once.
    let unknown = run(
        client,
        "/gallery/delete_all_data",
        json!({"card_types": ["Scan"]}),
    )
    .await;
    assert!(
        unknown["error"].to_string().contains("no card type"),
        "{unknown}"
    );
    let all = run(
        client,
        "/gallery/delete_all_data",
        json!({"card_types": ["Capture"]}),
    )
    .await;
    assert_eq!(all["status"], "completed", "{all}");
    assert_eq!(entries(client).await, Vec::<Value>::new());
    assert!(!data.join("captures/b.json").exists());
    assert!(data.join("secret.jpeg").is_file());
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
