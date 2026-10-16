//! How accurately autofocus finds focus (P54): from many offsets around
//! focus, on the simulator, which knows where focus is, tilt and waviness
//! included. Each method's error is how far the sample is from focus once
//! it's done: where fast and looping autofocus leave the stage, and where
//! the image a smart stack keeps was taken.
//!
//! It's slow, so it's ignored unless asked for, in a release build:
//!
//! ```text
//! cargo test --release -p microscope-things --test autofocus_bench -- --ignored --nocapture
//! ```
//!
//! For each method and sample it prints how many runs found nothing to
//! focus on, the others' error's mean, 95th percentile and worst, in µm,
//! and the mean time a run takes. It also prints each run more than 2 µm
//! out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::{Harness, TestClient};

use microscope_sim::render::FocalSurface;
use microscope_things::autofocus::{Autofocus, AutofocusActions, StackSettings, SweepStart};
use microscope_things::camera::SimulatedCamera;
use microscope_things::hardware::{CaptureMode, Position, StageApi};
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

/// Fast and looping autofocus's sweep: 1,000 steps, 50 µm, as a scan's.
const DZ: i64 = 1000;
/// How far from focus they start: up to 300 steps, 15 µm, either way.
const SWEEP_OFFSET: i64 = 300;
/// How far from focus a smart stack starts: up to 100 steps, 5 µm, as a
/// scan's site does near the focus of the one before.
const STACK_OFFSET: i64 = 100;
/// Runs for each method and sample.
const RUNS: usize = 20;
/// One method at a time: they share the CPU with the camera.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A sample, as the camera's settings make it.
struct Sample {
    name: &'static str,
    blob_density: f64,
    /// µm of height per mm in x and y.
    tilt: [f64; 2],
    waviness_um: f64,
}

const SAMPLES: [Sample; 3] = [
    Sample {
        name: "default",
        blob_density: 1500.0,
        tilt: [0.0, 0.0],
        waviness_um: 0.0,
    },
    Sample {
        name: "sparse",
        blob_density: 150.0,
        tilt: [0.0, 0.0],
        waviness_um: 0.0,
    },
    Sample {
        name: "tilted",
        blob_density: 1500.0,
        tilt: [5.0, 3.0],
        waviness_um: 1.0,
    },
];

#[derive(Clone, Copy)]
enum Method {
    Fast,
    Looping,
    SmartStack,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Method::Fast => "fast",
            Method::Looping => "looping",
            Method::SmartStack => "smart stack",
        }
    }
}

