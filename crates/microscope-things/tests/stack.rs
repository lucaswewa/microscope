//! Z-stacks (P36) on the simulator: smart stacks on a tilted sample keep the
//! sharpest planes, trying again where focus is out of reach; without focus
//! they save only if asked; basic stacks take their images about where the
//! stage is. Each saves in the data folder.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::{
    Autofocus, AutofocusActions, StackOrigin, StackResult, StackSettings,
};
use microscope_things::camera::{CaptureMode, SimulatedCamera};
use microscope_things::hardware::{Position, StageApi};
use microscope_things::illumination::{SimulatedIllumination, SimulatedIlluminationActions};
use microscope_things::stage::SimulatedStage;

/// One test at a time, as in the autofocus tests: they share the CPU.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The distance between images: 2.5 µm, at 0.05 µm a step.
const DZ: i64 = 50;

/// The simulator on its default sample, sparser than most tests use.
async fn start() -> (Harness<Autofocus>, PathBuf) {
    let data = std::env::temp_dir().join(format!("microscope-stack-{}", std::process::id()));
    let harness = Harness::builder("autofocus", Autofocus::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
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

fn stage(harness: &Harness<Autofocus>) -> ThingRef<SimulatedStage> {
    let runtime = harness.client().runtime();
    runtime
        .thing_ref::<SimulatedStage>("stage")
        .expect("configured")
}

fn settings(images_to_save: u32, min_images_to_test: u32) -> StackSettings {
    StackSettings {
        images_to_save,
        min_images_to_test,
        dz: DZ,
        capture_mode: CaptureMode::Quick,
    }
}

/// The z in an image's name, `img_<x>_<y>_<z>.jpeg`.
fn z_of(name: &str) -> i64 {
    let z = name.trim_end_matches(".jpeg").rsplit('_').next();
    z.expect("a z").parse().expect("a number")
}

/// How far from focus an image was, in µm, by its sidecar: the stage's
/// true z above a focal surface tilted in x.
fn defocus_um(folder: &Path, name: &str) -> f64 {
    let sidecar = std::fs::read(folder.join(name).with_extension("json")).expect("read");
    let sidecar: Value = serde_json::from_slice(&sidecar).expect("JSON");
    let simulation = &sidecar["simulation"];
    let [x, _, z] = [0, 1, 2].map(|i| simulation["true_position_um"][i].as_f64().unwrap());
    z + simulation["focal_tilt"][0].as_f64().unwrap() * x / 1000.0
}

#[tokio::test]
async fn smart_stacks_on_a_tilted_sample_keep_the_sharpest_planes() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let (harness, data) = start().await;
    put(harness.client(), "/camera/focal_tilt_x", json!(100.0)).await;
    let (stage, autofocus) = (stage(&harness), harness.thing());
    // At x = 0 focus is at z = 0, where the stack starts from. 0.4 mm the
    // other way it's 40 µm higher, beyond the first attempt's images, so the
    // stack autofocuses to reach it.
    for (x, tries) in [(0, 1), (-4000, 2)] {
        let folder = format!("stacks/{x}");
        stage
            .move_absolute(Position::new(x, 0, 0))
            .await
            .expect("moved");
        // Eight to save is raised to nine. Climbing, the first success
        // has fewer than four above the sharpest, so it takes more.
        let StackResult {
            success,
            sharpest_z,
            saved,
            attempts,
        } = autofocus
            .run_smart_stack(folder.clone(), settings(8, 9), false, 2000)
            .await
            .expect("stacked");
        let tried = format!("at {x}: {attempts} attempts, sharpest at {sharpest_z}, {saved:?}");
        assert!(success, "{tried}");
        assert_eq!(attempts, tries, "{tried}");
        let zs: Vec<i64> = saved.iter().map(|name| z_of(name)).collect();
        let around: Vec<i64> = (-4..=4).map(|k| sharpest_z + k * DZ).collect();
        assert_eq!(zs, around, "at {x}");
        // The middle is the plane nearest focus: within half a step of it.
        let folder = data.join(&folder);
        let defocus: Vec<f64> = saved.iter().map(|name| defocus_um(&folder, name)).collect();
        assert!(defocus[4].abs() <= 1.3, "at {x}: {defocus:?} µm out");
        assert!(
            saved
                .iter()
                .all(|name| name.starts_with(&format!("img_{x}_0_")))
        );
    }
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn without_focus_a_smart_stack_saves_only_if_asked() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let (harness, data) = start().await;
    // With the light off, every frame is as sharp as the others.
    let runtime = harness.client().runtime();
    let illumination = runtime.thing_ref::<SimulatedIllumination>("illumination");
    (illumination.expect("configured").set_led(false).await).expect("off");
    let autofocus = harness.thing();
    for (save_on_failure, count) in [(false, 0), (true, 1)] {
        let stacked = autofocus
            .run_smart_stack("dark".to_owned(), settings(1, 3), save_on_failure, 100)
            .await
            .expect("stacked");
        assert!(!stacked.success);
        // Autofocus finds no focus either, so it doesn't try again.
        assert_eq!(stacked.attempts, 1);
        assert_eq!(stacked.saved.len(), count, "{stacked:?}");
    }
    let files = std::fs::read_dir(data.join("dark"))
        .expect("listed")
        .count();
    assert_eq!(files, 2, "an image and its sidecar");
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}

#[tokio::test]
async fn basic_stacks_start_centre_or_end_where_the_stage_is() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let (harness, data) = start().await;
    let (stage, autofocus) = (stage(&harness), harness.thing());
    for (origin, offsets) in [
        (StackOrigin::Start, [0, 50, 100]),
        (StackOrigin::Centre, [-50, 0, 50]),
        (StackOrigin::End, [-100, -50, 0]),
    ] {
        stage
            .move_absolute(Position::new(0, 0, 1000))
            .await
            .expect("moved");
        let zs = autofocus
            .run_basic_stack("basic".to_owned(), settings(3, 9), origin)
            .await
            .expect("stacked");
        assert_eq!(zs, offsets.map(|dz| 1000 + dz), "{origin:?}");
        for z in zs {
            assert!(data.join(format!("basic/img_0_0_{z}.jpeg")).exists());
        }
    }
    // Only in the data folder.
    let outside = autofocus
        .run_basic_stack("../outside".to_owned(), settings(1, 9), StackOrigin::Start)
        .await;
    assert!(outside.is_err());
    assert!(!data.join("../outside").exists());
    harness.stop().await;
    std::fs::remove_dir_all(&data).expect("removed");
}
