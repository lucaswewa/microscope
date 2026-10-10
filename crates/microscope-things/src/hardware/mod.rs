//! The seams between the microscope's Things and its hardware (ADR-0018).
//!
//! Things that use hardware (autofocus, mapping, scanning, …) reach it only
//! through these interfaces: [`StageApi`], [`CameraApi`] and
//! [`IlluminationApi`]. Their slots name the interface, never a type
//! (`Slot<dyn StageApi>`), so a configuration file chooses the driver: the
//! simulator in milestone 1, real hardware in milestone 2.
//!
//! The conventions every driver follows:
//!
//! - **One Thing type per driver,** such as `SimulatedStage` or
//!   `SangaboardStage`, since `teta-wot` Things can't be generic. Each lists
//!   its interfaces in `#[thing(interfaces(…))]` and implements them for
//!   `ThingRef<Self>`.
//! - **Shared behaviour lives in plain structs** that each driver's Thing
//!   composes: capture buffers, streams, backlash compensation, the jog
//!   queue.
//! - **A vendor SDK that needs a thread of its own** runs behind a `teta-wot`
//!   `Device<D>` actor (milestone 2).
//! - **Names mirror OpenFlexure's** where they fit (ADR-0007): the Things are
//!   `stage`, `camera` and `illumination`, with actions such as
//!   `move_relative` and `capture_to_memory`.
//! - **Positions are whole steps** per axis, with an optional µm scale
//!   (ADR-0019).

mod camera;
mod illumination;
mod stage;

pub use camera::{CameraApi, Capture, CaptureMetadata, StreamInfo};
pub use illumination::IlluminationApi;
pub use stage::{AxisScale, Position, StageApi};
