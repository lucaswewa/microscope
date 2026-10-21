//! The `stage` Thing on a Zaber XYZ stage (P04, ADR-0042): `ZaberStage`,
//! over Zaber's ASCII protocol on a serial port.
//!
//! - **Axes:** x, y and z are each an axis on the chain: by default devices
//!   1, 2 and 3, axis 1, as integrated stages daisy-chained are.
//! - **Units:** a step is a microstep. Given each axis's microstep size in
//!   µm, from its datasheet, the stage is µm-native (ADR-0019).
//! - **Frames:** positions count from the zero `set_zero_position` sets, in
//!   the directions `axis_inverted` gives. The devices count from home.
//! - **Moves** wait until every axis is idle, polling where it is.
//!   Cancelled, the stage stops. A fault, such as a stall, ends the move
//!   with an error; a warning goes to the log.
//! - **Jogs** start a move and return at once; the latest wins.
//! - **The settings panel** has each axis's top speed and acceleration,
//!   read from the devices and written to them, and homing.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::drivers::device_error;
use crate::drivers::serial::{Line, Script, SerialPort};
use crate::drivers::zaber::{Address, Zaber, ZaberError, warning};
use crate::hardware::{AxisScale, Position, StageApi};
use crate::stage::{Axis, PerAxis};
use crate::ui::{Layout, OwnEntry, Success, UiElement};

/// How often the position is read while the stage moves.
const POLL: Duration = Duration::from_millis(50);
const NAMES: [&str; 3] = ["x", "y", "z"];
/// Zaber's units: a speed's data is microsteps a second times 1.6384, and
/// an acceleration's microsteps a second squared times 1.6384 / 10 000.
const SPEED: f64 = 1.6384;
const ACCELERATION: f64 = 1.6384 / 10_000.0;

/// How a `ZaberStage` is configured.
#[derive(Deserialize)]
pub struct ZaberConfig {
    /// The serial port, such as `COM3` or `/dev/ttyUSB0`.
    pub port: String,
    /// The port's speed: 115 200 bits a second, Zaber's for its ASCII
    /// protocol.
    #[serde(default = "baud")]
    pub baud: u32,
    /// Where each axis is on the chain, and its microstep size.
    #[serde(default = "chain")]
    pub axes: PerAxis<ZaberAxis>,
    /// A scripted chain in place of the port, for tests.
    #[serde(skip)]
    pub script: Option<Script>,
}

/// An axis on the chain.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ZaberAxis {
    /// Its device's number.
    pub device: u8,
    /// Its number on the device: 1 for an integrated stage.
    #[serde(default = "one")]
    pub axis: u8,
    /// How far a microstep moves it, in µm, from its datasheet.
    #[serde(default)]
    pub microstep_um: Option<f64>,
}

fn baud() -> u32 {
    115_200
}

fn one() -> u8 {
    1
}

fn chain() -> PerAxis<ZaberAxis> {
    let axis = |device| ZaberAxis {
        device,
        axis: 1,
        microstep_um: None,
    };
    PerAxis::new(axis(1), axis(2), axis(3))
}

/// The stage's axes, as a `teta-wot` driver on the device's thread.
pub struct ZaberDriver {
    zaber: Zaber,
    axes: [Address; 3],
}

/// Where each axis is on the device, whether any moves, and the warnings.
struct Polled {
    device: [i64; 3],
    busy: bool,
    warnings: [Option<String>; 3],
}

impl ZaberDriver {
    /// Opens the configured port, or its script.
    fn connect(config: &ZaberConfig) -> std::io::Result<ZaberDriver> {
        let line: Box<dyn Line> = match &config.script {
            Some(script) => Box::new(script.clone()),
            None => Box::new(SerialPort::open(&config.port, config.baud)?),
        };
        let PerAxis { x, y, z } = config.axes;
        let address = |axis: ZaberAxis| Address {
            device: axis.device,
            axis: axis.axis,
        };
        let axes = [address(x), address(y), address(z)];
        Ok(ZaberDriver {
            zaber: Zaber::new(line),
            axes,
        })
    }

    fn poll(&mut self) -> Result<Polled, ZaberError> {
        let mut polled = Polled {
            device: [0; 3],
            busy: false,
            warnings: Default::default(),
        };
        for (i, axis) in self.axes.into_iter().enumerate() {
            let reply = self.zaber.ask(axis, "get pos")?;
            polled.device[i] = reply.number()?;
            polled.busy |= reply.busy;
            polled.warnings[i] = reply.warning;
        }
        Ok(polled)
    }

