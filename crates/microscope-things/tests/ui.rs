//! Server-described UI (P24, P24b): a panel built for the simulated camera
//! serialises to the fixture that the web app's renderers are tested
//! against. That includes a control, a button and a download for things the
//! camera lacks, which are broken. Run with `BLESS=1` to rewrite the
//! fixture.

use std::path::PathBuf;

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::testing::Harness;

use microscope_things::camera::SimulatedCamera;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;
use microscope_things::ui::{Layout, RichText, Success, ThingUi, UiElement};

/// The fixture, shared with the web app's tests.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/tests/fixtures/server-ui/panel.json")
}

#[tokio::test]
async fn a_panel_serialises_to_the_shared_fixture() {
    let harness = Harness::builder("camera", SimulatedCamera::default())
        .thing("stage", SimulatedStage::default())
        .thing("illumination", SimulatedIllumination::default())
        .start()
        .await
        .expect("the harness starts");
    let runtime = harness.client().runtime();
    let ui = ThingUi::new(runtime.thing("camera").expect("served"));
    let panel = vec![
        UiElement::Header {
            text: "Simulation".into(),
        },
        UiElement::Text {
            text: RichText::from("What the ")
                .italic("simulated")
                .and(" camera sees. See ")
                .link("the guide", "https://example.org/guide")
                .and("."),
        },
        UiElement::Bullets {
            items: vec![
                "Changes apply at once.".into(),
                RichText::default()
                    .bold("Defaults")
                    .and(" aren't highlighted."),
            ],
        },
        ui.property("exposure_time", "Exposure (µs)")
            .step(100.0)
            .read_back(500)
            .into(),
        UiElement::Container {
            layout: Layout::Row,
            children: vec![
                ui.property("repeating", "Repeating sample").into(),
                ui.property("objective", "Objective")
                    .options([("10×", 10), ("40×", 40)])
                    .into(),
            ],
        },
        UiElement::Accordion {
            title: "More".to_owned(),
            children: vec![
                ui.property("settling_time", "Settling time")
                    .duration_hms()
                    .into(),
                ui.property("colour", "Colours").into(),
                ui.property("no_such_property", "Missing").into(),
            ],
        },
        UiElement::Container {
            layout: Layout::Row,
            children: vec![
                ui.action("settle", "Settle")
                    .confirm("Wait for the camera to settle?")
                    .in_dialog(Some("camera"))
                    .on_success(Success::Message("Settled.".to_owned()))
                    .refresh()
                    .into(),
                ui.action("capture_to_memory", "Capture")
                    .input(json!({"buffer_max": 2}))
                    .not_cancellable()
                    .on_success(Success::Output)
                    .secondary()
                    .into(),
                ui.action("no_such_action", "Missing").into(),
            ],
        },
        UiElement::Container {
            layout: Layout::Row,
            children: vec![
                ui.download_action("grab_jpeg", "Download a frame", "frame.jpg")
                    .input(json!({"stream_name": "lores"}))
                    .into(),
                ui.download_endpoint("settings.json", "Download settings", "camera.json")
                    .secondary()
                    .into(),
            ],
        },
    ];
    let json = serde_json::to_string_pretty(&panel).expect("serialises") + "\n";
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(fixture(), &json).expect("written");
    }
    let expected: Value = serde_json::from_slice(&std::fs::read(fixture()).expect("read"))
        .expect("the fixture is JSON");
    assert_eq!(
        serde_json::from_str::<Value>(&json).expect("JSON"),
        expected
    );
    harness.stop().await;
}

/// A Thing with a file to download.
#[derive(Thing)]
struct Archive {}

#[thing_impl]
impl Archive {
    #[endpoint(get, "settings.json")]
    async fn settings(&self) -> String {
        "{}".to_owned()
    }

    #[endpoint(post, "upload")]
    async fn upload(&self) -> String {
        String::new()
    }
}

#[tokio::test]
async fn a_download_from_an_endpoint_needs_one_that_answers_get() {
    let harness = Harness::builder("archive", Archive {})
        .start()
        .await
        .expect("the harness starts");
    let runtime = harness.client().runtime();
    let ui = ThingUi::new(runtime.thing("archive").expect("served"));
    assert!(
        !ui.download_endpoint("settings.json", "Settings", "a.json")
            .broken
    );
    assert!(ui.download_endpoint("other.json", "Other", "b.json").broken);
    assert!(ui.download_endpoint("upload", "Upload", "c.json").broken);
    harness.stop().await;
}
