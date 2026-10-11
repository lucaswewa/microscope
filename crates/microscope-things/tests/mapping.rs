//! Camera–stage mapping (P23) against the simulator, whose camera can be
//! turned and mirrored: the matrix it finds is the true one, backlash is
//! estimated, sparse places are worked around, and moves in image
//! coordinates land where asked.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_core::mapping::Matrix2;
use microscope_core::registration::{Gray, Options, register};
use microscope_things::camera::{Orientation, SimulatedCamera};
use microscope_things::hardware::{CameraApi, Position, StageApi};
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::mapping::{CameraStageMapping, CameraStageMappingActions};
use microscope_things::stage::SimulatedStage;

/// µm per camera pixel at 40×, over µm per stage step.
const STEPS_PER_PIXEL: f64 = (4.48 / 40.0) / 0.1;

/// The true matrix: a field-of-view move d (pixels) needs a stage move that
/// undoes the camera's turn, then its mirroring, scaled to steps.
fn truth(orientation: Orientation) -> Matrix2 {
    let (sin, cos) = (-orientation.rotation_deg).to_radians().sin_cos();
    let mirror = if orientation.flipped { -1.0 } else { 1.0 };
    let k = STEPS_PER_PIXEL;
    [[mirror * cos * k, -mirror * sin * k], [sin * k, cos * k]]
}

/// A data folder of the test's own.
fn data_folder() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("microscope-mapping-{}-{n}", std::process::id()))
}

