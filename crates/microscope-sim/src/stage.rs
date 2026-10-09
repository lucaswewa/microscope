//! A simulated XYZ stage: how it moves, and how its imperfections show.
//!
//! Positions are in steps per axis (x, y, z). The stage keeps three:
//!
//! - **the motors',** which move in a straight line, all axes together, at
//!   each axis's set speed, and stop on whole steps;
//! - **the carriage's,** the true position of the sample, which follows the
//!   motors through each axis's backlash: a dead band of play in which the
//!   motors turn without moving it;
//! - **the reported position,** the motors' position minus the zero offset,
//!   which is what the stage tells its users.
//!
//! Every method takes the time, `now` (see [`crate::clock`]): the model
//! never reads a clock, so tests decide exactly what time it is. A new move
//! interrupts the one in progress from wherever the motors are: the latest
//! command wins.

use std::fmt;
use std::time::Duration;

/// How a [`StageModel`] behaves. Every imperfection can be switched off.
#[derive(Debug, Clone, PartialEq)]
pub struct StageConfig {
    /// How fast each axis moves, in steps per second. An axis whose speed
    /// isn't positive moves at once.
    pub speed: [f64; 3],
    /// Each axis's backlash, in steps: the play between the motor and the
    /// carriage. 0 for none.
    pub backlash: [f64; 3],
    /// Each axis's travel: the lowest and highest motor positions, in steps
    /// from where the stage started. `None` for none.
    pub limits: [Option<(i64, i64)>; 3],
}

impl Default for StageConfig {
    /// 1,000 steps a second in x and y and 500 in z, with no backlash and
    /// no limits.
    fn default() -> Self {
        Self {
            speed: [1000.0, 1000.0, 500.0],
            backlash: [0.0; 3],
            limits: [None; 3],
        }
    }
}

/// Why a move ended before its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// An axis (0, 1 or 2) reached the end of its travel, and the stage
    /// stopped there.
    LimitReached {
        /// The axis.
        axis: usize,
    },
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveError::LimitReached { axis } => {
                write!(
                    f,
                    "the {} axis reached the end of its travel",
                    ["x", "y", "z"][*axis]
                )
            }
        }
    }
}

impl std::error::Error for MoveError {}

/// A move in progress: a straight line from `from` towards `to`, which ends
/// at `end`, either on arrival or where an axis reached its limit.
#[derive(Debug, Clone)]
struct Motion {
    start: Duration,
    from: [f64; 3],
    to: [f64; 3],
    /// The time the whole way would take.
    duration: Duration,
    /// When the move ends, and how.
    end: Duration,
    outcome: Result<(), MoveError>,
}

impl Motion {
    /// The motors' position at `time`, which is no later than `end`.
    fn at(&self, time: Duration) -> [f64; 3] {
        let fraction = if self.duration.is_zero() {
            1.0
        } else {
            (time - self.start).as_secs_f64() / self.duration.as_secs_f64()
        };
        std::array::from_fn(|axis| self.from[axis] + (self.to[axis] - self.from[axis]) * fraction)
    }
}

/// The simulated stage (see the module's description).
#[derive(Debug, Clone)]
pub struct StageModel {
    config: StageConfig,
    /// The motors' position when the stage last stopped, or when the move in
    /// progress started.
    motor: [f64; 3],
    /// The carriage's position at the same time.
    carriage: [f64; 3],
    /// The motor position that reads as 0.
    zero: [i64; 3],
    motion: Option<Motion>,
    /// How the last move ended.
    outcome: Result<(), MoveError>,
}

impl StageModel {
    /// A stage at rest at 0, as `config` describes.
    pub fn new(config: StageConfig) -> Self {
        Self {
            config,
            motor: [0.0; 3],
            carriage: [0.0; 3],
            zero: [0; 3],
            motion: None,
            outcome: Ok(()),
        }
    }

    /// How the stage behaves.
    pub fn config(&self) -> &StageConfig {
        &self.config
    }

    /// Changes how the stage behaves. A move in progress stops first, and the
    /// carriage keeps within the new backlash.
    pub fn set_config(&mut self, now: Duration, config: StageConfig) {
        self.stop(now);
        self.config = config;
        self.carriage =
            std::array::from_fn(|axis| self.follow(axis, self.carriage[axis], self.motor[axis]));
    }

