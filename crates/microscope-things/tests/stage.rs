//! The simulated stage (P16), through HTTP as the web app uses it and
//! through `StageApi` as other Things will.

use std::time::Duration;

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::{Harness, TestClient};

use microscope_things::hardware::{AxisScale, Position, StageApi};
use microscope_things::stage::{SimulatedStage, SimulatedStageActions};

async fn start() -> Harness<SimulatedStage> {
    Harness::builder("stage", SimulatedStage::default())
        .global_lock(true)
        .start()
        .await
        .expect("the harness starts")
}

/// Changes the simulation: `changes` over the defaults.
async fn simulate(client: &TestClient, changes: Value) {
    let mut simulation = client.get("/stage/simulation").await.json();
    for (key, value) in changes.as_object().expect("an object") {
        simulation[key] = value.clone();
    }
    let response = client.put_json("/stage/simulation", &simulation).await;
    assert!(response.status.is_success(), "{}", response.text());
}

/// Starts an action over HTTP: its invocation's path.
async fn post(client: &TestClient, action: &str, input: Value) -> String {
    let response = client
        .post_json(&format!("/stage/{action}"), Some(&input))
        .await;
    assert_eq!(response.status.as_u16(), 201, "{}", response.text());
    format!(
        "/action_invocations/{}",
        response.json()["id"].as_str().expect("an id")
    )
}