/// The mapping, with the camera turned to `orientation`, a quick stage with
/// backlash or none, and the stage at `at`.
async fn start(
    orientation: Orientation,
    backlash: bool,
    at: Position,
    data: &PathBuf,
) -> Harness<CameraStageMapping> {
    let harness = Harness::builder("camera_stage_mapping", CameraStageMapping::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    put(client, "/camera/orientation", json!(orientation)).await;
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["speed"] = json!({"x": 1e6, "y": 1e6, "z": 1e6});
    simulation["backlash"] = json!(backlash);
    put(client, "/stage/simulation", simulation).await;
    let stage = client.runtime().thing_ref::<SimulatedStage>("stage");
    StageApi::move_absolute(&stage.expect("configured"), at)
        .await
        .expect("moved");
    harness
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

fn assert_matrix_near(found: Matrix2, expected: Matrix2, share: f64) {
    for (i, j) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let error = (found[i][j] - expected[i][j]).abs();
        assert!(
            error <= share * STEPS_PER_PIXEL,
            "{found:?} against {expected:?}"
        );
    }
}

async fn calibrates(orientation: Orientation) {
    let data = data_folder();
    let harness = start(orientation, false, Position::default(), &data).await;
    let calibration = harness.thing().calibrate_xy().await.expect("calibrated");
    assert_matrix_near(
        calibration.matrix.expect("fitted"),
        truth(orientation),
        0.01,
    );
    assert_eq!(calibration.image_resolution, [820, 616]);
    // Each move shifts the image about a tenth of the frame's height.
    for m in &calibration.moves {
        assert!((m.image[0].hypot(m.image[1]) - 61.6).abs() < 2.0, "{m:?}");
    }
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn recovers_the_matrix_for_a_camera_square_to_the_stage() {
    calibrates(Orientation::default()).await;
}

#[tokio::test]
async fn recovers_the_matrix_for_a_turned_camera() {
    calibrates(Orientation {
        rotation_deg: 35.0,
        flipped: false,
    })
    .await;
}

#[tokio::test]
async fn recovers_the_matrix_for_a_turned_and_mirrored_camera() {
    calibrates(Orientation {
        rotation_deg: -120.0,
        flipped: true,
    })
    .await;
}

#[tokio::test]
async fn estimates_backlash_and_leaves_it_out_of_the_fit() {
    let data = data_folder();
    let harness = start(Orientation::default(), true, Position::default(), &data).await;
    let calibration = harness.thing().calibrate_xy().await.expect("calibrated");
    // The simulator's backlash is 80 steps.
    assert!(
        (calibration.backlash_steps - 80.0).abs() <= 10.0,
        "{}",
        calibration.backlash_steps
    );
    assert!(calibration.moves.iter().any(|m| !m.kept));
    assert_matrix_near(
        calibration.matrix.expect("fitted"),
        truth(Orientation::default()),
        0.01,
    );
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn works_around_a_sparse_place_on_the_sample() {
    // Here a registration while sizing y's step, and some moves, can't be
    // trusted: the step is sized from the last one that could, and the
    // moves are left out.
    let data = data_folder();
    let at = Position::new(-8800, -1200, 0);
    let harness = start(Orientation::default(), false, at, &data).await;
    let calibration = harness.thing().calibrate_xy().await.expect("calibrated");
    let untracked: Vec<_> = calibration
        .moves
        .iter()
        .filter(|m| m.confidence < 0.3)
        .collect();
    assert!(!untracked.is_empty() && untracked.iter().all(|m| !m.kept));
    assert_matrix_near(
        calibration.matrix.expect("fitted"),
        truth(Orientation::default()),
        0.01,
    );
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn moves_and_converts_in_image_coordinates_once_calibrated() {
    let data = data_folder();
    let orientation = Orientation {
        rotation_deg: 35.0,
        flipped: false,
    };
    let harness = start(orientation, false, Position::default(), &data).await;
    let (client, mapping) = (harness.client(), harness.thing());
    assert_eq!(
        client
            .get("/camera_stage_mapping/calibration_required")
            .await
            .json(),
        json!(true)
    );
    assert!(mapping.move_in_image_coordinates(10.0, 0.0).await.is_err());
    assert_eq!(
        client
            .get("/camera_stage_mapping/last_calibration")
            .await
            .json(),
        Value::Null
    );

    let calibration = mapping.calibrate_xy().await.expect("calibrated");
    let home = json!({"x": 0, "y": 0, "z": 0});
    assert_eq!(client.get("/stage/position").await.json(), home);
    assert_eq!(
        client
            .get("/camera_stage_mapping/calibration_required")
            .await
            .json(),
        json!(false)
    );
    let saved = client
        .get("/camera_stage_mapping/last_calibration")
        .await
        .json();
    assert_eq!(saved["timestamp"], json!(calibration.timestamp));
    assert_eq!(saved["moves"].as_array().expect("moves").len(), 40);

    let steps = mapping
        .convert_image_to_stage_coordinates(100.0, -40.0)
        .await
        .expect("converted");
    let back = mapping
        .convert_stage_to_image_coordinates(steps.x, steps.y)
        .await
        .expect("converted");
    assert!(
        (back.x - 100.0).abs() < 1.0 && (back.y + 40.0).abs() < 1.0,
        "{back:?}"
    );

    // What was 120 px right of and 60 px below the centre comes to the centre.
    let runtime = client.runtime();
    let camera = runtime
        .thing_ref::<SimulatedCamera>("camera")
        .expect("configured");
    let frame = async || {
        camera.grab_frame().await.expect("a frame");
        let frame = camera.grab_frame().await.expect("a frame");
        Gray::from_rgb8(
            frame.width() as usize,
            frame.height() as usize,
            frame.as_raw(),
        )
    };
    let before = frame().await;
    mapping
        .move_in_image_coordinates(120.0, 60.0)
        .await
        .expect("moved");
    let shift = register(&before, &frame().await, &Options::default());
    assert!((shift.dx + 120.0).hypot(shift.dy + 60.0) < 1.0, "{shift:?}");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn an_empty_or_too_sparse_slide_cant_be_calibrated() {
    // On an empty slide it can't size the step; here, the sample is so
    // sparse that it sizes it but can't track x's moves.
    for (density, at) in [
        (0.0, Position::default()),
        (250.0, Position::new(-8800, 7500, 0)),
    ] {
        let data = data_folder();
        let harness = start(Orientation::default(), false, at, &data).await;
        put(harness.client(), "/camera/blob_density", json!(density)).await;
        let error = harness
            .thing()
            .calibrate_xy()
            .await
            .expect_err("nothing to track");
        assert!(
            format!("{error:?}").contains("too little detail"),
            "{error:?}"
        );
        let position = harness.client().get("/stage/position").await.json();
        assert_eq!(position, json!(at), "back where it began");
        harness.stop().await;
        let _ = std::fs::remove_dir_all(&data);
    }
}
