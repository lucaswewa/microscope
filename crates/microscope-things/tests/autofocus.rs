//! Autofocus (P21) against the simulated camera and stage: it converges
//! from offsets around focus, with backlash on and off, loops back into
//! range, measures sharpness along moves, and can be cancelled.

use std::time::Duration;

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::autofocus::{Autofocus, AutofocusActions, SweepStart};
use microscope_things::camera::SimulatedCamera;
use microscope_things::hardware::{Position, StageApi};
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

/// The sweep the tests use: 400 steps, 20 µm at 0.05 µm a step.
const DZ: i64 = 400;
/// One test at a time: each runs a camera, and when they share the CPU their
/// frame rates fall, and with them the sweeps' sampling.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// How close to focus counts as focused: 2 µm, 40 steps. Frames come about
/// 50 steps apart in a sweep, so the sharpest is up to 25 steps from focus,
/// plus the sensor's noise.
const TOLERANCE_UM: f64 = 2.0;

async fn start(backlash: bool) -> Harness<Autofocus> {
    let harness = Harness::builder("autofocus", Autofocus::default())
        .thing("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let mut simulation = client.get("/stage/simulation").await.json();
    simulation["backlash"] = json!(backlash);
    put(client, "/stage/simulation", simulation).await;
    harness
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

/// How far the sample really is from focus, in µm.
fn defocus_um(stage: &ThingRef<SimulatedStage>) -> f64 {
    ThingRef::thing(stage).true_position_um()[2]
}

/// Offsets from focus within the sweep's reach, from a fixed sequence.
fn offsets(seed: u64) -> impl Iterator<Item = i64> {
    let mut state = seed;
    std::iter::repeat_with(move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((state >> 33) % 300) as i64 - 150
    })
}

async fn converges_from_random_offsets(backlash: bool, seed: u64) {
    let _turn = ONE_AT_A_TIME.lock().await;
    let harness = start(backlash).await;
    let (stage, autofocus) = (stage(&harness), harness.thing());
    for offset in offsets(seed).take(3) {
        stage
            .move_absolute(Position::new(0, 0, offset))
            .await
            .expect("moved");
        let curve = autofocus
            .fast_autofocus(DZ, SweepStart::Centre)
            .await
            .expect("focused");
        assert!(curve.points.len() >= 5, "{} frames", curve.points.len());
        let defocus = defocus_um(&stage);
        assert!(
            defocus.abs() <= TOLERANCE_UM,
            "from {offset}: {defocus} µm out"
        );
        // It ends where it said it would.
        assert_eq!(
            StageApi::position(&stage).await.expect("known").z,
            curve.focus_z.unwrap()
        );
    }
    harness.stop().await;
}

#[tokio::test]
async fn fast_autofocus_converges_without_backlash() {
    converges_from_random_offsets(false, 1).await;
}

#[tokio::test]
async fn fast_autofocus_converges_with_backlash() {
    converges_from_random_offsets(true, 2).await;
}

#[tokio::test]
async fn looping_autofocus_sweeps_again_when_focus_is_at_the_edge() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let harness = start(false).await;
    let (stage, autofocus) = (stage(&harness), harness.thing());
    // Focus is just past the bottom of a sweep centred here.
    stage
        .move_absolute(Position::new(0, 0, 220))
        .await
        .expect("moved");
    let curve = autofocus
        .looping_autofocus(DZ, SweepStart::Centre)
        .await
        .expect("focused");
    assert!(curve.sweeps >= 2, "{} sweeps", curve.sweeps);
    assert!(
        defocus_um(&stage).abs() <= TOLERANCE_UM,
        "{} µm out",
        defocus_um(&stage)
    );
    harness.stop().await;
}

#[tokio::test]
async fn sharpness_peaks_at_focus_along_measured_moves() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let harness = start(false).await;
    let curve = harness
        .thing()
        .z_move_and_measure_sharpness(vec![-200, 400], 0.0)
        .await
        .expect("measured");
    let (low, high) = curve
        .points
        .iter()
        .fold((f64::MAX, f64::MIN), |(low, high), point| {
            (low.min(point.z), high.max(point.z))
        });
    // Frames come about 50 steps apart, so the ends are within 50 of the moves' ends.
    assert!(low < -150.0 && high > 120.0, "z from {low} to {high}");
    let sharpest = curve
        .points
        .iter()
        .max_by_key(|point| point.sharpness)
        .expect("frames");
    assert!(sharpest.z.abs() <= 40.0, "sharpest at {}", sharpest.z);
    assert_eq!(curve.focus_z, None);
    harness.stop().await;
}

#[tokio::test]
async fn autofocus_can_be_cancelled_and_refuses_an_empty_range() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let harness = start(false).await;
    let client = harness.client();
    let response = client
        .post_json("/autofocus/looping_autofocus", Some(&json!({"dz": 4000})))
        .await;
    let invocation = format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    );
    while client.get("/stage/moving").await.json() != json!(true) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(client.delete(&invocation).await.status.is_success());
    while client.get(&invocation).await.json()["status"] != "cancelled" {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(client.get("/stage/moving").await.json(), json!(false));

    let refused = harness.thing().fast_autofocus(0, SweepStart::Base).await;
    assert!(refused.is_err());
    harness.stop().await;
}
