//! `ZaberStage` (P04): against a scripted chain, each command and reply,
//! the frames, a jog, a stall, a refusal, a silent stage and the speed
//! settings; and on the owner's stage, where `MICROSCOPE_ZABER_PORT` names
//! its port.

use std::time::{Duration, Instant};

use serde_json::json;
use teta_wot::FromConfig;
use teta_wot::testing::Harness;
use tokio::sync::Mutex;

use microscope_things::drivers::hardware_device;
use microscope_things::drivers::serial::Script;
use microscope_things::hardware::{Position, StageApi};
use microscope_things::stage::Axis;
use microscope_things::zaber::{ZaberConfig, ZaberStage, ZaberStageActions};

/// The hardware tests share one port, so they take turns.
static ONE_AT_A_TIME: Mutex<()> = Mutex::const_new(());

/// A script of the chain's conversation, devices 1, 2 and 3 with axis 1,
/// numbering the commands as the stage does.
#[derive(Default)]
struct Chain {
    script: Script,
    id: u8,
}

impl Chain {
    /// The stage reading where it is as its port opens, and as it starts.
    fn started(at: [i64; 3]) -> Chain {
        Chain::default()
            .poll(at, "IDLE", IDLE)
            .poll(at, "IDLE", IDLE)
    }

    /// Then `command` to `device`, answered with `reply`, such as
    /// `OK IDLE -- 0`.
    fn ask(mut self, device: u8, command: &str, reply: &str) -> Chain {
        let id = self.id;
        self.id += 1;
        let sent = format!("/{device} 1 {id} {command}");
        let answer = format!("@{device:02} 1 {id:02} {reply}");
        self.script = self.script.expect(&sent, &[&answer]);
        self
    }

    /// Then where each axis is, `IDLE` or `BUSY`, with its warning.
    fn poll(self, at: [i64; 3], status: &str, warnings: [&str; 3]) -> Chain {
        (1..=3).fold(self, |chain, device| {
            let i = usize::from(device - 1);
            let reply = format!("OK {status} {} {}", warnings[i], at[i]);
            chain.ask(device, "get pos", &reply)
        })
    }

    /// Then `command` to each axis, accepted.
    fn each(self, command: &str) -> Chain {
        (1..=3).fold(self, |chain, device| {
            chain.ask(device, command, "OK IDLE -- 0")
        })
    }

    /// The stage on this script, which ends with each axis stopped as the
    /// stage closes.
    async fn start(self) -> (Harness<ZaberStage>, Script) {
        let script = self.each("stop").script;
        let mut config: ZaberConfig = serde_json::from_value(json!({"port": "scripted", "axes": {
            "x": {"device": 1}, "y": {"device": 2}, "z": {"device": 3, "microstep_um": 0.1}
        }}))
        .expect("a configuration");
        config.script = Some(script.clone());
        let harness = Harness::builder("stage", ZaberStage::from_config(config))
            .start()
            .await
            .expect("started");
        (harness, script)
    }
}

const IDLE: [&str; 3] = ["--", "--", "--"];

/// Stops the stage, and checks it said all its script.
async fn finish(harness: Harness<ZaberStage>, script: &Script) {
    harness.stop().await;
    assert_eq!(script.remaining(), 0, "{:?}", script.sent());
}

#[tokio::test]
async fn a_move_goes_to_the_axes_that_move_and_ends_where_the_stage_stops() {
    let (harness, script) = Chain::started([100, 200, 300])
        .poll([100, 200, 300], "IDLE", IDLE)
        .ask(1, "move rel 50", "OK BUSY -- 0")
        .ask(3, "move rel -5", "OK BUSY -- 0")
        .poll([120, 200, 298], "BUSY", IDLE)
        .poll([150, 200, 295], "IDLE", IDLE)
        .start()
        .await;
    let stage = harness.thing();
    let here = StageApi::position(stage).await.expect("read");
    assert_eq!(here, Position::new(100, 200, 300));
    let stopped = StageApi::move_relative(stage, Position::new(50, 0, -5)).await;
    assert_eq!(stopped.expect("moved"), Position::new(150, 200, 295));
    let scale = stage.axis_scale();
    assert_eq!((scale.x, scale.z), (None, Some(0.1)));
    finish(harness, &script).await;
}

