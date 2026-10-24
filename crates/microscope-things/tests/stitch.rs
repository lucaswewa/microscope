//! Stitching scans on request (P39): a scan's images, one a site, become a
//! deep-zoom mosaic in its `stitched/` folder, which its gallery card then
//! shows and offers to download, with where each image went (P53).
//! `stitch_all_scans` stitches those that aren't. These place the images
//! where the stage put them: refining by correlation (P13) is tested on a
//! drifting stage's scan, in `smart_scan.rs`.

use std::path::{Path, PathBuf};

use image::{GenericImageView, Rgb, RgbImage};
use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::camera::SimulatedCamera;
use microscope_things::gallery::Gallery;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::smart_scan::SmartScan;
use microscope_things::stage::SimulatedStage;

/// The tiles' size, and how far apart they're cut: a 3 × 3 grid.
const TILE: [u32; 2] = [200, 150];
const STEP: [u32; 2] = [100, 75];

fn picture() -> RgbImage {
    RgbImage::from_fn(400, 300, |x, y| {
        let wave = 128.0 + 90.0 * (f64::from(x) / 13.0).sin() * (f64::from(y) / 11.0).cos();
        Rgb([(x * 255 / 400) as u8, (y * 255 / 300) as u8, wave as u8])
    })
}

/// A scan's folder as a scan leaves it: tiles cut from `picture`, each with
/// its sidecar, where the stage steps are the picture's pixels, taken in a
/// snake: along the first row, back along the second, and along the third.
/// The middle site has a stack of three, the middle one right.
fn scan(data: &Path, name: &str) -> PathBuf {
    let images = data.join("scans").join(name).join("images");
    std::fs::create_dir_all(&images).expect("created");
    let picture = picture();
    for j in 0..3 {
        for i in 0..3 {
            let (x, y) = (i * STEP[0], j * STEP[1]);
            let tile = picture.view(x, y, TILE[0], TILE[1]).to_image();
            let middle = (i, j) == (1, 1);
            let black = RgbImage::new(TILE[0], TILE[1]);
            let white = RgbImage::from_pixel(TILE[0], TILE[1], Rgb([255; 3]));
            let stack: Vec<(i64, &RgbImage)> = if middle {
                vec![(0, &black), (50, &tile), (100, &white)]
            } else {
                vec![(50, &tile)]
            };
            let taken = j * 3 + if j == 1 { 2 - i } else { i };
            for (z, image) in stack {
                let acquired = format!("2026-01-01T00:00:{:02}.{z:03}Z", taken);
                save(&images, [i64::from(x), i64::from(y), z], image, &acquired);
            }
        }
    }
    data.join("scans").join(name)
}

fn save(images: &Path, [x, y, z]: [i64; 3], image: &RgbImage, acquired: &str) {
    let name = format!("img_{x}_{y}_{z}.jpeg");
    image.save(images.join(&name)).expect("saved");
    let sidecar = json!({
        "format": 1,
        "image": name,
        "acquired": acquired,
        "width": TILE[0],
        "height": TILE[1],
        "um_per_pixel": null,
        "stage": {
            "steps": {"x": x, "y": y, "z": z},
            "um": {"x": null, "y": null, "z": null},
            "um_per_step": {"x": null, "y": null, "z": null}
        },
        "camera": {},
        "camera_stage_mapping": {
            "image_to_stage_displacement_matrix": [[1.0, 0.0], [0.0, 1.0]],
            "image_resolution": TILE
        },
        "simulation": null
    });
    let sidecar_path = images.join(name).with_extension("json");
    std::fs::write(sidecar_path, sidecar.to_string()).expect("written");
}

