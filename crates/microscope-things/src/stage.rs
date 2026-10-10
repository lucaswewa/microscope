//! The `stage` Thing in simulation: `SimulatedStage`, on the simulator's
//! stage model (P14), mirroring OpenFlexure's stage.
//!
//! - **Frames:** positions are in the program's frame. An axis whose
//!   `axis_inverted` setting is on counts the other way from the hardware
//!   (here, the model).
//! - **Moves:** `move_relative`, `move_absolute` and `move_to_origin` wait
//!   until the stage stops. Cancelling one stops the stage where it is. A
//!   stage that reaches its travel limit ends the move with an error. With
//!   `backlash_compensation`, a move ends travelling in the direction its
//!   axes' `backlash_steps` prefer (see [`BacklashCompensation`]).
//! - **Jogs:** `jog` starts a move and returns at once. It takes no global
//!   lock, and the latest command wins; `stop: true` halts the stage, as
//!   does `halt`.
//! - **Tracking:** while the stage moves, a task refreshes `position` and
//!   `moving` every 20 ms; `arrived` is emitted when a move ends.
//! - **Simulation:** the `simulation` setting switches backlash and travel
//!   limits on and off, and sets the speed and the µm per step, which the
//!   simulated camera uses to draw where the sample really is.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use microscope_sim::clock::Clock;
use microscope_sim::{StageConfig, StageModel};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::hardware::{AxisScale, BacklashCompensation, Position, StageApi};

/// How often the position is refreshed while the stage moves.
const POLL: Duration = Duration::from_millis(20);

/// One value per axis.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PerAxis<T> {
    /// The x axis's.
    pub x: T,
    /// The y axis's.
    pub y: T,
    /// The z axis's.
    pub z: T,
}

impl<T: Copy> PerAxis<T> {
    /// `(x, y, z)`.
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    fn to_array(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }
}

/// The lowest and highest position an axis reaches, in steps from where the
/// stage started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Travel {
    /// The lowest position.
    pub min: i64,
    /// The highest position.
    pub max: i64,
}

/// An axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// x.
    X,
    /// y.
    Y,
    /// z, the focus.
    Z,
}

/// How the simulated stage behaves. Its imperfections can be switched on
/// and off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageSimulation {
    /// How far a step moves each axis, in µm.
    pub um_per_step: PerAxis<f64>,
    /// How fast each axis moves, in steps per second.
    pub speed: PerAxis<f64>,
    /// Whether the axes have mechanical backlash.
    pub backlash: bool,
    /// How much, in steps, when they do.
    pub backlash_size: PerAxis<f64>,
    /// Whether the axes stop at the ends of their travel.
    pub limits: bool,
    /// Each axis's travel, when they do.
    pub travel: PerAxis<Travel>,
}

impl Default for StageSimulation {
    /// 0.1 µm steps in x and y and 0.05 µm in z, at 1,000 and 500 steps a
    /// second; no backlash (80 steps when on); no limits (±10 mm in x and y,
    /// ±1 mm in z when on).
    fn default() -> Self {
        let xy = Travel {
            min: -100_000,
            max: 100_000,
        };
        Self {
            um_per_step: PerAxis::new(0.1, 0.1, 0.05),
            speed: PerAxis::new(1000.0, 1000.0, 500.0),
            backlash: false,
            backlash_size: PerAxis::new(80.0, 80.0, 80.0),
            limits: false,
            travel: PerAxis::new(
                xy,
                xy,
                Travel {
                    min: -20_000,
                    max: 20_000,
                },
            ),
        }
    }
}

impl StageSimulation {
    fn model_config(&self) -> StageConfig {
        StageConfig {
            speed: self.speed.to_array(),
            backlash: if self.backlash {
                self.backlash_size.to_array()
            } else {
                [0.0; 3]
            },
            limits: if self.limits {
                self.travel
                    .to_array()
                    .map(|travel| Some((travel.min, travel.max)))
            } else {
                [None; 3]
            },
        }
    }
}

/// The system's clock: the time since the stage was made.
#[derive(Debug)]
struct SystemClock(Instant);

impl Default for SystemClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

/// The model, and what the Thing remembers about it.
#[derive(Debug)]
struct State {
    model: StageModel,
    /// The simulation settings the model has.
    applied: StageSimulation,
    compensation: BacklashCompensation,
    /// Whether the last refresh found the stage moving.
    moving: bool,
    /// Where the move in progress started, in the hardware's frame.
    move_start: Position,
}

impl Default for State {
    fn default() -> Self {
        let applied = StageSimulation::default();
        Self {
            model: StageModel::new(applied.model_config()),
            applied,
            compensation: BacklashCompensation::default(),
            moving: false,
            move_start: Position::default(),
        }
    }
}