    /// Sends each axis its command, where `command` gives one.
    fn each(&mut self, command: impl Fn(usize) -> Option<String>) -> Result<(), ZaberError> {
        for (i, axis) in self.axes.into_iter().enumerate() {
            if let Some(command) = command(i) {
                self.zaber.ask(axis, &command)?;
            }
        }
        Ok(())
    }

    /// The setting `name` of the axes `which`.
    fn get(&mut self, name: &str, which: usize) -> Result<i64, ZaberError> {
        self.zaber
            .ask(self.axes[which], &format!("get {name}"))?
            .number()
    }
}

impl Driver for ZaberDriver {
    fn open(&mut self) -> anyhow::Result<()> {
        let polled = self.poll()?;
        for (name, flag) in NAMES.iter().zip(&polled.warnings) {
            if let Some(flag) = flag {
                tracing::warn!("the stage's {name} axis: {}", warning(flag));
            }
        }
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        Ok(self.each(|_| Some("stop".to_owned()))?)
    }
}

/// A Zaber XYZ stage.
#[derive(Thing)]
#[thing(config = ZaberConfig, interfaces(StageApi))]
pub struct ZaberStage {
    /// Where the stage is, in steps.
    #[property(default = Position::default(), readonly)]
    position: Prop<Position>,
    /// Whether the stage is moving.
    #[property(default = false, readonly)]
    moving: Prop<bool>,
    /// The devices' position that is (0, 0, 0).
    #[setting(default = Position::default(), readonly)]
    zero: Prop<Position>,
    /// Whether each axis counts the other way from its device.
    #[setting(default = PerAxis::default(), readonly)]
    axis_inverted: Prop<PerAxis<bool>>,
    /// Whether the z axis's direction has been checked.
    #[setting(default = false, readonly)]
    z_direction_checked: Prop<bool>,
    /// The stage stopped: where it is.
    #[event]
    arrived: Event<Position>,
    #[device(init = ZaberDriver::connect(config))]
    device: Device<ZaberDriver>,
    #[thing(init = config.axes.map_scale())]
    scale: AxisScale,
    /// The warnings last logged, so each is logged once.
    warned: Mutex<[Option<String>; 3]>,
    started: Arc<Notify>,
    tracker: Mutex<Option<JoinHandle<()>>>,
    entry: OwnEntry,
}

impl PerAxis<ZaberAxis> {
    fn map_scale(&self) -> AxisScale {
        AxisScale {
            x: self.x.microstep_um,
            y: self.y.microstep_um,
            z: self.z.microstep_um,
        }
    }
}

#[thing_impl]
impl ZaberStage {
    /// The axes, in order.
    #[property]
    async fn axis_names(&self) -> Vec<String> {
        NAMES.map(String::from).to_vec()
    }

    /// How far a step moves each axis, in µm, where the configuration says.
    #[property]
    async fn um_per_step(&self) -> AxisScale {
        self.scale
    }

    /// Whether the stage needs calibrating: until the z axis's direction
    /// has been checked.
    #[property]
    async fn calibration_required(&self) -> bool {
        !self.z_direction_checked.get()
    }

    /// Whether the stage can be calibrated (its z direction checked).
    #[property]
    async fn can_calibrate(&self) -> bool {
        true
    }

    /// Moves by `x`, `y` and `z` steps, returning where the stage stopped.
    /// A Zaber stage needs no backlash compensation, so it's ignored.
    #[action]
    async fn move_relative(
        &self,
        ctx: ActionCtx,
        #[param(default)] x: i64,
        #[param(default)] y: i64,
        #[param(default)] z: i64,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        let _ = backlash_compensation;
        self.start_relative(Position::new(x, y, z)).await?;
        self.wait(&ctx).await
    }

    /// Moves to `x`, `y` and `z`; an axis that isn't given stays where it
    /// is. Backlash compensation is ignored.
    #[action]
    async fn move_absolute(
        &self,
        ctx: ActionCtx,
        #[param(default)] x: Option<i64>,
        #[param(default)] y: Option<i64>,
        #[param(default)] z: Option<i64>,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        let _ = backlash_compensation;
        let target = [x, y, z];
        let (zero, signs) = (self.zero.get().to_array(), self.signs());
        let device = move |i: usize| target[i].map(|to| zero[i] + signs[i] * to);
        self.call(move |driver| driver.each(|i| Some(format!("move abs {}", device(i)?))))
            .await?;
        self.wait(&ctx).await
    }