async fn start(data: &Path) -> Harness<Gallery> {
    let harness = Harness::builder("gallery", Gallery::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("smart_scan", SmartScan::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    // A smooth picture, through JPEG, has little but the JPEG's blocks for
    // correlation to go by.
    let refine = json!(false);
    let response = (harness.client())
        .put_json("/smart_scan/refine_with_correlation", &refine)
        .await;
    assert!(response.status.is_success(), "{}", response.text());
    harness
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

async fn entry(client: &TestClient, name: &str) -> Value {
    let list = client.get("/gallery/list_data").await.json();
    let entries = list.as_array().expect("a list");
    (entries.iter().find(|e| e["name"] == name).expect("listed")).clone()
}

/// The names of the files in the ZIP archive the action `zip` returns.
async fn zipped(client: &TestClient, zip: &Value) -> Vec<String> {
    let blob = &zip["output"]["href"];
    let archive = client
        .get(blob.as_str().expect("a blob"))
        .await
        .body
        .to_vec();
    let archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).expect("a ZIP");
    let mut names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_scan_is_stitched_into_its_mosaic_which_its_card_then_shows() {
    let data = std::env::temp_dir().join(format!("microscope-stitch-{}", std::process::id()));
    let folder = scan(&data, "grid");
    let harness = start(&data).await;
    let client = harness.client();
    let card = entry(client, "grid").await;
    assert_eq!(card["viewer"]["type"], "directory");
    assert_eq!(card["card_actions"][0]["action"], "stitch_scan");

    let stitched = run(
        client,
        "/smart_scan/stitch_scan",
        json!({"scan_name": "grid"}),
    )
    .await;
    assert_eq!(stitched["status"], "completed", "{stitched}");
    assert_eq!(
        stitched["output"],
        json!({"size": [400, 300], "levels": 10, "images": 9, "correlation": null})
    );
    // The middle of the middle site's stack, so the picture again.
    let overview = image::open(folder.join("stitched/overview.jpeg")).expect("an overview");
    let pairs = overview
        .to_rgb8()
        .into_raw()
        .into_iter()
        .zip(picture().into_raw());
    let worst = pairs.map(|(a, b)| a.abs_diff(b)).max().unwrap_or(0);
    assert!(worst < 40, "{worst}");
    assert!(!data.join("scans/grid/stitched.partial").exists());
    // Where each image went, numbered as they were taken.
    let tiles: Value = serde_json::from_slice(
        &std::fs::read(folder.join("stitched/tiles.json")).expect("the tiles' record"),
    )
    .expect("JSON");
    assert_eq!(tiles["size"], json!([400, 300]));
    let snake = [
        [0, 0],
        [1, 0],
        [2, 0],
        [2, 1],
        [1, 1],
        [0, 1],
        [0, 2],
        [1, 2],
        [2, 2],
    ];
    let placed: Vec<Value> = (snake.iter().zip(1..))
        .map(|([i, j], number)| {
            let (x, y) = (i * STEP[0], j * STEP[1]);
            // The middle site's is the middle of its stack, at 50 too.
            json!({
                "image": format!("img_{x}_{y}_50.jpeg"),
                "number": number,
                "x": x,
                "y": y,
                "width": TILE[0],
                "height": TILE[1],
                "stage_x": x,
                "stage_y": y,
            })
        })
        .collect();
    assert_eq!(tiles["tiles"], json!(placed));

    // The card shows the mosaic, and offers it to download.
    let card = entry(client, "grid").await;
    assert_eq!(
        card["viewer"],
        json!({
            "type": "dzi",
            "path": "scans/grid/stitched/mosaic.dzi",
            "tiles": "scans/grid/stitched/tiles.json"
        })
    );
    assert_eq!(card["thumbnail"], "scans/grid/stitched/thumbnail.jpeg");
    assert_eq!(card["card_actions"], json!([]));
    let labels: Vec<&str> = (card["download_buttons"].as_array().expect("buttons").iter())
        .map(|button| button["label"].as_str().expect("a label"))
        .collect();
    assert_eq!(
        labels,
        [
            "Images (ZIP)",
            "Record",
            "Mosaic (JPEG)",
            "Mosaic tiles (ZIP)"
        ]
    );
    let overview = &card["download_buttons"][2]["path"];
    assert_eq!(overview, "scans/grid/stitched/overview.jpeg");
    assert!(data.join(overview.as_str().expect("a path")).is_file());
    let zip = run(
        client,
        "/smart_scan/download_stitched_zip",
        json!({"scan_name": "grid"}),
    )
    .await;
    let names = zipped(client, &zip).await;
    for name in [
        "mosaic.dzi",
        "mosaic_files/9/0_0.jpeg",
        "overview.jpeg",
        "thumbnail.jpeg",
    ] {
        assert!(names.contains(&name.to_owned()), "{name}: {names:?}");
    }
    assert!(names.len() > 10, "{names:?}");

    // Stitched before where each image went was recorded (P53b), its card
    // offers to stitch it again, which replaces it.
    std::fs::remove_file(folder.join("stitched/tiles.json")).expect("removed");
    let card = entry(client, "grid").await;
    assert_eq!(card["viewer"]["tiles"], Value::Null);
    assert_eq!(card["card_actions"][0]["label"], "Stitch again");
    let again = run(
        client,
        "/smart_scan/stitch_scan",
        json!({"scan_name": "grid"}),
    )
    .await;
    assert_eq!(again["status"], "completed", "{again}");
    let card = entry(client, "grid").await;
    assert_eq!(card["viewer"]["tiles"], "scans/grid/stitched/tiles.json");
    assert_eq!(card["card_actions"], json!([]));

    // Only scans in `scans/`, and only stitched ones' mosaics.
    for name in ["../scans/grid", "missing", ""] {
        let refused = run(
            client,
            "/smart_scan/stitch_scan",
            json!({"scan_name": name}),
        )
        .await;
        assert_eq!(refused["status"], "error", "{name}: {refused}");
    }
    scan(&data, "later");
    let unstitched = run(
        client,
        "/smart_scan/download_stitched_zip",
        json!({"scan_name": "later"}),
    )
    .await;
    assert!(
        unstitched["error"].to_string().contains("isn't stitched"),
        "{unstitched}"
    );
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn stitching_all_scans_stitches_those_that_arent() {
    let data = std::env::temp_dir().join(format!("microscope-stitch-all-{}", std::process::id()));
    scan(&data, "one");
    scan(&data, "two");
    // Without sidecars, there's nothing to place.
    let bare = data.join("scans/bare/images");
    std::fs::create_dir_all(&bare).expect("created");
    picture().save(bare.join("img_0_0_0.jpeg")).expect("saved");
    let harness = start(&data).await;
    let client = harness.client();
    let first = run(
        client,
        "/smart_scan/stitch_scan",
        json!({"scan_name": "one"}),
    )
    .await;
    assert_eq!(first["status"], "completed", "{first}");
    let modified = |name: &str| {
        let dzi = data.join(format!("scans/{name}/stitched/mosaic.dzi"));
        std::fs::metadata(dzi).and_then(|m| m.modified()).ok()
    };
    let before = modified("one");
    let all = run(client, "/smart_scan/stitch_all_scans", json!({})).await;
    assert_eq!(all["output"], json!(["two"]), "{all}");
    assert!(modified("two").is_some());
    assert_eq!(modified("one"), before, "already stitched");
    assert!(!data.join("scans/bare/stitched").exists());
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
