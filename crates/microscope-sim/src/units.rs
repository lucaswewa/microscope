//! Frames and units (ADR-0017).
//!
//! - **Stage steps:** a stage's positions are whole steps per axis
//!   (ADR-0019), and a [`StageScale`] says how many µm one step is.
//! - **The sample plane,** in µm: x to the right and y downwards as the
//!   camera sees it, with the sample's centre at the origin. The stage's x
//!   and y in µm are the sample point at the centre of the field of view.
//! - **Image pixels:** a pixel covers [`um_per_px`] µm of the sample, the
//!   sensor's pixel pitch divided by the objective's magnification.

/// A point in the sample plane, in µm.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// To the right, in µm.
    pub x: f64,
    /// Downwards, in µm.
    pub y: f64,
}

impl Point {
    /// The point `(x, y)`, in µm.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// How far one step moves each axis (x, y, z), in µm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageScale {
    /// µm per step, for x, y and z.
    pub um_per_step: [f64; 3],
}

impl StageScale {
    /// A position in steps, in µm.
    pub fn to_um(&self, steps: [i64; 3]) -> [f64; 3] {
        std::array::from_fn(|axis| steps[axis] as f64 * self.um_per_step[axis])
    }

    /// A position in µm, in the nearest whole steps.
    pub fn to_steps(&self, um: [f64; 3]) -> [i64; 3] {
        std::array::from_fn(|axis| (um[axis] / self.um_per_step[axis]).round() as i64)
    }
}

/// How much of the sample one image pixel covers, in µm: the sensor's pixel
/// pitch (µm) divided by the objective's magnification.
pub fn um_per_px(pixel_pitch_um: f64, magnification: f64) -> f64 {
    pixel_pitch_um / magnification
}
