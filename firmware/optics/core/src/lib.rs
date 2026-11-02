//! The optics controller's logic, with no hardware in it, so that it can be tested on a PC. The
//! firmware feeds it what the pins and USB say, and sets the pins from what it says.
//! `docs/misc/optima_optics_protocol.md` describes the protocol.
#![no_std]
#![forbid(unsafe_code)]

mod controller;
mod line;

pub use controller::{Controller, DOOR_SETTLE_US, Event, Inputs, MAX_TRIGGER_US, Outputs};
pub use line::{Line, LineBuffer, MAX_LINE, Request};

/// The protocol's version, which changes when a change could break a client.
pub const PROTOCOL_VERSION: u32 = 1;
/// The epi LEDs' channels: 1 is 395 nm, 2 is 470 nm, 3 is 565 nm and 4 is 625 nm.
pub const CHANNELS: usize = 4;
/// The cameras: 1 is the mono camera, and 2 the colour one.
pub const CAMERAS: usize = 2;
