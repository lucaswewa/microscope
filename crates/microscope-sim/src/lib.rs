//! The microscope simulator (ADR-0017).
//!
//! It models a microscope in physical units and draws what the camera sees
//! for any stage position and focus. P13 brings:
//!
//! - [`units`]: stage steps, the sample plane in µm, and image pixels;
//! - [`specimen`]: the [`Specimen`] trait, and [`blobs`], a procedural
//!   specimen of coloured blobs;
//! - [`optics`]: objectives from 4× to 100×, sensors, field of view, depth
//!   of field and defocus blur;
//! - [`render`]: [`render_frame`], which draws a frame from a [`SimState`];
//! - [`stage`]: [`StageModel`], a stage's motion with backlash and travel
//!   limits, and [`clock`], the time it is given (P14).
//!
//! Tissue and image-backed specimens, and the optical imperfections (P25),
//! come later. The crate depends on `microscope-core`
//! but not on `teta-wot`: the simulated Things in `microscope-things` wrap
//! it. Nothing reads the clock, so frames depend only on their inputs.

pub mod blobs;
pub mod clock;
pub mod filters;
pub mod optics;
pub mod random;
pub mod render;
pub mod specimen;
pub mod stage;
pub mod units;

pub use blobs::{BlobConfig, BlobSpecimen};
pub use optics::{OBJECTIVES, Objective, Sensor};
pub use render::{SimState, render_frame};
pub use specimen::{Extent, Specimen};
pub use stage::{MoveError, StageConfig, StageModel};
pub use units::{Point, StageScale};
