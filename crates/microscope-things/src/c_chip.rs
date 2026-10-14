//! The C-Chip workflow (P37): the C-Chip pattern (P34) over a counting
//! chamber's well, with a basic stack (P36) at each site and no autofocus
//! between. The behaviour spec opens the P37 notes.

use std::sync::Arc;

use microscope_core::scan::{C_CHIP_COUNTS, C_CHIP_OVERLAP, RegularGrid};
use microscope_core::stack::{Counts, MAX_TEST};
use serde_json::json;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

use crate::autofocus::{self, Autofocus, AutofocusActions, StackOrigin, StackSettings, SweepStart};
use crate::hardware::{CaptureMode, Position};
use crate::mapping::CameraStageMapping;
use crate::ui::{OwnEntry, ThingUi, UiElement};
use crate::workflow::{self, Acquired, Begun, ScanWorkflow};

/// What the app calls it.
const NAME: &str = "C-Chip scan";
/// What it does, for the app to show.
const DESCRIPTION: &str = "For a C-Chip counting chamber: images its well in a fixed grid \
    of 5 columns and 7 rows, from where the stage is as its corner, with a stack of images \
    at each site, all on the starting focus.";

/// A scan of a C-Chip's well: see the module's description.
#[derive(Thing)]
#[thing(interfaces(ScanWorkflow))]
pub struct CChipWorkflow {
    #[slot(default = "autofocus")]
    autofocus: Slot<Autofocus>,
    #[slot(default = "camera_stage_mapping")]
    mapping: Slot<CameraStageMapping>,
    /// How far autofocus sweeps at the start, in steps.
    #[setting(default = 1000, ge = 400, le = 3000)]
    autofocus_dz: Prop<i64>,
    /// How each image is captured.
    #[setting(default = CaptureMode::Standard)]
    capture_mode: Prop<CaptureMode>,
    /// How many images each site's stack takes, all saved: odd.
    #[setting(default = 5, ge = 1, le = 9)]
    stack_images_to_save: Prop<u32>,
    /// The distance between a stack's images, in steps.
    #[setting(default = 50, ge = 10, le = 400)]
    stack_dz: Prop<i64>,
    entry: OwnEntry,
}

#[thing_impl]
impl CChipWorkflow {
    /// What the app calls it.
    #[property]
    async fn display_name(&self) -> String {
        NAME.to_owned()
    }

    /// Whether it can start: once the camera–stage mapping is calibrated.
    #[property]
    async fn ready(&self) -> bool {
        self.is_ready()
    }

    /// The settings, described for the Slide Scan tab (ADR-0024).
    #[endpoint(get, "settings_ui")]
    async fn settings_ui(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(panel)
    }

    /// Keeps the Thing's registry entry, for its panel's names.
    #[on_start]
    async fn keep_entry(&self, ctx: ThingCtx) {
        self.entry.keep::<Self>(&ctx);
    }
}

impl CChipWorkflow {
    fn is_ready(&self) -> bool {
        ThingRef::thing(&self.mapping).calibrated().is_some()
    }

    async fn begin(&self, start: Position) -> Result<Begun, ActionError> {
        let step = workflow::step(&self.mapping, C_CHIP_OVERLAP)?;
        let planner =
            RegularGrid::c_chip([start.x, start.y], step).map_err(ActionError::handled)?;
        let dz = self.autofocus_dz.get();
        let stack = self.stack();
        let images = autofocus::counts(&stack)?;
        self.autofocus
            .looping_autofocus(dz, SweepStart::Centre)
            .await?;
        let settings = json!({
            "pattern": "c_chip",
            "overlap": C_CHIP_OVERLAP,
            "dx": step.dx,
            "dy": step.dy,
            "x_count": C_CHIP_COUNTS[0],
            "y_count": C_CHIP_COUNTS[1],
            "autofocus_dz": dz,
            "capture_mode": stack.capture_mode,
            "stack_images_to_save": images.save,
            "stack_dz": stack.dz,
        });
        Ok(Begun {
            settings,
            planner: Box::new(planner),
        })
    }

    /// A basic stack centred on where z is, saving every image. Its centre
    /// is the site's focus, so the next site's stack is centred there too.
    async fn acquire(&self, folder: String) -> Result<Acquired, ActionError> {
        let mut stack = self.stack();
        // Odd already: `begin` said if it had to change it.
        let counts = Counts::new(stack.min_images_to_test, stack.images_to_save);
        stack.images_to_save = counts.save as u32;
        let zs = (self.autofocus)
            .run_basic_stack(folder, stack, StackOrigin::Centre)
            .await?;
        Ok(Acquired {
            images: zs.len() as u32,
            focus: zs.get(zs.len() / 2).copied(),
        })
    }

    /// The stack settings: as many tested as could be saved, since a basic
    /// stack saves every image.
    fn stack(&self) -> StackSettings {
        StackSettings {
            images_to_save: self.stack_images_to_save.get(),
            min_images_to_test: MAX_TEST,
            dz: self.stack_dz.get(),
            capture_mode: self.capture_mode.get(),
        }
    }
}

impl ScanWorkflow for ThingRef<CChipWorkflow> {
    fn display_name(&self) -> String {
        NAME.to_owned()
    }

    fn description(&self) -> String {
        DESCRIPTION.to_owned()
    }

    fn ready(&self) -> bool {
        ThingRef::thing(self).is_ready()
    }

    fn begin(&self, start: Position) -> BoxFuture<'_, Result<Begun, ActionError>> {
        let workflow = Arc::clone(ThingRef::thing(self));
        Box::pin(async move { workflow.begin(start).await })
    }

    fn acquire(&self, folder: String) -> BoxFuture<'_, Result<Acquired, ActionError>> {
        let workflow = Arc::clone(ThingRef::thing(self));
        Box::pin(async move { workflow.acquire(folder).await })
    }
}

/// The C-Chip workflow's settings panel.
fn panel(ui: &ThingUi) -> Vec<UiElement> {
    vec![
        ui.property("autofocus_dz", "Autofocus range (steps)")
            .into(),
        ui.property("capture_mode", "Capture mode").into(),
        ui.property("stack_images_to_save", "Images per site (odd)")
            .into(),
        ui.property("stack_dz", "Stack step (steps)").into(),
    ]
}
