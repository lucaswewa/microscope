//! The microscope simulator (ADR-0017).
//!
//! It models a microscope in physical units and draws what the camera sees
//! for any stage position and focus. P13 brings:
//!
//! - [`units`]: stage steps, the sample plane in µm, and image pixels;
//! - [`specimen`]: the [`Specimen`] trait; [`blobs`], a procedural
//!   specimen of coloured blobs; [`tissue`], a procedural H&E-like section
//!   (P25); and [`slide_image`], a picture laid on the slide (P25);
//! - [`optics`]: objectives from 4× to 100×, sensors, field of view, depth
//!   of field and defocus blur;
//! - [`render`]: [`render_frame`], which draws a frame from a [`SimState`],
//!   with the optics' [`Effects`]: a focal surface, vignetting, uneven
//!   lighting, and exposure and gain (P25); [`noise`] is their smooth noise;
//! - [`stage`]: [`StageModel`], a stage's motion with backlash and travel
//!   limits, and [`clock`], the time it is given (P14).
//!
//! The crate depends on `microscope-core`
//! but not on `teta-wot`: the simulated Things in `microscope-things` wrap
//! it. Nothing reads the clock, so frames depend only on their inputs.

pub mod blobs;
pub mod clock;
pub mod filters;
pub mod noise;
pub mod optics;
pub mod random;
pub mod render;
pub mod slide_image;
pub mod specimen;
pub mod stage;
pub mod tissue;
pub mod units;

pub use blobs::{BlobConfig, BlobSpecimen};
pub use optics::{OBJECTIVES, Objective, Sensor};
pub use render::{Effects, FocalSurface, SimState, render_frame};
pub use slide_image::ImageSpecimen;
pub use specimen::{Extent, Specimen};
pub use stage::{MoveError, StageConfig, StageModel};
pub use tissue::{TissueConfig, TissueSpecimen};
pub use units::{Point, StageScale};
