//! The microscope's Web of Things.
//!
//! This crate holds every `teta-wot` Thing the server offers (`camera`,
//! `stage`, `autofocus`, `camera_stage_mapping`, `smart_scan`, `gallery`, …),
//! the [`hardware`] interfaces through which Things use one another (so a
//! configuration can swap the simulator for real drivers), and the model for
//! server-described UI elements. [`registry()`] lists the Things a
//! configuration can name; [`fakes`] has a test Thing for each interface.
//! The Things' names mirror the OpenFlexure Microscope's where practical.
//!
//! The algorithms live in `microscope-core` and the simulator in
//! `microscope-sim`; this crate connects them to `teta-wot`.

pub mod camera;
pub mod data;
pub mod fakes;
pub mod hardware;
pub mod illumination;
pub mod registry;
pub mod stage;
pub mod system;

pub use registry::registry;
