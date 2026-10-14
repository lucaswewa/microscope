//! The histo workflow (P37): a smart spiral (P34) out from the start over a
//! tissue section, skipping background, with a smart stack (P36) at each
//! site. The behaviour spec opens the P37 notes.

use std::sync::Arc;

use image::{DynamicImage, RgbImage};
use microscope_core::scan::SmartSpiral;
use serde_json::json;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

use crate::autofocus::{self, Autofocus, AutofocusActions, StackSettings, SweepStart};
use crate::camera::SampleCheck;
use crate::hardware::{BackgroundDetector, CameraApi, CaptureMode, LORES_SIZE, Position};
use crate::mapping::CameraStageMapping;
use crate::ui::{Layout, OwnEntry, Success, ThingUi, UiElement};
use crate::workflow::{self, Acquired, Begun, ScanWorkflow};

/// What the app calls it.
const NAME: &str = "Histo scan";

/// A scan of a tissue section: see the module's description.
#[derive(Thing)]
#[thing(interfaces(ScanWorkflow))]
pub struct HistoScanWorkflow {
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    #[slot(default = "autofocus")]
    autofocus: Slot<Autofocus>,
    #[slot(default = "camera_stage_mapping")]
    mapping: Slot<CameraStageMapping>,
    /// Tells sample from the empty slide.
    #[slot(default = "bg_channel_deviations_luv")]
    background_detector: Slot<dyn BackgroundDetector>,
    /// The share of each image that its neighbours overlap.
    #[setting(default = 0.35, ge = 0.1, le = 0.7)]
    overlap: Prop<f64>,
    /// The furthest a site may be from the start, in steps.
    #[setting(default = 25_000, ge = 1000)]
    max_range: Prop<i64>,
    /// Whether the steps in x and y are both the smaller of the two.
    #[setting(default = false)]
    equal_distances: Prop<bool>,
    /// Whether sites that look like background are skipped.
    #[setting(default = true)]
    skip_background: Prop<bool>,
    /// How far autofocus sweeps, in steps.
    #[setting(default = 1000, ge = 400, le = 3000)]
    autofocus_dz: Prop<i64>,
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
impl HistoScanWorkflow {
    /// What the app calls it.
    #[property]
    async fn display_name(&self) -> String {
        NAME.to_owned()
    }

    /// Whether it can start: once the camera–stage mapping is calibrated
    /// and, while it skips background, its detector has a background.
    #[property]
    async fn ready(&self) -> bool {
        self.is_ready()
    }

    /// Sets the background detector's background from a frame of what the
    /// camera sees now, which should be the empty slide.
    #[action]
    async fn set_background(&self) -> Result<(), ActionError> {
        let frame = self.frame().await?;
        self.background_detector.set_background(&frame)
    }

    /// Whether what the camera sees now holds sample, and how much of it.
    #[action]
    async fn check_background(&self) -> Result<SampleCheck, ActionError> {
        self.check().await
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

impl HistoScanWorkflow {
    fn is_ready(&self) -> bool {
        let mapped = ThingRef::thing(&self.mapping).calibrated().is_some();
        mapped && (!self.skip_background.get() || self.background_detector.ready())
    }

    async fn begin(&self, start: Position) -> Result<Begun, ActionError> {
        let overlap = self.overlap.get();
        let mut step = workflow::step(&self.mapping, overlap)?;
        let equal = self.equal_distances.get();
        if equal {
            step = step.equal();
        }
        let skip = self.skip_background.get();
        if skip && !self.check().await?.sample {
            return Err(ActionError::handled(
                "the start looks like background: move to the sample first",
            ));
        }
        let max_range = self.max_range.get();
        let planner =
            SmartSpiral::new([start.x, start.y], step, max_range).map_err(ActionError::handled)?;
        let dz = self.autofocus_dz.get();
        let stack = self.stack();
        let images = autofocus::counts(&stack)?;
        self.autofocus
            .looping_autofocus(dz, SweepStart::Centre)
            .await?;
        let settings = json!({
            "pattern": "smart_spiral",
            "overlap": overlap,
            "equal_distances": equal,
            "dx": step.dx,
            "dy": step.dy,
            "max_range": max_range,
            "skip_background": skip,
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

    /// Nothing where it looks like background, if skipping it; otherwise a
    /// smart stack.
    async fn acquire(&self, folder: String) -> Result<Acquired, ActionError> {
        if self.skip_background.get() {
            let check = self.check().await?;
            if !check.sample {
                tracing::info!("Skipped as background: {}", check.message);
                return Ok(Acquired {
                    images: 0,
                    focus: None,
                });
            }
        }
        let dz = self.autofocus_dz.get();
        workflow::smart_stack(&self.autofocus, folder, self.stack(), dz).await
    }

    /// How the background detector judges what the camera sees now.
    async fn check(&self) -> Result<SampleCheck, ActionError> {
        SampleCheck::of(&**self.background_detector, &self.frame().await?)
    }

    /// A frame drawn wholly after the call, at the low-resolution stream's
    /// size, as the camera's own background actions take it.
    async fn frame(&self) -> Result<RgbImage, ActionError> {
        // The next frame may have been begun before the call; the one after
        // it wasn't.
        self.camera.grab_frame().await?;
        let frame = self.camera.grab_frame().await?;
        let (width, height) = LORES_SIZE;
        let small = DynamicImage::ImageRgb8(frame).thumbnail_exact(width, height);
        Ok(small.into_rgb8())
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

impl ScanWorkflow for ThingRef<HistoScanWorkflow> {
    fn display_name(&self) -> String {
        NAME.to_owned()
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

/// The histo workflow's panel: background detection, then the settings.
fn panel(ui: &ThingUi) -> Vec<UiElement> {
    let set = ui.action("set_background", "Set background");
    let set = set.on_success(Success::Message("The background is set.".to_owned()));
    let check = ui.action("check_background", "Check background");
    let mut panel = vec![
        UiElement::Header {
            text: "Background detect".into(),
        },
        UiElement::Text {
            text: "Move to an empty part of the slide, set the background, then move back \
                   to the sample. Check background says how much sample the camera sees."
                .into(),
        },
        UiElement::Container {
            layout: Layout::Row,
            children: vec![
                set.refresh().into(),
                check.on_success(Success::Output).into(),
            ],
        },
        UiElement::Header {
            text: "Scan settings".into(),
        },
        ui.property("overlap", "Image overlap (0.1–0.7)")
            .step(0.05)
            .into(),
        ui.property("max_range", "Maximum range (steps)").into(),
        ui.property("equal_distances", "Equal distances in x and y")
            .into(),
        ui.property("skip_background", "Skip background").into(),
    ];
    panel.extend(workflow::stack_panel(ui));
    panel
}