/// A simulated XYZ stage, mirroring OpenFlexure's stage.
#[derive(Thing)]
#[thing(interfaces(StageApi))]
pub struct SimulatedStage {
    /// Where the stage is, in steps.
    #[property(default = Position::default(), readonly)]
    position: Prop<Position>,
    /// Whether the stage is moving.
    #[property(default = false, readonly)]
    moving: Prop<bool>,
    /// The steps that take up each axis's backlash in a compensated move,
    /// in the hardware's frame: the sign is the direction the move ends in;
    /// 0 for none.
    #[setting(default = Position::new(200, 200, 200))]
    backlash_steps: Prop<Position>,
    /// Whether each axis counts the other way from the hardware.
    #[setting(default = PerAxis::default(), readonly)]
    axis_inverted: Prop<PerAxis<bool>>,
    /// Whether the z axis's direction has been checked.
    #[setting(default = false, readonly)]
    z_direction_checked: Prop<bool>,
    /// How the simulated stage behaves.
    #[setting(default = StageSimulation::default())]
    simulation: Prop<StageSimulation>,
    /// The stage stopped: where it is.
    #[event]
    arrived: Event<Position>,
    state: Mutex<State>,
    clock: SystemClock,
    started: Notify,
    tracker: Mutex<Option<JoinHandle<()>>>,
}

#[thing_impl]
impl SimulatedStage {
    /// The axes, in order.
    #[property]
    async fn axis_names(&self) -> Vec<String> {
        ["x", "y", "z"].map(String::from).to_vec()
    }

    /// How far a step moves each axis, in µm.
    #[property]
    async fn um_per_step(&self) -> AxisScale {
        self.scale()
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
    #[action]
    async fn move_relative(
        &self,
        ctx: ActionCtx,
        #[param(default)] x: i64,
        #[param(default)] y: i64,
        #[param(default)] z: i64,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        self.move_by(&ctx, Position::new(x, y, z), backlash_compensation)
            .await
    }

    /// Moves to `x`, `y` and `z`; an axis that isn't given stays where it is.
    #[action]
    async fn move_absolute(
        &self,
        ctx: ActionCtx,
        #[param(default)] x: Option<i64>,
        #[param(default)] y: Option<i64>,
        #[param(default)] z: Option<i64>,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        let here = self.here();
        let target = Position::new(
            x.unwrap_or(here.x),
            y.unwrap_or(here.y),
            z.unwrap_or(here.z),
        );
        self.move_by(&ctx, target - here, backlash_compensation)
            .await
    }

    /// Moves to (0, 0, 0).
    #[action]
    async fn move_to_origin(
        &self,
        ctx: ActionCtx,
        #[param(default)] backlash_compensation: bool,
    ) -> Result<Position, ActionError> {
        let here = self.here();
        self.move_by(&ctx, -here, backlash_compensation).await
    }

    /// Makes where the stage is (0, 0, 0), without moving it.
    #[action]
    async fn set_zero_position(&self) -> Position {
        let now = self.clock.now();
        {
            let mut state = self.state();
            let here = Position::from_array(state.model.reported(now));
            state.model.set_zero(now);
            // A jog in progress keeps its start in the new frame.
            state.move_start = state.move_start - here;
        }
        self.here()
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
    ) -> Position {
        if stop {
            self.stop_motion();
        } else {
            self.begin_move(self.flip(Position::new(x, y, z)));
        }
        self.here()
    }

    /// Stops the stage where it is. (OpenFlexure has no such action; the
    /// name `stop` is the lifecycle hook's in `teta-wot`.)
    #[action(global_lock = false)]
    async fn halt(&self) -> Position {
        self.stop_motion();
        self.here()
    }

    /// Makes an axis count the other way.
    #[action]
    async fn invert_axis_direction(&self, axis: Axis) -> Result<(), ActionError> {
        self.axis_inverted
            .update(|inverted| match axis {
                Axis::X => inverted.x = !inverted.x,
                Axis::Y => inverted.y = !inverted.y,
                Axis::Z => inverted.z = !inverted.z,
            })
            .map_err(ActionError::handled)?;
        self.refresh();
        Ok(())
    }

    /// Records the z axis's direction, once checked: whether it counts the
    /// other way from the hardware.
    #[action]
    async fn calibrate_z_direction(&self, z_inverted: bool) -> Result<(), ActionError> {
        self.axis_inverted
            .update(|inverted| inverted.z = z_inverted)
            .map_err(ActionError::handled)?;
        self.z_direction_checked
            .set(true)
            .map_err(ActionError::handled)?;
        self.refresh();
        Ok(())
    }

    /// Starts the task that tracks moves.
    #[on_start]
    async fn start_tracking(&self, ctx: ThingCtx) {
        let Some(stage) = ctx.server().thing_ref::<SimulatedStage>(ctx.name()) else {
            return;
        };
        let stage = Arc::clone(ThingRef::thing(&stage));
        let task = tokio::spawn(async move {
            loop {
                stage.started.notified().await;
                while stage.refresh() {
                    tokio::time::sleep(POLL).await;
                }
            }
        });
        *self.tracker.lock().expect("not poisoned") = Some(task);
    }

    /// Stops the task that tracks moves.
    #[on_stop]
    async fn stop_tracking(&self) {
        if let Some(task) = self.tracker.lock().expect("not poisoned").take() {
            task.abort();
        }
    }
}

impl SimulatedStage {
    /// Where the sample really is, in µm in the hardware's frame: what the
    /// simulated camera draws. It differs from the position by the zero
    /// offset and the backlash.
    pub fn true_position_um(&self) -> [f64; 3] {
        let now = self.clock.now();
        let mut state = self.state();
        self.update(&mut state, now);
        let steps = state.model.true_position(now);
        let scale = self.simulation.get().um_per_step.to_array();
        std::array::from_fn(|axis| steps[axis] * scale[axis])
    }

