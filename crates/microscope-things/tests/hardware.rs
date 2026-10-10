//! The hardware interfaces (ADR-0018), through the test fakes: a Thing that
//! reaches the stage, camera and illumination only through their interfaces,
//! connected by configuration files, including one that swaps the camera.
//! Also the shared types' arithmetic, units and wire shape (ADR-0019).

use serde_json::{Value, json};
use teta_wot::prelude::*;
use teta_wot::server::ServerConfig;
use teta_wot::testing::TestClient;

use microscope_things::fakes::{self, FakeCamera, FakeIllumination, FakeStage};
use microscope_things::hardware::{AxisScale, CameraApi, IlluminationApi, Position, StageApi};

/// Uses hardware only through its interfaces, as autofocus and scanning will.
#[derive(Thing)]
pub struct Bench {
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    /// Optional hardware connects by interface, with no default name: a
    /// named default must exist, even for an `OptSlot`.
    #[slot]
    illumination: OptSlot<dyn IlluminationApi>,
}

#[thing_impl]
impl Bench {
    /// Uses each interface: the light on, a move there and back, a frame
    /// and a capture.
    #[action]
    async fn exercise(&self) -> Result<Value, ActionError> {
        if let Some(light) = self.illumination.get() {
            light.set_led(true).await?;
        }
        let start = self.stage.position().await?;
        let moved = self.stage.move_relative(Position::new(10, -5, 2)).await?;
        let back = self.stage.move_absolute(start).await?;
        self.stage.stop().await?;
        let frame = self.camera.grab_frame().await?;
        let capture = self.camera.capture_to_memory().await?;
        Ok(json!({
            "moved": moved,
            "back": back,
            "scale": self.stage.axis_scale(),
            "frame": [frame.width(), frame.height()],
            "frame_size": self.camera.frame_size(),
            "capture": [capture.metadata.width, capture.metadata.height],
            "stream": self.camera.stream_info(),
            "light": self.illumination.get().map(|light| light.led_on()),
        }))
    }
}

/// Builds a server from `things` with the fakes and the bench, runs the
/// bench, and hands the client to `check`.
async fn bench(things: Value) -> (Value, TestClient) {
    let registry =
        fakes::register(microscope_things::registry()).register::<Bench>("tests.hardware:Bench");
    let settings = tempfile_dir();
    let config = ServerConfig::from_value(&json!({
        "things": things,
        "settings_folder": settings,
    }))
    .expect("a valid configuration");
    let server = ThingServer::from_config(&config, &registry)
        .expect("the Things are known")
        .build()
        .expect("the server builds");
    let client = TestClient::start(server).await.expect("the server starts");
    let bench = client
        .runtime()
        .thing_ref::<Bench>("bench")
        .expect("configured");
    let result = bench.exercise().await.expect("the bench runs");
    (result, client)
}

/// A settings folder in the system's temporary folder, unique to this run.
fn tempfile_dir() -> String {
    let folder = std::env::temp_dir().join(format!(
        "microscope-hardware-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_nanos()
    ));
    folder.to_string_lossy().into_owned()
}

#[tokio::test]
async fn a_thing_uses_hardware_through_its_interfaces() {
    let (result, client) = bench(json!({
        "bench": "tests.hardware:Bench",
        "stage": "microscope.fakes:FakeStage",
        "camera": "microscope.fakes:FakeCamera",
        "illumination": "microscope.fakes:FakeIllumination",
    }))
    .await;
    assert_eq!(result["moved"], json!({"x": 10, "y": -5, "z": 2}));
    assert_eq!(result["back"], json!({"x": 0, "y": 0, "z": 0}));
    assert_eq!(result["scale"], json!({"x": 0.1, "y": 0.1, "z": 0.1}));
    assert_eq!(result["frame"], json!([64, 48]));
    assert_eq!(result["frame_size"], json!([64, 48]));
    assert_eq!(result["capture"], json!([128, 96]));
    assert_eq!(
        result["stream"],
        json!({"affordance": "mjpeg_stream", "width": 64, "height": 48})
    );
    assert_eq!(result["light"], json!(true));

    let runtime = client.runtime();
    let stage = runtime.thing_ref::<FakeStage>("stage").expect("configured");
    assert_eq!(
        ThingRef::thing(&stage).targets(),
        [Position::new(10, -5, 2), Position::new(0, 0, 0)]
    );
    let camera = runtime
        .thing_ref::<FakeCamera>("camera")
        .expect("configured");
    assert_eq!(ThingRef::thing(&camera).captures(), 1);
    assert!(
        runtime
            .thing_ref::<FakeIllumination>("illumination")
            .is_some()
    );
    client.stop().await;
}

#[tokio::test]
async fn configuration_alone_chooses_the_hardware() {
    // Two cameras; the bench is pointed at the second, and has no light.
    let (result, client) = bench(json!({
        "bench": {"class": "tests.hardware:Bench", "thing_slots": {"camera": "big_camera"}},
        "stage": "microscope.fakes:FakeStage",
        "camera": "microscope.fakes:FakeCamera",
        "big_camera": {"class": "microscope.fakes:FakeCamera", "kwargs": {"width": 320, "height": 240}},
    }))
    .await;
    assert_eq!(result["frame"], json!([320, 240]));
    assert_eq!(result["light"], Value::Null);
    let runtime = client.runtime();
    let camera = runtime
        .thing_ref::<FakeCamera>("camera")
        .expect("configured");
    let big = runtime
        .thing_ref::<FakeCamera>("big_camera")
        .expect("configured");
    assert_eq!(
        (
            ThingRef::thing(&camera).captures(),
            ThingRef::thing(&big).captures()
        ),
        (0, 1)
    );
    client.stop().await;
}

#[test]
fn the_fakes_are_not_in_the_servers_registry() {
    let registry = microscope_things::registry();
    for class in ["microscope.fakes:FakeStage", "FakeCamera"] {
        assert!(!registry.contains(class), "{class}");
    }
    assert!(registry.contains("microscope.system:MicroscopeSystem"));
}

#[test]
fn positions_add_and_subtract_and_go_on_the_wire_as_objects() {
    let a = Position::new(1, 2, 3);
    let b = Position::new(10, -20, 30);
    assert_eq!(a + b, Position::new(11, -18, 33));
    assert_eq!(b - a, Position::new(9, -22, 27));
    assert_eq!(-a, Position::new(-1, -2, -3));
    assert_eq!(Position::from_array(a.to_array()), a);
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        json!({"x": 1, "y": 2, "z": 3})
    );
}

#[test]
fn axis_scales_convert_where_they_are_known() {
    let scale = AxisScale {
        x: Some(0.5),
        y: Some(0.5),
        z: None,
    };
    assert_eq!(
        scale.to_um(Position::new(4, -2, 10)),
        [Some(2.0), Some(-1.0), None]
    );
    // Converting back needs every axis.
    assert_eq!(scale.to_steps([2.0, -1.0, 0.0]), None);
    let known = AxisScale::uniform(0.25);
    assert_eq!(
        known.to_steps([1.0, -0.6, 0.1]),
        Some(Position::new(4, -2, 0))
    );
    assert_eq!(
        serde_json::to_value(scale).unwrap(),
        json!({"x": 0.5, "y": 0.5, "z": null})
    );
}
