//! Background detection and camera calibration (P27): both detectors tell
//! tissue from the empty slide with vignetting and uneven lighting on; the
//! camera's detector setting, its errors, and its calibration panels.

use serde_json::{Value, json};
use teta_wot::testing::{Harness, TestClient};

use microscope_things::background::{ChannelDeviationDetector, ColourChannelDetector};
use microscope_things::camera::SimulatedCamera;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;

/// A camera with both background detectors, its sensor noise as shipped.
async fn start() -> Harness<SimulatedCamera> {
    Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .thing("bg_color_channels_luv", ColourChannelDetector::default())
        .thing(
            "bg_channel_deviations_luv",
            ChannelDeviationDetector::default(),
        )
        .start()
        .await
        .expect("the harness starts")
}

async fn put(client: &TestClient, path: &str, value: Value) {
    let response = client.put_json(path, &value).await;
    assert!(response.status.is_success(), "{path}: {}", response.text());
}

/// Runs an action over HTTP: its final record.
async fn run(client: &TestClient, action: &str) -> Value {
    let response = client.post_json(action, Some(&json!({}))).await;
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

#[tokio::test]
async fn both_detectors_tell_tissue_from_the_empty_slide_despite_the_lighting() {
    let harness = start().await;
    let client = harness.client();
    put(client, "/camera/sample_type", json!("tissue")).await;
    put(client, "/camera/tissue_coverage", json!(1.0)).await;
    put(client, "/camera/vignetting", json!(0.4)).await;
    put(client, "/camera/uneven_illumination", json!(0.3)).await;
    for detector in ["bg_color_channels_luv", "bg_channel_deviations_luv"] {
        put(client, "/camera/background_detector_name", json!(detector)).await;
        let required = || async { client.get("/camera/calibration_required").await.json() };
        assert_eq!(required().await, json!(true), "{detector}");

        let calibrated = run(client, "/camera/full_auto_calibrate").await;
        assert_eq!(calibrated["status"], json!("completed"), "{calibrated}");
        assert_eq!(required().await, json!(false), "{detector}");
        assert_eq!(
            client.get("/camera/sample_loaded").await.json(),
            json!(true)
        );

        let tissue = run(client, "/camera/image_is_sample").await["output"].clone();
        assert_eq!(tissue["sample"], json!(true), "{detector}: {tissue}");
        assert!(
            tissue["coverage"].as_f64().expect("a number") > 25.0,
            "{detector}: {tissue}"
        );

        assert_eq!(
            run(client, "/camera/remove_sample").await["status"],
            json!("completed")
        );
        let empty = run(client, "/camera/image_is_sample").await["output"].clone();
        assert_eq!(empty["sample"], json!(false), "{detector}: {empty}");
        assert!(
            empty["message"]
                .as_str()
                .expect("words")
                .starts_with("only "),
            "{empty}"
        );
        run(client, "/camera/load_sample").await;
    }
    harness.stop().await;
}

#[tokio::test]
async fn the_camera_refuses_unknown_detectors_and_asks_for_a_background() {
    let harness = start().await;
    let client = harness.client();
    let name = client.get("/camera/background_detector_name").await.json();
    assert_eq!(name, json!("bg_channel_deviations_luv"));
    let refused = client
        .put_json("/camera/background_detector_name", &json!("bg_magic"))
        .await;
    assert_eq!(refused.status.as_u16(), 422, "{}", refused.text());

    let unset = run(client, "/camera/image_is_sample").await;
    assert_eq!(unset["status"], json!("error"), "{unset}");
    assert!(
        unset.to_string().contains("background isn't set"),
        "{unset}"
    );

    // With the light off, the frame is blank: no background.
    run(client, "/illumination/set_led").await;
    let off = client
        .post_json("/illumination/set_led", Some(&json!({"led_on": false})))
        .await;
    assert!(off.status.is_success());
    let blank = run(client, "/camera/set_background").await;
    assert_eq!(blank["status"], json!("error"), "{blank}");
    assert!(blank.to_string().contains("blank"), "{blank}");
    harness.stop().await;
}

#[tokio::test]
async fn the_calibration_panels_name_what_the_things_have() {
    let harness = start().await;
    let client = harness.client();
    let named = |panel: Value, key: &str| -> Vec<String> {
        let elements = panel.as_array().expect("a list").clone();
        elements
            .iter()
            .map(|element| {
                assert_eq!(element.get("broken"), None, "{element}");
                element[key].as_str().expect("named").to_owned()
            })
            .collect()
    };
    let primary = client
        .get("/camera/primary_calibration_actions")
        .await
        .json();
    assert_eq!(named(primary, "action"), ["full_auto_calibrate"]);
    let secondary = client
        .get("/camera/secondary_calibration_actions")
        .await
        .json();
    assert_eq!(
        named(secondary, "action"),
        ["set_background", "remove_sample", "load_sample"]
    );
    for detector in ["bg_color_channels_luv", "bg_channel_deviations_luv"] {
        let panel = client.get(&format!("/{detector}/settings_ui")).await.json();
        assert_eq!(
            named(panel, "property"),
            ["channel_tolerance", "min_sample_coverage"]
        );
    }
    harness.stop().await;
}
