//! Server-described UI (P24): a panel built for the simulated camera
//! serialises to the fixture that the web app's renderers are tested
//! against, including a control for a property the camera lacks, which is
//! broken. Run with `BLESS=1` to rewrite the fixture.

use std::path::PathBuf;

use serde_json::Value;
use teta_wot::testing::Harness;

use microscope_things::camera::SimulatedCamera;
use microscope_things::illumination::SimulatedIllumination;
use microscope_things::stage::SimulatedStage;
use microscope_things::ui::{Layout, RichText, ThingUi, UiElement};

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