    /// Moves to (0, 0, 0).
    #[action]
    async fn move_to_origin(
        &self,
        ctx: ActionCtx,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        let (x, y, z) = (Some(0), Some(0), Some(0));
        self.move_absolute(ctx, x, y, z, backlash_compensation)
            .await
    }

    /// Makes where the stage is (0, 0, 0), without moving it.
    #[action]
    async fn set_zero_position(&self) -> Result<Position, ActionError> {
        let polled = self.call(ZaberDriver::poll).await?;
        let zero = Position::from_array(polled.device);
        self.zero.set(zero).map_err(ActionError::handled)?;
        self.refresh().await?;
        Ok(self.position.get())
    }

    /// Starts moving by `x`, `y` and `z` steps and returns at once, or with
    /// `stop`, halts. The latest jog wins.
    #[action(global_lock = false)]
    async fn jog(
        &self,
        #[param(default)] x: i64,
        #[param(default)] y: i64,
        #[param(default)] z: i64,
        #[param(default)] stop: bool,
    ) -> Result<Position, ActionError> {
        if stop {
            return self.halt().await;
        }
        self.start_relative(Position::new(x, y, z)).await?;
        self.started.notify_one();
        Ok(self.position.get())
    }

    /// Stops the stage where it is.
    #[action(global_lock = false)]
    async fn halt(&self) -> Result<Position, ActionError> {
        self.call(|driver| driver.each(|_| Some("stop".to_owned())))
            .await?;
        self.started.notify_one();
        Ok(self.position.get())
    }

    /// Moves every axis to its home sensor, which the devices count from.
    /// Clear the stage first: z homes too.
    #[action]
    async fn home(&self, ctx: ActionCtx) -> Result<Position, ActionError> {
        self.call(|driver| driver.each(|_| Some("home".to_owned())))
            .await?;
        self.wait(&ctx).await
    }

    /// Closes the stage's port, and opens it again.
    #[action]
    async fn reconnect(&self) -> Result<Position, ActionError> {
        self.device.reset().await.map_err(device_error)?;
        self.refresh().await?;
        Ok(self.position.get())
    }

    /// Makes an axis count the other way.
    #[action]
    async fn invert_axis_direction(&self, axis: Axis) -> Result<(), ActionError> {
        let update = |inverted: &mut PerAxis<bool>| match axis {
            Axis::X => inverted.x = !inverted.x,
            Axis::Y => inverted.y = !inverted.y,
            Axis::Z => inverted.z = !inverted.z,
        };
        (self.axis_inverted.update(update)).map_err(ActionError::handled)?;
        self.refresh().await.map(|_| ())
    }

    /// Records the z axis's direction, once checked: whether it counts the
    /// other way from its device.
    #[action]
    async fn calibrate_z_direction(&self, z_inverted: bool) -> Result<(), ActionError> {
        let update = |inverted: &mut PerAxis<bool>| inverted.z = z_inverted;
        (self.axis_inverted.update(update)).map_err(ActionError::handled)?;
        (self.z_direction_checked.set(true)).map_err(ActionError::handled)?;
        self.refresh().await.map(|_| ())
    }

    /// The x and y axes' top speed, in steps a second.
    #[property]
    async fn xy_speed(&self) -> Result<f64, PropertyError> {
        self.get_setting("maxspeed", 0, SPEED).await
    }

    #[setter(xy_speed)]
    async fn set_xy_speed(&self, speed: f64) -> Result<(), PropertyError> {
        self.set_setting("maxspeed", [true, true, false], speed * SPEED)
            .await
    }

    /// The z axis's top speed, in steps a second.
    #[property]
    async fn z_speed(&self) -> Result<f64, PropertyError> {
        self.get_setting("maxspeed", 2, SPEED).await
    }

    #[setter(z_speed)]
    async fn set_z_speed(&self, speed: f64) -> Result<(), PropertyError> {
        self.set_setting("maxspeed", [false, false, true], speed * SPEED)
            .await
    }

    /// The x and y axes' acceleration, in steps a second squared.
    #[property]
    async fn xy_acceleration(&self) -> Result<f64, PropertyError> {
        self.get_setting("accel", 0, ACCELERATION).await
    }