/// Waits for an invocation to end: its record.
async fn finished(client: &TestClient, invocation: &str) -> Value {
    for _ in 0..500 {
        let record = client.get(invocation).await.json();
        if !matches!(record["status"].as_str(), Some("pending" | "running")) {
            return record;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{invocation} didn't end");
}

/// Runs an action over HTTP: its record.
async fn run(client: &TestClient, action: &str, input: Value) -> Value {
    let invocation = post(client, action, input).await;
    finished(client, &invocation).await
}

async fn position(client: &TestClient) -> Value {
    client.get("/stage/position").await.json()
}

/// Waits until `moving` is `moving`.
async fn wait_for(client: &TestClient, moving: bool) {
    for _ in 0..200 {
        if client.get("/stage/moving").await.json() == json!(moving) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the stage's moving never became {moving}");
}

#[tokio::test]
async fn moves_wait_and_return_where_the_stage_stopped() {
    let harness = start().await;
    let client = harness.client();
    let record = run(
        client,
        "move_relative",
        json!({"x": 100, "y": -50, "z": 20}),
    )
    .await;
    assert_eq!(record["status"], "completed");
    assert_eq!(record["output"], json!({"x": 100, "y": -50, "z": 20}));
    assert_eq!(position(client).await, record["output"]);
    assert_eq!(client.get("/stage/moving").await.json(), json!(false));

    let record = run(client, "move_absolute", json!({"x": 10})).await;
    assert_eq!(record["output"], json!({"x": 10, "y": -50, "z": 20}));
    let record = run(client, "set_zero_position", json!({})).await;
    assert_eq!(record["output"], json!({"x": 0, "y": 0, "z": 0}));
    run(client, "move_relative", json!({"y": 30})).await;
    let record = run(client, "move_to_origin", json!({})).await;
    assert_eq!(record["output"], json!({"x": 0, "y": 0, "z": 0}));
    // The zero moved, not the stage: it is where it was before zeroing.
    let um = ThingRef::thing(harness.thing()).true_position_um();
    assert!(
        (um[0] - 1.0).abs() < 1e-9 && (um[1] + 5.0).abs() < 1e-9,
        "{um:?}"
    );
    harness.stop().await;
}

#[tokio::test]
async fn properties_describe_the_stage_and_calibration_checks_z() {
    let harness = start().await;
    let client = harness.client();
    assert_eq!(
        client.get("/stage/axis_names").await.json(),
        json!(["x", "y", "z"])
    );
    assert_eq!(
        client.get("/stage/um_per_step").await.json(),
        json!({"x": 0.1, "y": 0.1, "z": 0.05})
    );
    assert_eq!(client.get("/stage/can_calibrate").await.json(), json!(true));
    assert_eq!(
        client.get("/stage/calibration_required").await.json(),
        json!(true)
    );

    let record = run(client, "calibrate_z_direction", json!({"z_inverted": true})).await;
    assert_eq!(record["status"], "completed");
    assert_eq!(
        client.get("/stage/calibration_required").await.json(),
        json!(false)
    );
    assert_eq!(
        client.get("/stage/axis_inverted").await.json(),
        json!({"x": false, "y": false, "z": true})
    );
    // The readonly settings are changed only by actions.
    let response = client
        .put_json("/stage/z_direction_checked", &json!(false))
        .await;
    assert!(response.status.is_client_error());
    harness.stop().await;
}

#[tokio::test]
async fn an_inverted_axis_counts_the_other_way() {
    let harness = start().await;
    let client = harness.client();
    run(client, "move_relative", json!({"x": 100})).await;
    let record = run(client, "invert_axis_direction", json!({"axis": "x"})).await;
    assert_eq!(record["status"], "completed");
    assert_eq!(position(client).await, json!({"x": -100, "y": 0, "z": 0}));
    // +50 in the program's frame is -50 in the hardware's.
    run(client, "move_relative", json!({"x": 50})).await;
    assert_eq!(position(client).await, json!({"x": -50, "y": 0, "z": 0}));
    let um = ThingRef::thing(harness.thing()).true_position_um();
    assert!((um[0] - 5.0).abs() < 1e-9, "{um:?}");
    // Jogs too.
    run(client, "jog", json!({"x": -20})).await;
    wait_for(client, false).await;
    assert_eq!(position(client).await, json!({"x": -70, "y": 0, "z": 0}));
    let rejected = client
        .post_json("/stage/invert_axis_direction", Some(&json!({"axis": "w"})))
        .await;
    assert_eq!(rejected.status.as_u16(), 422);
    harness.stop().await;
}

#[tokio::test]
async fn cancelling_a_move_stops_the_stage() {
    let harness = start().await;
    let client = harness.client();
    // 100 seconds' travel.
    let invocation = post(client, "move_relative", json!({"x": 100_000})).await;
    wait_for(client, true).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(client.delete(&invocation).await.status.is_success());
    let record = finished(client, &invocation).await;
    assert_eq!(record["status"], "cancelled");
    assert_eq!(client.get("/stage/moving").await.json(), json!(false));
    let stopped = position(client).await;
    let x = stopped["x"].as_i64().expect("steps");
    assert!(x > 0 && x < 100_000, "{stopped}");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(position(client).await, stopped);
    harness.stop().await;
}

#[tokio::test]
async fn jogs_return_at_once_and_the_latest_wins() {
    let harness = start().await;
    let client = harness.client();
    let record = run(client, "jog", json!({"x": 100_000})).await;
    assert_eq!(record["status"], "completed");
    assert_eq!(client.get("/stage/moving").await.json(), json!(true));
    tokio::time::sleep(Duration::from_millis(50)).await;

    // The next jog replaces it: x stays where it got to.
    run(client, "jog", json!({"y": 100_000})).await;
    let x = position(client).await["x"].clone();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let now = position(client).await;
    assert_eq!(now["x"], x);
    assert!(now["y"].as_i64().expect("steps") > 0, "{now}");

    run(client, "jog", json!({"stop": true})).await;
    assert_eq!(client.get("/stage/moving").await.json(), json!(false));
    let stopped = position(client).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(position(client).await, stopped);
    harness.stop().await;
}

#[tokio::test]
async fn a_jog_or_a_halt_interrupts_a_move() {
    let harness = start().await;
    let client = harness.client();
    for interruption in [("jog", json!({"stop": true})), ("halt", json!({}))] {
        // The move holds the global lock; jogs and halts don't need it.
        let invocation = post(client, "move_relative", json!({"x": 100_000})).await;
        wait_for(client, true).await;
        let record = run(client, interruption.0, interruption.1).await;
        assert_eq!(record["status"], "completed", "{record}");
        let moved = finished(client, &invocation).await;
        assert_eq!(moved["status"], "completed");
        assert_eq!(moved["output"], position(client).await);
    }
    harness.stop().await;
}

#[tokio::test]
async fn the_end_of_travel_ends_a_move_with_an_error() {
    let harness = start().await;
    let client = harness.client();
    simulate(
        client,
        json!({"limits": true, "travel": {
            "x": {"min": -100, "max": 100},
            "y": {"min": -100, "max": 100},
            "z": {"min": -100, "max": 100},
        }}),
    )
    .await;
    let record = run(client, "move_relative", json!({"x": 500})).await;
    assert_eq!(record["status"], "error");
    assert!(
        record["error"]
            .to_string()
            .contains("the x axis reached the end of its travel"),
        "{record}"
    );
    assert_eq!(position(client).await, json!({"x": 100, "y": 0, "z": 0}));
    // Moving back in is fine.
    let record = run(client, "move_relative", json!({"x": -50})).await;
    assert_eq!(record["status"], "completed");
    harness.stop().await;
}

#[tokio::test]
async fn backlash_compensation_ends_every_move_the_same_way() {
    let harness = start().await;
    simulate(
        harness.client(),
        json!({"backlash": true, "speed": {"x": 1e6, "y": 1e6, "z": 1e6}}),
    )
    .await;
    let stage = harness.thing();
    let true_x = || ThingRef::thing(stage).true_position_um()[0];
    let mut ends = Vec::new();
    for compensate in [false, true] {
        for side in [1000, -1000] {
            SimulatedStageActions::move_absolute(stage, Some(side), None, None, false)
                .await
                .expect("moved");
            let at = SimulatedStageActions::move_absolute(stage, Some(0), None, None, compensate)
                .await
                .expect("moved");
            assert_eq!(at, Position::new(0, 0, 0));
            ends.push(true_x());
        }
    }
    // Without compensation the play shows (80 steps of 0.1 µm); with it, it doesn't.
    assert!((ends[0] - ends[1] - 8.0).abs() < 1e-9, "{ends:?}");
    assert!((ends[2] - ends[3]).abs() < 1e-9, "{ends:?}");

    harness.stop().await;
}

#[tokio::test]
async fn settings_survive_a_restart() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_nanos();
    let folder =
        std::env::temp_dir().join(format!("microscope-stage-{}-{nanos}", std::process::id()));
    let harness = Harness::builder("stage", SimulatedStage::default())
        .global_lock(true)
        .settings_folder(&folder)
        .start()
        .await
        .expect("the harness starts");
    let client = harness.client();
    let backlash = json!({"x": -100, "y": 0, "z": 300});
    let response = client.put_json("/stage/backlash_steps", &backlash).await;
    assert!(response.status.is_success());
    run(client, "invert_axis_direction", json!({"axis": "y"})).await;
    run(
        client,
        "calibrate_z_direction",
        json!({"z_inverted": false}),
    )
    .await;
    simulate(client, json!({"backlash": true})).await;
    harness.stop().await;

    let harness = Harness::builder("stage", SimulatedStage::default())
        .global_lock(true)
        .settings_folder(&folder)
        .start()
        .await
        .expect("the harness restarts");
    let client = harness.client();
    assert_eq!(client.get("/stage/backlash_steps").await.json(), backlash);
    assert_eq!(
        client.get("/stage/axis_inverted").await.json(),
        json!({"x": false, "y": true, "z": false})
    );
    assert_eq!(
        client.get("/stage/calibration_required").await.json(),
        json!(false)
    );
    assert_eq!(
        client.get("/stage/simulation").await.json()["backlash"],
        json!(true)
    );
    harness.stop().await;
    std::fs::remove_dir_all(&folder).expect("removed");
}

#[tokio::test]
async fn arrived_is_emitted_when_a_jog_ends() {
    let harness = start().await;
    let client = harness.client();
    let mut arrivals = client.events("/stage/arrived").await;
    // Nothing waits for a jog: the tracking task notices it end.
    run(client, "jog", json!({"x": 30, "z": -10})).await;
    let arrived = arrivals.next_within(Duration::from_secs(2)).await;
    assert_eq!(arrived, Some(json!({"x": 30, "y": 0, "z": -10})));
    assert_eq!(client.get("/stage/moving").await.json(), json!(false));
    harness.stop().await;
}

#[tokio::test]
async fn other_things_use_it_through_stage_api() {
    let harness = start().await;
    let stage: &dyn StageApi = harness.thing();
    assert_eq!(
        stage.axis_scale(),
        AxisScale {
            x: Some(0.1),
            y: Some(0.1),
            z: Some(0.05)
        }
    );
    let moved = stage
        .move_relative(Position::new(20, 10, -5))
        .await
        .expect("moved");
    assert_eq!(moved, Position::new(20, 10, -5));
    let back = stage
        .move_absolute(Position::new(0, 10, 0))
        .await
        .expect("moved");
    assert_eq!(back, Position::new(0, 10, 0));
    assert_eq!(stage.position().await.expect("known"), back);
    stage.stop().await.expect("stopped");
    harness.stop().await;
}
