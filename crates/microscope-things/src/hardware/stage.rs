//! What other Things need from a stage, and its units (ADR-0019).

use std::ops::{Add, Neg, Sub};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;

/// A stage position or displacement, in whole steps per axis: motor steps
/// for a stepper stage, the native resolution for a µm-native one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    /// Steps in x.
    pub x: i64,
    /// Steps in y.
    pub y: i64,
    /// Steps in z, the focus.
    pub z: i64,
}

impl Position {
    /// The position `(x, y, z)`.
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    /// As `[x, y, z]`.
    pub const fn to_array(self) -> [i64; 3] {
        [self.x, self.y, self.z]
    }

    /// From `[x, y, z]`.
    pub const fn from_array([x, y, z]: [i64; 3]) -> Self {
        Self { x, y, z }
    }
}

impl Add for Position {
    type Output = Position;

    fn add(self, other: Position) -> Position {
        Position::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Position {
    type Output = Position;

    fn sub(self, other: Position) -> Position {
        Position::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Neg for Position {
    type Output = Position;

    fn neg(self) -> Position {
        Position::new(-self.x, -self.y, -self.z)
    }
}

/// How far one step moves each axis, in µm, where the stage knows it: a
/// µm-native stage always does; a stepper stage does once it is calibrated.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AxisScale {
    /// µm per step in x, if known.
    pub x: Option<f64>,
    /// µm per step in y, if known.
    pub y: Option<f64>,
    /// µm per step in z, if known.
    pub z: Option<f64>,
}

impl AxisScale {
    /// The same known scale on every axis.
    pub const fn uniform(um_per_step: f64) -> Self {
        Self {
            x: Some(um_per_step),
            y: Some(um_per_step),
            z: Some(um_per_step),
        }
    }

    /// A position in µm per axis; `None` on an axis whose scale isn't known.
    pub fn to_um(&self, position: Position) -> [Option<f64>; 3] {
        let scales = [self.x, self.y, self.z];
        let steps = position.to_array();
        std::array::from_fn(|axis| scales[axis].map(|scale| steps[axis] as f64 * scale))
    }

    /// The whole steps nearest to a distance in µm per axis, if every axis's
    /// scale is known.
    pub fn to_steps(&self, um: [f64; 3]) -> Option<Position> {
        let [x, y, z] = [self.x?, self.y?, self.z?];
        let scales = [x, y, z];
        Some(Position::from_array(std::array::from_fn(|axis| {
            (um[axis] / scales[axis]).round() as i64
        })))
    }
}

/// What other Things need from a stage, whatever drives it.
#[teta_wot::interface]
pub trait StageApi: Send + Sync {
    /// Where the stage is.
    fn position(&self) -> BoxFuture<'_, Result<Position, ActionError>>;

    /// Moves by `displacement`, returning where the stage ends up.
    fn move_relative(&self, displacement: Position)
    -> BoxFuture<'_, Result<Position, ActionError>>;

    /// Moves to `target`, returning where the stage ends up.
    fn move_absolute(&self, target: Position) -> BoxFuture<'_, Result<Position, ActionError>>;

    /// Stops any move at once, where the stage is.
    fn stop(&self) -> BoxFuture<'_, Result<(), ActionError>>;

    /// How far a step moves each axis, where the stage knows.
    fn axis_scale(&self) -> AxisScale;
}