    #[setter(xy_acceleration)]
    async fn set_xy_acceleration(&self, acceleration: f64) -> Result<(), PropertyError> {
        let data = acceleration * ACCELERATION;
        self.set_setting("accel", [true, true, false], data).await
    }

    /// The z axis's acceleration, in steps a second squared.
    #[property]
    async fn z_acceleration(&self) -> Result<f64, PropertyError> {
        self.get_setting("accel", 2, ACCELERATION).await
    }

    #[setter(z_acceleration)]
    async fn set_z_acceleration(&self, acceleration: f64) -> Result<(), PropertyError> {
        let data = acceleration * ACCELERATION;
        self.set_setting("accel", [false, false, true], data).await
    }

    /// The stage's settings, for the settings panel (ADR-0024).
    #[endpoint(get, "settings_ui")]
    async fn settings_ui(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(|ui| {
            let row = |children| UiElement::Container {
                layout: Layout::Row,
                children,
            };
            let homed = Success::Message("The stage is homed.".to_owned());
            vec![
                UiElement::Header {
                    text: "The Zaber stage".into(),
                },
                row(vec![
                    ui.property("xy_speed", "x and y top speed (steps a second)")
                        .into(),
                    ui.property("z_speed", "z top speed (steps a second)")
                        .into(),
                ]),
                row(vec![
                    ui.property("xy_acceleration", "x and y acceleration (steps a second²)")
                        .into(),
                    ui.property("z_acceleration", "z acceleration (steps a second²)")
                        .into(),
                ]),
                UiElement::Text {
                    text: "Homing moves each axis to its home sensor, where it counts from. \
                           Clear the stage first: z homes too."
                        .into(),
                },
                (ui.action("home", "Home the stage").secondary())
                    .on_success(homed)
                    .into(),
            ]
        })
    }

    /// The z direction's check, for the calibration wizard (ADR-0024).
    #[endpoint(get, "z_direction_check")]
    async fn z_direction_check(&self) -> Json<Vec<UiElement>> {
        let z_inverted = self.axis_inverted.get().z;
        self.entry.panel(|ui| {
            let text = "Moving +z should raise the objective, towards the sample. Jog z up \
                        in the Control tab and watch: if it lowers the objective instead, \
                        invert z in Settings. Then confirm the direction.";
            let confirmed = Success::Message("The z direction is confirmed.".to_owned());
            vec![
                UiElement::Text { text: text.into() },
                ui.action("calibrate_z_direction", "Confirm the z direction")
                    .input(json!({ "z_inverted": z_inverted }))
                    .on_success(confirmed)
                    .refresh()
                    .into(),
            ]
        })
    }

    /// Starts the task that follows jogs, and reads where the stage is.
    #[on_start]
    async fn start_tracking(&self, ctx: ThingCtx) {
        self.entry.keep::<Self>(&ctx);
        if let Err(error) = self.refresh().await {
            tracing::warn!("the stage's position couldn't be read: {error}");
        }
        let Some(stage) = ctx.server().thing_ref::<ZaberStage>(ctx.name()) else {
            return;
        };
        let stage = Arc::clone(ThingRef::thing(&stage));
        let task = tokio::spawn(async move {
            loop {
                stage.started.notified().await;
                while let Ok(polled) = stage.refresh().await
                    && polled.busy
                {
                    tokio::time::sleep(POLL).await;
                }
            }
        });
        *self.tracker.lock().expect("not poisoned") = Some(task);
    }

    /// Stops the task that follows jogs.
    #[on_stop]
    async fn stop_tracking(&self) {
        if let Some(task) = self.tracker.lock().expect("not poisoned").take() {
            task.abort();
        }
    }
}

