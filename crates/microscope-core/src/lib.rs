//! Microscope algorithms, free of I/O.
//!
//! This crate will hold the parts of the microscope that are pure
//! computation: geometry and units, image registration, camera–stage
//! mapping, focus metrics, background detection, scan planning, and mosaic
//! and deep-zoom output. It doesn't depend on `teta-wot`, an async runtime
//! or the file system, so its tests are fast and deterministic, and
//! milestone 2 can reuse it unchanged with real hardware.
//!
//! See ADR-0005 for the rules between the workspace's crates.