    fn scale(&self) -> AxisScale {
        let PerAxis { x, y, z } = self.simulation.get().um_per_step;
        AxisScale {
            x: Some(x),
            y: Some(y),
            z: Some(z),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("not poisoned")
    }

    /// Between the program's frame and the hardware's: inverted axes change
    /// sign. It is its own inverse.
    fn flip(&self, position: Position) -> Position {
        let inverted = self.axis_inverted.get();
        let sign = |inverted: bool| if inverted { -1 } else { 1 };
        Position::new(
            position.x * sign(inverted.x),
            position.y * sign(inverted.y),
            position.z * sign(inverted.z),
        )
    }

    /// Where the stage is now, after a refresh.
    fn here(&self) -> Position {
        self.refresh();
        self.position.get()
    }

    /// Moves by `displacement`, in the program's frame (with compensation,
    /// perhaps in two moves), waiting for each to end.
    async fn move_by(
        &self,
        ctx: &ActionCtx,
        displacement: Position,
        compensate: bool,
    ) -> Result<Position, ActionError> {
        let displacement = self.flip(displacement);
        let moves = if compensate {
            self.state()
                .compensation
                .plan(displacement, self.backlash_steps.get())
        } else {
            vec![displacement]
        };
        for step in moves {
            self.begin_move(step);
            loop {
                if !self.refresh() {
                    break;
                }
                if let Err(cancelled) = ctx.sleep(POLL).await {
                    self.stop_motion();
                    return Err(cancelled.into());
                }
            }
            let now = self.clock.now();
            self.state()
                .model
                .outcome(now)
                .map_err(ActionError::handled)?;
        }
        Ok(self.position.get())
    }

    /// Starts a move by `displacement`, in the hardware's frame, replacing
    /// any move in progress.
    fn begin_move(&self, displacement: Position) {
        let now = self.clock.now();
        {
            let mut state = self.state();
            if self.update(&mut state, now) {
                // The move it replaces went part of the way.
                let made = Position::from_array(state.model.reported(now)) - state.move_start;
                state.compensation.record(made, self.backlash_steps.get());
            }
            state.move_start = Position::from_array(state.model.reported(now));
            state.moving = true;
            state.model.move_by(now, displacement.to_array());
            self.update(&mut state, now);
        }
        self.started.notify_one();
    }

    /// Stops the stage where it is.
    fn stop_motion(&self) {
        let now = self.clock.now();
        let mut state = self.state();
        state.model.stop(now);
        self.update(&mut state, now);
    }

    /// Brings the Thing up to date with the model. Returns whether the stage
    /// is moving.
    fn refresh(&self) -> bool {
        let now = self.clock.now();
        self.update(&mut self.state(), now)
    }

    /// Applies changed simulation settings once the stage is at rest; brings
    /// `position` and `moving` up to date; and when a move has ended,
    /// records it and emits `arrived`. Returns whether the stage is moving.
    fn update(&self, state: &mut State, now: Duration) -> bool {
        let moving = state.model.is_moving(now);
        let hardware = Position::from_array(state.model.reported(now));
        if state.moving && !moving {
            let made = hardware - state.move_start;
            state.compensation.record(made, self.backlash_steps.get());
        }
        if !moving {
            let simulation = self.simulation.get();
            if state.applied != simulation {
                state.model.set_config(now, simulation.model_config());
                state.applied = simulation;
            }
        }
        let position = self.flip(hardware);
        if self.position.get() != position {
            let _ = self.position.set(position);
        }
        if self.moving.get() != moving {
            let _ = self.moving.set(moving);
        }
        if state.moving && !moving {
            self.arrived.emit(position);
        }
        state.moving = moving;
        moving
    }
}

impl StageApi for ThingRef<SimulatedStage> {
    fn position(&self) -> BoxFuture<'_, Result<Position, ActionError>> {
        let position = ThingRef::thing(self).here();
        Box::pin(async move { Ok(position) })
    }

    fn move_relative(
        &self,
        displacement: Position,
    ) -> BoxFuture<'_, Result<Position, ActionError>> {
        let Position { x, y, z } = displacement;
        SimulatedStageActions::move_relative(self, x, y, z, false)
    }

    fn move_absolute(&self, target: Position) -> BoxFuture<'_, Result<Position, ActionError>> {
        let Position { x, y, z } = target;
        SimulatedStageActions::move_absolute(self, Some(x), Some(y), Some(z), false)
    }

    fn stop(&self) -> BoxFuture<'_, Result<(), ActionError>> {
        Box::pin(async move { SimulatedStageActions::halt(self).await.map(|_| ()) })
    }

    fn axis_scale(&self) -> AxisScale {
        ThingRef::thing(self).scale()
    }
}