#[tokio::test]
async fn positions_count_from_the_zero_in_each_axis_direction() {
    let at = [1000, 2000, 3000];
    let (harness, script) = Chain::started(at)
        // The zero, read, then where that leaves the stage.
        .poll(at, "IDLE", IDLE)
        .poll(at, "IDLE", IDLE)
        // z inverted.
        .poll(at, "IDLE", IDLE)
        .ask(1, "move abs 1010", "OK BUSY -- 0")
        .ask(3, "move abs 2995", "OK BUSY -- 0")
        .poll([1010, 2000, 2995], "IDLE", IDLE)
        .start()
        .await;
    let stage = harness.thing();
    let zeroed = ZaberStageActions::set_zero_position(stage).await;
    assert_eq!(zeroed.expect("zeroed"), Position::new(0, 0, 0));
    (ZaberStageActions::invert_axis_direction(stage, Axis::Z).await).expect("inverted");
    let moved = ZaberStageActions::move_absolute(stage, Some(10), None, Some(5), false).await;
    assert_eq!(moved.expect("moved"), Position::new(10, 0, 5));
    finish(harness, &script).await;
}

#[tokio::test]
async fn a_jog_returns_at_once_and_the_stage_follows_it() {
    let (harness, script) = Chain::started([0, 0, 0])
        .ask(2, "move rel 40", "OK BUSY -- 0")
        .poll([0, 30, 0], "BUSY", IDLE)
        .poll([0, 40, 0], "IDLE", IDLE)
        .start()
        .await;
    let stage = harness.thing();
    (ZaberStageActions::jog(stage, 0, 40, 0, false).await).expect("jogging");
    let deadline = Instant::now() + Duration::from_secs(5);
    while script.remaining() > 3 {
        assert!(Instant::now() < deadline, "{:?}", script.sent());
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    finish(harness, &script).await;
}

#[tokio::test]
async fn a_stall_ends_the_move_with_an_error_and_clears_the_warning() {
    let (harness, script) = Chain::started([0, 0, 0])
        .ask(1, "move rel 1000", "OK BUSY -- 0")
        .poll([400, 0, 0], "IDLE", ["FS", "--", "--"])
        .each("warnings clear")
        .start()
        .await;
    let stage = harness.thing();
    let stalled = StageApi::move_relative(stage, Position::new(1000, 0, 0)).await;
    assert_eq!(
        stalled.expect_err("stalled").to_string(),
        "the stage stopped: x: it stalled, and stopped (FS)"
    );
    finish(harness, &script).await;
}

#[tokio::test]
async fn a_move_the_stage_refuses_says_why() {
    let (harness, script) = Chain::started([0, 0, 0])
        .ask(1, "move abs 99999999", "RJ IDLE -- BADDATA")
        .start()
        .await;
    let stage = harness.thing();
    let target = Position::new(99_999_999, 0, 0);
    let refused = ZaberStageActions::move_absolute(stage, Some(target.x), None, None, false).await;
    let refused = refused.expect_err("refused").to_string();
    assert!(
        refused.contains("a position beyond its travel (BADDATA)"),
        "{refused}"
    );
    finish(harness, &script).await;
}

#[tokio::test]
async fn a_stage_that_doesnt_reply_stops_the_server_starting() {
    let mut config: ZaberConfig = serde_json::from_value(json!({"port": "COM9"})).expect("config");
    config.script = Some(Script::new().expect("/1 1 0 get pos", &[]));
    let started = Harness::builder("stage", ZaberStage::from_config(config))
        .start()
        .await;
    let error = format!("{:#}", started.err().expect("not started"));
    assert!(
        error.contains("didn't reply to `get pos`: is it on, and on this port?"),
        "{error}"
    );
}

#[tokio::test]
async fn speeds_and_accelerations_are_in_steps_a_second() {
    let (harness, script) = Chain::started([0, 0, 0])
        .ask(1, "get maxspeed", "OK IDLE -- 163840")
        .ask(1, "set maxspeed 81920", "OK IDLE -- 0")
        .ask(2, "set maxspeed 81920", "OK IDLE -- 0")
        .ask(3, "get accel", "OK IDLE -- 2")
        .start()
        .await;
    let client = harness.client();
    assert_eq!(client.get("/stage/xy_speed").await.json(), 100_000.0);
    let set = client.put_json("/stage/xy_speed", &json!(50_000.0)).await;
    assert!(set.status.is_success(), "{}", set.text());
    let z: f64 =
        serde_json::from_value(client.get("/stage/z_acceleration").await.json()).expect("a number");
    assert!((z - 12_207.03).abs() < 0.01, "{z}");
    finish(harness, &script).await;
}

#[tokio::test]
async fn the_settings_panel_has_the_speeds_and_homing() {
    let (harness, script) = Chain::started([0, 0, 0]).start().await;
    let panel = harness.client().get("/stage/settings_ui").await.json();
    let text = panel.to_string();
    for part in ["The Zaber stage", "xy_speed", "z_acceleration", "\"home\""] {
        assert!(text.contains(part), "{part}: {text}");
    }
    finish(harness, &script).await;
}

/// The owner's stage, where `MICROSCOPE_ZABER_PORT` names its port: devices
/// 1, 2 and 3, as integrated stages daisy-chained.
async fn real_stage() -> Option<Harness<ZaberStage>> {
    let port = hardware_device("MICROSCOPE_ZABER_PORT")?;
    let config = serde_json::from_value(json!({ "port": port })).expect("a configuration");
    let harness = Harness::builder("stage", ZaberStage::from_config(config))
        .start()
        .await
        .expect("the stage starts");
    Some(harness)
}

#[tokio::test]
async fn on_the_stage_moves_land_where_asked_and_come_back() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let Some(harness) = real_stage().await else {
        return;
    };
    let stage = harness.thing();
    let start = StageApi::position(stage).await.expect("read");
    // Small moves, and smaller in z, near the sample.
    for step in [(1000, 0, 0), (0, 1000, 0), (0, 0, 100)] {
        let step = Position::new(step.0, step.1, step.2);
        let there = StageApi::move_relative(stage, step).await.expect("moved");
        assert_eq!(there, start + step);
        let back = StageApi::move_relative(stage, -step)
            .await
            .expect("moved back");
        assert_eq!(back, start);
    }
    harness.stop().await;
}