impl ZaberStage {
    /// Runs `f` on the driver.
    async fn call<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut ZaberDriver) -> Result<R, ZaberError> + Send + 'static,
    ) -> Result<R, ActionError> {
        let f = move |driver: &mut ZaberDriver, _: &CancelToken| f(driver);
        self.device.try_call(f).await.map_err(device_error)
    }

    /// `-1` on the axes that count the other way.
    fn signs(&self) -> [i64; 3] {
        let PerAxis { x, y, z } = self.axis_inverted.get();
        [x, y, z].map(|inverted| if inverted { -1 } else { 1 })
    }

    async fn start_relative(&self, displacement: Position) -> Result<(), ActionError> {
        let signs = self.signs();
        let steps = displacement.to_array();
        let command = move |i: usize| {
            let steps = signs[i] * steps[i];
            (steps != 0).then(|| format!("move rel {steps}"))
        };
        self.call(move |driver| driver.each(command)).await
    }

    /// Reads where the stage is, and whether it moves, and says so.
    async fn refresh(&self) -> Result<Polled, ActionError> {
        let polled = self.call(ZaberDriver::poll).await?;
        let (zero, signs) = (self.zero.get().to_array(), self.signs());
        let position = std::array::from_fn(|i| signs[i] * (polled.device[i] - zero[i]));
        let position = Position::from_array(position);
        let _ = self.position.set(position);
        let was_moving = self.moving.get();
        let _ = self.moving.set(polled.busy);
        if was_moving && !polled.busy {
            self.arrived.emit(position);
        }
        let mut warned = self.warned.lock().expect("not poisoned");
        for (i, flag) in polled.warnings.iter().enumerate() {
            if *flag != warned[i]
                && let Some(flag) = flag
            {
                tracing::warn!("the stage's {} axis: {}", NAMES[i], warning(flag));
            }
            warned[i].clone_from(flag);
        }
        Ok(polled)
    }

    /// Waits until every axis is idle, and returns where the stage stopped,
    /// or the faults that stopped it. Cancelled, it stops the stage.
    async fn wait(&self, ctx: &ActionCtx) -> Result<Position, ActionError> {
        loop {
            let polled = self.refresh().await?;
            if !polled.busy {
                let faults: Vec<String> = (polled.warnings.iter().enumerate())
                    .filter_map(|(i, flag)| Some((i, flag.as_ref()?)))
                    .filter(|(_, flag)| flag.starts_with('F'))
                    .map(|(i, flag)| format!("{}: {}", NAMES[i], warning(flag)))
                    .collect();
                if faults.is_empty() {
                    return Ok(self.position.get());
                }
                let clear = |_| Some("warnings clear".to_owned());
                self.call(move |driver| driver.each(clear)).await?;
                return Err(ActionError::handled(format!(
                    "the stage stopped: {}",
                    faults.join("; ")
                )));
            }
            if let Err(cancelled) = ctx.sleep(POLL).await {
                let _ = self.halt().await;
                return Err(cancelled.into());
            }
        }
    }

    async fn get_setting(&self, name: &str, which: usize, unit: f64) -> Result<f64, PropertyError> {
        let name = name.to_owned();
        let data = self.call(move |driver| driver.get(&name, which)).await;
        let data = data.map_err(|error| PropertyError::Failed(anyhow::anyhow!("{error}")))?;
        Ok(data as f64 / unit)
    }

    async fn set_setting(
        &self,
        name: &str,
        axes: [bool; 3],
        data: f64,
    ) -> Result<(), PropertyError> {
        let command = format!("set {name} {}", data.round() as i64);
        let set = move |driver: &mut ZaberDriver| driver.each(|i| axes[i].then(|| command.clone()));
        let done = self.call(set).await;
        done.map_err(|error| PropertyError::Failed(anyhow::anyhow!("{error}")))
    }
}

impl StageApi for ThingRef<ZaberStage> {
    fn position(&self) -> BoxFuture<'_, Result<Position, ActionError>> {
        Box::pin(async move {
            ThingRef::thing(self).refresh().await?;
            Ok(ThingRef::thing(self).position.get())
        })
    }

    fn move_relative(
        &self,
        displacement: Position,
    ) -> BoxFuture<'_, Result<Position, ActionError>> {
        let Position { x, y, z } = displacement;
        ZaberStageActions::move_relative(self, x, y, z, false)
    }

    fn move_absolute(&self, target: Position) -> BoxFuture<'_, Result<Position, ActionError>> {
        let Position { x, y, z } = target;
        ZaberStageActions::move_absolute(self, Some(x), Some(y), Some(z), false)
    }

    fn stop(&self) -> BoxFuture<'_, Result<(), ActionError>> {
        Box::pin(async move { ZaberStageActions::halt(self).await.map(|_| ()) })
    }

    fn set_zero(&self) -> BoxFuture<'_, Result<(), ActionError>> {
        Box::pin(async move { ZaberStageActions::set_zero_position(self).await.map(|_| ()) })
    }

    fn axis_scale(&self) -> AxisScale {
        ThingRef::thing(self).scale
    }
}
