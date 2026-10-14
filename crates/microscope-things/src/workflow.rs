//! Scan workflows (P35, ADR-0029): how a scan plans its path and what it
//! does at each site. `smart_scan` runs the one chosen, through the
//! `ScanWorkflow` interface. The snake and raster workflows are one Thing
//! type, `GridWorkflow`, configured with its order. At each site they take
//! a smart stack (P36). The histo and C-Chip workflows (P37) are in
//! [`crate::histo`] and [`crate::c_chip`], and share what's here.

use std::sync::Arc;

use microscope_core::scan::{Order, Planner, RegularGrid, Step};
use microscope_core::stack::Counts;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

use crate::autofocus::{self, Autofocus, AutofocusActions, StackSettings, SweepStart};
use crate::hardware::{CaptureMode, Position, StageApi};
use crate::mapping::CameraStageMapping;
use crate::ui::{OwnEntry, ThingUi, UiElement};

/// A scan's start: its settings, fixed for the scan, and its planner.
pub struct Begun {
    /// The settings, as the scan's record keeps them.
    pub settings: Value,
    /// Where the scan goes.
    pub planner: Box<dyn Planner + Send>,
}

/// What a workflow did at a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acquired {
    /// How many images it saved.
    pub images: u32,
    /// The z it found focus at, if it did.
    pub focus: Option<i64>,
}

/// What `smart_scan` needs from a workflow.
#[teta_wot::interface]
pub trait ScanWorkflow: Send + Sync {
    /// What the app calls it.
    fn display_name(&self) -> String;
    /// Whether it can start.
    fn ready(&self) -> bool;
    /// Starts a scan with the stage at `start`: it refuses if it can't, and
    /// otherwise gets ready, such as by autofocusing, and fixes its settings.
    fn begin(&self, start: Position) -> BoxFuture<'_, Result<Begun, ActionError>>;
    /// With the stage at a site: focuses, and saves its images in `folder`,
    /// a folder in the data folder.
    fn acquire(&self, folder: String) -> BoxFuture<'_, Result<Acquired, ActionError>>;
}

/// The order a `GridWorkflow` visits its grid in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GridOrder {
    /// Every other row reversed.
    #[default]
    Snake,
    /// Every row in the same direction.
    Raster,
}

/// How a `GridWorkflow` is configured.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct GridWorkflowConfig {
    /// Its order.
    #[serde(default)]
    pub order: GridOrder,
}

/// A scan over a grid of columns and rows, from the stage's position as its
/// first corner: autofocus there first, then a smart stack at each site.
#[derive(Thing)]
#[thing(config = GridWorkflowConfig, interfaces(ScanWorkflow))]
pub struct GridWorkflow {
    #[thing(init = config.order)]
    order: GridOrder,
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
    #[slot(default = "autofocus")]
    autofocus: Slot<Autofocus>,
    #[slot(default = "camera_stage_mapping")]
    mapping: Slot<CameraStageMapping>,
    /// The share of each image that its neighbours overlap.
    #[setting(default = 0.35, ge = 0.1, le = 0.7)]
    overlap: Prop<f64>,
    /// How far autofocus sweeps, in steps.
    #[setting(default = 1000, ge = 400, le = 3000)]
    autofocus_dz: Prop<i64>,
    /// How many columns.
    #[setting(default = 3, ge = 1)]
    x_count: Prop<u32>,
    /// How many rows.
    #[setting(default = 2, ge = 1)]
    y_count: Prop<u32>,
    /// How each image is captured.
    #[setting(default = CaptureMode::Standard)]
    capture_mode: Prop<CaptureMode>,
    /// How many images of each stack to save: odd.
    #[setting(default = 1, ge = 1, le = 9)]
    stack_images_to_save: Prop<u32>,
    /// How many of a stack's latest images are judged together: odd.
    #[setting(default = 9, ge = 3, le = 9)]
    stack_min_images_to_test: Prop<u32>,
    /// The distance between a stack's images, in steps.
    #[setting(default = 50, ge = 10, le = 400)]
    stack_dz: Prop<i64>,
    entry: OwnEntry,
}

#[thing_impl]
impl GridWorkflow {
    /// What the app calls it.
    #[property]
    async fn display_name(&self) -> String {
        self.name().to_owned()
    }