async fn start(data: &Path) -> Harness<Autofocus> {
    let harness = Harness::builder("autofocus", Autofocus::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .application_config(json!({"data_folder": data}))
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["backlash"] = json!(true);
    put(client, "/stage/simulation", simulation).await;
    harness
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

/// Offsets from focus, up to `reach` either way, from a fixed sequence.
fn offsets(reach: i64) -> impl Iterator<Item = i64> {
    let mut state = 54_u64;
    std::iter::repeat_with(move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((state >> 33) % (2 * reach as u64 + 1)) as i64 - reach
    })
}

/// How far a point at `true_um` on the stage is from focus, in µm.
fn defocus_um(surface: &FocalSurface, [x, y, z]: [f64; 3]) -> f64 {
    z + surface.height_um(x, y)
}

/// The error's mean, 95th percentile and worst, in µm, from its sizes.
fn summary(errors: &mut [f64]) -> [f64; 3] {
    errors.sort_by(f64::total_cmp);
    let mean = errors.iter().sum::<f64>() / errors.len() as f64;
    let p95 = errors[((errors.len() as f64 * 0.95).ceil() as usize).min(errors.len()) - 1];
    [mean, p95, errors[errors.len() - 1]]
}

async fn bench(method: Method) {
    let _turn = ONE_AT_A_TIME.lock().await;
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let data: PathBuf =
        std::env::temp_dir().join(format!("microscope-af-bench-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&data).expect("created");
    let harness = start(&data).await;
    let client = harness.client();
    let runtime = client.runtime();
    let stage = runtime
        .thing_ref::<SimulatedStage>("stage")
        .expect("configured");
    let autofocus = harness.thing();
    for sample in &SAMPLES {
        let camera = |setting: &str| format!("/camera/{setting}");
        put(client, &camera("blob_density"), json!(sample.blob_density)).await;
        put(client, &camera("focal_tilt_x"), json!(sample.tilt[0])).await;
        put(client, &camera("focal_tilt_y"), json!(sample.tilt[1])).await;
        put(client, &camera("focal_waviness"), json!(sample.waviness_um)).await;
        let surface = FocalSurface {
            tilt_um_per_mm: sample.tilt,
            waviness_um: sample.waviness_um,
        };
        let reach = match method {
            Method::SmartStack => STACK_OFFSET,
            _ => SWEEP_OFFSET,
        };
        let (mut errors, mut missed, mut seconds) = (Vec::new(), 0, 0.0);
        for (run, offset) in offsets(reach).take(RUNS).enumerate() {
            // Sites 200 µm apart, each with its own field and, if tilted,
            // its own focus.
            let (x, y) = ((run % 4) as i64 * 2000, (run / 4) as i64 * 2000);
            let um_per_step_z = 0.05;
            let focus =
                (-surface.height_um(x as f64 * 0.1, y as f64 * 0.1) / um_per_step_z).round();
            let at = Position::new(x, y, focus as i64 + offset);
            stage.move_absolute(at).await.expect("moved");
            let began = Instant::now();
            // How far out it left the sample, or None if it found nothing
            // to focus on.
            let error = match method {
                Method::Fast | Method::Looping => {
                    let focused = match method {
                        Method::Fast => (autofocus.fast_autofocus(DZ, SweepStart::Centre))
                            .await
                            .expect("it ran")
                            .focus_z
                            .is_some(),
                        _ => (autofocus.looping_autofocus(DZ, SweepStart::Centre))
                            .await
                            .is_ok(),
                    };
                    let true_um = ThingRef::thing(&stage).true_position_um();
                    focused.then(|| defocus_um(&surface, true_um))
                }
                Method::SmartStack => {
                    let folder = format!("bench/{}-{run}", sample.name);
                    let stack = StackSettings {
                        capture_mode: CaptureMode::Quick,
                        ..StackSettings::default()
                    };
                    let stacked = (autofocus.run_smart_stack(folder.clone(), stack, true, DZ))
                        .await
                        .expect("it ran");
                    let kept = data.join(&folder).join(&stacked.saved[0]);
                    let sidecar: Value = serde_json::from_slice(
                        &std::fs::read(kept.with_extension("json")).expect("its sidecar"),
                    )
                    .expect("JSON");
                    let taken = &sidecar["simulation"]["true_position_um"];
                    let taken = [0, 1, 2].map(|i| taken[i].as_f64().expect("a number"));
                    stacked.success.then(|| defocus_um(&surface, taken))
                }
            };
            seconds += began.elapsed().as_secs_f64();
            match error {
                Some(error) => {
                    if error.abs() > 2.0 {
                        println!("  run {run} at ({x}, {y}), {offset} from focus: {error:+.2} µm");
                    }
                    errors.push(error.abs());
                }
                None => missed += 1,
            }
        }
        let errors = if errors.is_empty() {
            "— | — | —".to_owned()
        } else {
            let [mean, p95, worst] = summary(&mut errors);
            format!("{mean:.2} | {p95:.2} | {worst:.2}")
        };
        println!(
            "| {} | {} | {RUNS} | {missed} | {errors} | {:.1} |",
            method.name(),
            sample.name,
            seconds / RUNS as f64
        );
    }
    harness.stop().await;
    let _ = std::fs::remove_dir_all(&data);
}

#[tokio::test]
#[ignore = "a slow benchmark, run on demand"]
async fn fast_autofocus_accuracy() {
    bench(Method::Fast).await;
}

#[tokio::test]
#[ignore = "a slow benchmark, run on demand"]
async fn looping_autofocus_accuracy() {
    bench(Method::Looping).await;
}

#[tokio::test]
#[ignore = "a slow benchmark, run on demand"]
async fn smart_stack_accuracy() {
    bench(Method::SmartStack).await;
}