    /// Starts a move to `target`, a reported position. It replaces any move
    /// in progress, from wherever the motors are now.
    pub fn move_to(&mut self, now: Duration, target: [i64; 3]) {
        let from = self.halt(now);
        let to: [f64; 3] = std::array::from_fn(|axis| (target[axis] + self.zero[axis]) as f64);
        let seconds = (0..3)
            .map(|axis| {
                let speed = self.config.speed[axis];
                if speed > 0.0 {
                    (to[axis] - from[axis]).abs() / speed
                } else {
                    0.0
                }
            })
            .fold(0.0, f64::max);
        let duration = Duration::from_secs_f64(seconds);

        // The share of the way the stage gets before an axis reaches its limit.
        let (mut reach, mut outcome) = (1.0, Ok(()));
        for axis in 0..3 {
            let Some((low, high)) = self.config.limits[axis] else {
                continue;
            };
            let (start, end) = (from[axis], to[axis]);
            let limit = if end > high as f64 {
                high as f64
            } else if end < low as f64 {
                low as f64
            } else {
                continue;
            };
            let share = ((limit - start) / (end - start)).clamp(0.0, 1.0);
            if share < reach {
                (reach, outcome) = (share, Err(MoveError::LimitReached { axis }));
            }
        }

        self.motion = Some(Motion {
            start: now,
            from,
            to,
            duration,
            end: now + duration.mul_f64(reach),
            outcome,
        });
        self.outcome = Ok(());
    }

    /// Starts a move by `delta` steps from where the stage is now. Like
    /// [`move_to`](Self::move_to), it replaces any move in progress.
    pub fn move_by(&mut self, now: Duration, delta: [i64; 3]) {
        let here = self.reported(now);
        self.move_to(now, std::array::from_fn(|axis| here[axis] + delta[axis]));
    }

    /// Stops the stage where it is, on the nearest whole steps.
    pub fn stop(&mut self, now: Duration) {
        let motor = self.halt(now).map(f64::round);
        self.carriage =
            std::array::from_fn(|axis| self.follow(axis, self.carriage[axis], motor[axis]));
        self.motor = motor;
    }

    /// Whether a move is in progress at `now`.
    pub fn is_moving(&mut self, now: Duration) -> bool {
        self.settle(now);
        self.motion.is_some()
    }

    /// When the move in progress will end, if there is one.
    pub fn arrival(&self) -> Option<Duration> {
        self.motion.as_ref().map(|motion| motion.end)
    }

    /// How the last move ended: `Ok` while one is in progress.
    pub fn outcome(&mut self, now: Duration) -> Result<(), MoveError> {
        self.settle(now);
        self.outcome
    }

    /// The reported position: the motors' position, in whole steps, minus
    /// the zero offset.
    pub fn reported(&mut self, now: Duration) -> [i64; 3] {
        let motor = self.motor_at(now);
        std::array::from_fn(|axis| motor[axis].round() as i64 - self.zero[axis])
    }

    /// The true position: where the carriage really is, in steps from where
    /// the stage started. It differs from the reported one by the zero
    /// offset and the backlash.
    pub fn true_position(&mut self, now: Duration) -> [f64; 3] {
        self.settle(now);
        let motor = self.motor_at(now);
        std::array::from_fn(|axis| self.follow(axis, self.carriage[axis], motor[axis]))
    }

    /// Makes the position now read as 0, without moving.
    pub fn set_zero(&mut self, now: Duration) {
        let motor = self.motor_at(now);
        self.zero = motor.map(|steps| steps.round() as i64);
    }

    /// The motors' position at `now`.
    fn motor_at(&mut self, now: Duration) -> [f64; 3] {
        self.settle(now);
        match &self.motion {
            Some(motion) => motion.at(now),
            None => self.motor,
        }
    }

    /// Ends a move that has finished by `now`.
    fn settle(&mut self, now: Duration) {
        if let Some(motion) = self.motion.take_if(|motion| now >= motion.end) {
            self.finish(&motion, motion.end);
            self.outcome = motion.outcome;
        }
    }

    /// Ends any move at `now`, returning where the motors are.
    fn halt(&mut self, now: Duration) -> [f64; 3] {
        self.settle(now);
        if let Some(motion) = self.motion.take() {
            self.finish(&motion, now);
        }
        self.motor
    }

    fn finish(&mut self, motion: &Motion, time: Duration) {
        let motor = motion.at(time);
        self.carriage =
            std::array::from_fn(|axis| self.follow(axis, self.carriage[axis], motor[axis]));
        self.motor = motor;
    }

    /// Where the carriage ends up when the motor moves to `motor`: within
    /// half the backlash of it. A motor that moves one way within a move
    /// pushes the carriage along; the play only shows when it turns back.
    fn follow(&self, axis: usize, carriage: f64, motor: f64) -> f64 {
        let half = self.config.backlash[axis].max(0.0) / 2.0;
        carriage.clamp(motor - half, motor + half)
    }
}