#[tokio::test]
async fn on_the_stage_a_halt_stops_a_long_move() {
    let _turn = ONE_AT_A_TIME.lock().await;
    let Some(harness) = real_stage().await else {
        return;
    };
    let stage = harness.thing();
    let start = StageApi::position(stage).await.expect("read");
    (ZaberStageActions::jog(stage, 20_000, 0, 0, false).await).expect("jogging");
    tokio::time::sleep(Duration::from_millis(200)).await;
    (ZaberStageActions::halt(stage).await).expect("halted");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let stopped = StageApi::position(stage).await.expect("read");
    assert!(
        start.x < stopped.x && stopped.x < start.x + 20_000,
        "{start:?} {stopped:?}"
    );
    (StageApi::move_absolute(stage, start).await).expect("back");
    harness.stop().await;
}

/// Homing moves every axis to its end, so it runs only where
/// `MICROSCOPE_ZABER_HOMING` says the stage is clear.
#[tokio::test]
async fn on_the_stage_homing_ends_at_the_devices_zero() {
    let _turn = ONE_AT_A_TIME.lock().await;
    if hardware_device("MICROSCOPE_ZABER_HOMING").is_none() {
        return;
    }
    let Some(harness) = real_stage().await else {
        return;
    };
    let homed = ZaberStageActions::home(harness.thing()).await;
    assert_eq!(homed.expect("homed"), Position::new(0, 0, 0));
    harness.stop().await;
}
