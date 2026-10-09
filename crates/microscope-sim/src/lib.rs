//! The microscope simulator.
//!
//! This crate will model a microscope in physical units: specimens (procedural
//! blobs and tissue, or a loaded image), optics (objectives, focus, blur,
//! illumination, noise) and an XYZ stage (speed, backlash, travel limits). It
//! renders what the camera sees for any stage position and focus.
//!
//! It depends on `microscope-core`, but not on `teta-wot`: the simulated
//! Things in `microscope-things` wrap it. Time is injected rather than read
//! from the system clock, so tests can control it.
