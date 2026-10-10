//! Every Thing type a configuration file can name.
//!
//! Class names look like Python import strings
//! (`microscope.system:MicroscopeSystem`), as in LabThings configuration
//! files; `teta-wot` also accepts the short Rust type name
//! (`MicroscopeSystem`). Each phase that adds a Thing registers it here and
//! names it in `configs/simulation.json`. The plan's Things, by the names
//! they will have (plan §5.3):
//!
//! | Thing | Class | Phase |
//! |---|---|---|
//! | `system` | `microscope.system:MicroscopeSystem` | P02 (here) |
//! | `stage` | `microscope.stage:SimulatedStage` | P16 (here) |
//! | `camera`, `illumination` | `microscope.camera:SimulatedCamera`, `microscope.illumination:SimulatedIllumination` | P17 |
//! | `autofocus` | `microscope.autofocus:Autofocus` | P21 |
//! | `camera_stage_mapping` | `microscope.camera_stage_mapping:CameraStageMapping` | P23 |
//!
//! and later the background detectors (P27), `stage_measure` (P30),
//! `gallery` (P31), `smart_scan` and the scan workflows (P35, P37) and
//! `sequence` (P41). The test fakes ([`crate::fakes`]) are never here.

use teta_wot::server::ThingRegistry;

use crate::stage::SimulatedStage;
use crate::system::MicroscopeSystem;

/// Every Thing type a configuration file can name, under its class name.
pub fn registry() -> ThingRegistry {
    ThingRegistry::new()
        .register::<MicroscopeSystem>("microscope.system:MicroscopeSystem")
        .register::<SimulatedStage>("microscope.stage:SimulatedStage")
}