    /// Whether it can start: once the camera–stage mapping is calibrated.
    #[property]
    async fn ready(&self) -> bool {
        ThingRef::thing(&self.mapping).calibrated().is_some()
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

impl GridWorkflow {
    fn name(&self) -> &'static str {
        match self.order {
            GridOrder::Snake => "Snake scan",
            GridOrder::Raster => "Raster scan",
        }
    }

    async fn begin(&self, start: Position) -> Result<Begun, ActionError> {
        let overlap = self.overlap.get();
        let step = step(&self.mapping, overlap)?;
        let (order, counts) = (self.order, [self.x_count.get(), self.y_count.get()]);
        let planned = match order {
            GridOrder::Snake => Order::Snake,
            GridOrder::Raster => Order::Raster,
        };
        let planner = RegularGrid::new([start.x, start.y], step, counts, planned)
            .map_err(ActionError::handled)?;
        let dz = self.autofocus_dz.get();
        let stack = self.stack();
        let images = autofocus::counts(&stack)?;
        self.autofocus
            .looping_autofocus(dz, SweepStart::Centre)
            .await?;
        let settings = json!({
            "order": order,
            "overlap": overlap,
            "dx": step.dx,
            "dy": step.dy,
            "x_count": counts[0],
            "y_count": counts[1],
            "autofocus_dz": dz,
            "capture_mode": stack.capture_mode,
            "stack_images_to_save": images.save,
            "stack_min_images_to_test": images.test,
            "stack_dz": stack.dz,
        });
        Ok(Begun {
            settings,
            planner: Box::new(planner),
        })
    }

    async fn acquire(&self, folder: String) -> Result<Acquired, ActionError> {
        let dz = self.autofocus_dz.get();
        smart_stack(&self.autofocus, folder, self.stack(), dz).await
    }

    /// The stack settings.
    fn stack(&self) -> StackSettings {
        StackSettings {
            images_to_save: self.stack_images_to_save.get(),
            min_images_to_test: self.stack_min_images_to_test.get(),
            dz: self.stack_dz.get(),
            capture_mode: self.capture_mode.get(),
        }
    }
}

impl ScanWorkflow for ThingRef<GridWorkflow> {
    fn display_name(&self) -> String {
        ThingRef::thing(self).name().to_owned()
    }

    fn ready(&self) -> bool {
        ThingRef::thing(&ThingRef::thing(self).mapping)
            .calibrated()
            .is_some()
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

/// A grid workflow's settings panel.
fn panel(ui: &ThingUi) -> Vec<UiElement> {
    let mut panel = vec![
        ui.property("overlap", "Image overlap (0.1–0.7)")
            .step(0.05)
            .into(),
        ui.property("x_count", "Columns").into(),
        ui.property("y_count", "Rows").into(),
    ];
    panel.extend(stack_panel(ui));
    panel
}

/// The grid's step from `overlap`, through the camera–stage mapping, which
/// must be calibrated.
pub(crate) fn step(
    mapping: &ThingRef<CameraStageMapping>,
    overlap: f64,
) -> Result<Step, ActionError> {
    let (matrix, size) = (ThingRef::thing(mapping).calibrated())
        .ok_or_else(|| ActionError::handled("calibrate the camera–stage mapping first"))?;
    Step::from_overlap(&matrix, size, overlap).map_err(ActionError::handled)
}

/// A smart stack at a site, saving even without focus so every site has an
/// image. The planner is told of focus only if the stack found it.
pub(crate) async fn smart_stack(
    autofocus: &ThingRef<Autofocus>,
    folder: String,
    mut stack: StackSettings,
    autofocus_dz: i64,
) -> Result<Acquired, ActionError> {
    // Odd and consistent already: `begin` said if it had to change them.
    let counts = Counts::new(stack.min_images_to_test, stack.images_to_save);
    (stack.min_images_to_test, stack.images_to_save) = (counts.test as u32, counts.save as u32);
    let stacked = (autofocus.run_smart_stack(folder, stack, true, autofocus_dz)).await?;
    Ok(Acquired {
        images: stacked.saved.len() as u32,
        focus: stacked.success.then_some(stacked.sharpest_z),
    })
}

/// The settings panel's autofocus, capture and smart stack settings.
pub(crate) fn stack_panel(ui: &ThingUi) -> [UiElement; 5] {
    [
        ui.property("autofocus_dz", "Autofocus range (steps)")
            .into(),
        ui.property("capture_mode", "Capture mode").into(),
        ui.property("stack_images_to_save", "Images to save per site (odd)")
            .into(),
        ui.property("stack_min_images_to_test", "Images to test (odd, 3–9)")
            .into(),
        ui.property("stack_dz", "Stack step (steps)").into(),
    ]
}
