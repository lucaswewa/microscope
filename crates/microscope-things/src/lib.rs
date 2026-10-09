//! The microscope's Web of Things.
//!
//! This crate will hold every `teta-wot` Thing the server offers (`camera`,
//! `stage`, `autofocus`, `camera_stage_mapping`, `smart_scan`, `gallery`, …),
//! the interfaces through which Things use one another (so a configuration
//! can swap the simulator for real drivers), and the model for server-described
//! UI elements. The Things' names mirror the OpenFlexure Microscope's where
//! practical.
//!
//! The algorithms live in `microscope-core` and the simulator in
//! `microscope-sim`; this crate connects them to `teta-wot`.
