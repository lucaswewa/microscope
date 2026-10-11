//! The `camera_stage_mapping` Thing: how stage moves move the image, so
//! that a move can be asked for in pixels (P23; the behaviour spec opens the
//! P23 notes).

use std::path::PathBuf;
use std::sync::Mutex;

use image::RgbImage;
use microscope_core::mapping::{Matrix2, apply, fit, invert, run_backlash};
use microscope_core::registration::{Gray, Options, register};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;

use crate::data;
use crate::hardware::{CameraApi, Position, StageApi};

/// Runs of moves on each axis, alternating direction.
const RUNS: usize = 4;
/// Moves in each run.
const MOVES_PER_RUN: usize = 5;
/// Below this, a registration can't be trusted.
const MIN_CONFIDENCE: f64 = 0.3;
/// The fewest moves on each axis the fit can use.
const MIN_KEPT_MOVES: usize = MOVES_PER_RUN;
/// The largest RMS residual of the fit, as a share of the mean stage move.
const MAX_FIT_ERROR: f64 = 0.1;

/// One measured move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeasuredMove {
    /// The stage's move, in steps (x, y).
    pub stage: [i64; 2],
    /// The field of view's move, in pixels (right, down).
    pub image: [f64; 2],
    /// How far the registration that measured it can be trusted (0–1).
    pub confidence: f64,
    /// Whether the fit used it: moves it couldn't track, and moves taking
    /// up backlash, are left out.
    pub kept: bool,
}

/// What a calibration measured and found, as saved in the data folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Calibration {
    /// When it was made (UTC), which names its file.
    pub timestamp: String,
    /// The frames' size: (width, height).
    pub image_resolution: [u32; 2],
    /// The image-to-stage displacement matrix, unless the fit failed.
    pub matrix: Option<Matrix2>,
    /// The backlash it estimated, in steps.
    pub backlash_steps: f64,
    /// Every move it made.
    pub moves: Vec<MeasuredMove>,
}

/// A stage displacement, in steps.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StageXy {
    /// In x.
    pub x: i64,
    /// In y.
    pub y: i64,
}

/// An image displacement, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImageXy {
    /// Rightwards.
    pub x: f64,
    /// Downwards.
    pub y: f64,
}

/// Calibrates and uses the relationship between stage moves and the image.
#[derive(Thing)]
pub struct CameraStageMapping {
    /// The camera, whose frames are registered.
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    /// The stage, which is moved.
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
    /// The image-to-stage displacement matrix, by rows: moving the stage by
    /// A·(dx, dy) steps moves the field of view by (dx, dy) pixels.
    #[setting(default = None, readonly)]
    image_to_stage_displacement_matrix: Prop<Option<Matrix2>>,
    /// The frames' size when it was calibrated: (width, height).
    #[setting(default = None, readonly)]
    image_resolution: Prop<Option<[u32; 2]>>,
    /// When it was last calibrated, which names the saved data.
    #[setting(default = None, readonly)]
    last_cal_timestamp: Prop<Option<String>>,
    data_folder: Mutex<Option<PathBuf>>,
}

#[thing_impl]
impl CameraStageMapping {
    /// The data saved by the last calibration, if any.
    #[property]
    async fn last_calibration(&self) -> Option<Calibration> {
        let path = self.saved(&self.last_cal_timestamp.get()?)?;
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    /// Whether it needs calibrating: until it has a matrix.
    #[property]
    async fn calibration_required(&self) -> bool {
        self.image_to_stage_displacement_matrix.get().is_none()
    }

    /// Whether it can be calibrated.
    #[property]
    async fn can_calibrate(&self) -> bool {
        true
    }

    /// Measures how stage moves move the image, and keeps the matrix.
    #[action]
    async fn calibrate_xy(&self) -> Result<Calibration, ActionError> {
        let stage = &**self.stage;
        let start = stage.position().await?;
        let measured = self.measure(start).await;
        // Back where it began, whatever happened.
        stage.move_absolute(start).await?;
        let (resolution, moves, backlash_steps) = measured?;

        let pairs: Vec<([f64; 2], [f64; 2])> = moves
            .iter()
            .filter(|m| m.kept)
            .map(|m| (m.image, m.stage.map(|s| s as f64)))
            .collect();
        let matrix = fit(&pairs, MAX_FIT_ERROR);
        let calibration = Calibration {
            timestamp: chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ").to_string(),
            image_resolution: resolution,
            matrix,
            backlash_steps,
            moves,
        };
        self.save(&calibration)?;
        let Some(matrix) = matrix else {
            return Err(ActionError::handled(
                "the stage didn't move as expected: check it moves freely",
            ));
        };
        let stored = [
            self.image_to_stage_displacement_matrix.set(Some(matrix)),
            self.image_resolution.set(Some(resolution)),
            self.last_cal_timestamp
                .set(Some(calibration.timestamp.clone())),
        ];
        stored
            .into_iter()
            .collect::<Result<(), _>>()
            .map_err(ActionError::handled)?;
        Ok(calibration)
    }

    /// Moves the field of view by (`x`, `y`) pixels: the point that far from
    /// the image's centre comes to the centre.
    #[action]
    async fn move_in_image_coordinates(&self, x: f64, y: f64) -> Result<Position, ActionError> {
        let steps = self.to_stage(x, y)?;
        self.stage
            .move_relative(Position::new(steps.x, steps.y, 0))
            .await
    }

    /// The stage move, in steps, that moves the field of view by (`x`, `y`)
    /// pixels.
    #[action]
    async fn convert_image_to_stage_coordinates(
        &self,
        x: f64,
        y: f64,
    ) -> Result<StageXy, ActionError> {
        self.to_stage(x, y)
    }

    /// How far the field of view moves, in pixels, when the stage moves by
    /// (`x`, `y`) steps.
    #[action]
    async fn convert_stage_to_image_coordinates(
        &self,
        x: i64,
        y: i64,
    ) -> Result<ImageXy, ActionError> {
        let inverse = invert(&self.matrix()?)
            .ok_or_else(|| ActionError::handled("the matrix can't be inverted"))?;
        let [x, y] = apply(&inverse, [x as f64, y as f64]);
        Ok(ImageXy { x, y })
    }

    /// Finds the data folder.
    #[on_start]
    async fn find_data_folder(&self, ctx: ThingCtx) {
        *self.data_folder.lock().expect("not poisoned") = Some(data::data_folder(ctx.server()));
    }
}

impl CameraStageMapping {
    /// The matrix and the frames' size it was calibrated at, once it's
    /// calibrated.
    pub fn calibrated(&self) -> Option<(Matrix2, [u32; 2])> {
        let matrix = self.image_to_stage_displacement_matrix.get()?;
        Some((matrix, self.image_resolution.get()?))
    }

    fn matrix(&self) -> Result<Matrix2, ActionError> {
        self.image_to_stage_displacement_matrix
            .get()
            .ok_or_else(|| ActionError::handled("camera-stage mapping isn't calibrated yet"))
    }

    fn to_stage(&self, x: f64, y: f64) -> Result<StageXy, ActionError> {
        let [sx, sy] = apply(&self.matrix()?, [x, y]);
        Ok(StageXy {
            x: sx.round() as i64,
            y: sy.round() as i64,
        })
    }

    /// The saved data's path for `timestamp`.
    fn saved(&self, timestamp: &str) -> Option<PathBuf> {
        let folder = self.data_folder.lock().expect("not poisoned").clone()?;
        Some(
            folder
                .join("calibration")
                .join(format!("camera_stage_mapping_{timestamp}.json")),
        )
    }

    fn save(&self, calibration: &Calibration) -> Result<(), ActionError> {
        let path = self
            .saved(&calibration.timestamp)
            .ok_or_else(|| ActionError::handled("there's no data folder"))?;
        let json = serde_json::to_vec_pretty(calibration).map_err(ActionError::handled)?;
        let saved = path.parent().map_or(Ok(()), std::fs::create_dir_all);
        saved
            .and_then(|()| std::fs::write(&path, json))
            .map_err(ActionError::handled)
    }

    /// Both axes' moves: the frames' size, every move, and the backlash.
    async fn measure(
        &self,
        start: Position,
    ) -> Result<([u32; 2], Vec<MeasuredMove>, f64), ActionError> {
        let (camera, stage) = (&**self.camera, &**self.stage);
        let first = fresh(camera).await?;
        let resolution = [first.width(), first.height()];
        let target = f64::from(resolution[0].min(resolution[1])) / 10.0;
        let (mut moves, mut backlashes) = (Vec::new(), Vec::new());
        for axis in 0..2 {
            let step = self.size_step(axis, target).await?;
            let mut kept_on_axis = 0;
            for run in 0..RUNS {
                let sign = if run % 2 == 0 { 1 } else { -1 };
                let stage_move = along(axis, sign * step);
                let mut previous = gray(&fresh(camera).await?);
                let mut measured = Vec::new();
                for _ in 0..MOVES_PER_RUN {
                    stage.move_relative(stage_move).await?;
                    let next = gray(&fresh(camera).await?);
                    measured.push(field_shift(&previous, &next));
                    previous = next;
                }
                let tracked: Vec<Option<[f64; 2]>> = measured
                    .iter()
                    .map(|(image, confidence)| (*confidence >= MIN_CONFIDENCE).then_some(*image))
                    .collect();
                // Each run after the first starts by reversing.
                let short = if run > 0 {
                    let backlash = run_backlash(&tracked, step as f64);
                    backlashes.extend(backlash.steps);
                    backlash.short
                } else {
                    vec![false; tracked.len()]
                };
                for ((image, confidence), short) in measured.into_iter().zip(short) {
                    let kept = confidence >= MIN_CONFIDENCE && !short;
                    kept_on_axis += usize::from(kept);
                    moves.push(MeasuredMove {
                        stage: [stage_move.x, stage_move.y],
                        image,
                        confidence,
                        kept,
                    });
                }
            }
            // Each axis is measured around the start.
            stage.move_absolute(start).await?;
            if kept_on_axis < MIN_KEPT_MOVES {
                return Err(too_little_detail());
            }
        }
        let backlash = backlashes.iter().sum::<f64>() / backlashes.len().max(1) as f64;
        Ok((resolution, moves, backlash))
    }

    /// Moves along `axis` by 1, 2, 4, … steps until the image has moved
    /// further than `target` pixels: the step that would move it `target`.
    /// A registration it can't trust ends the search early, and the last
    /// one that saw the image move sizes the step.
    async fn size_step(&self, axis: usize, target: f64) -> Result<i64, ActionError> {
        let (camera, stage) = (&**self.camera, &**self.stage);
        let before = gray(&fresh(camera).await?);
        let (mut moved, mut sized) = (0, None);
        for power in 0..15 {
            stage.move_relative(along(axis, 1 << power)).await?;
            moved += 1 << power;
            let (shift, confidence) = field_shift(&before, &gray(&fresh(camera).await?));
            if confidence < MIN_CONFIDENCE {
                return sized.ok_or_else(too_little_detail);
            }
            let distance = shift[0].hypot(shift[1]);
            let step = (moved as f64 * target / distance).round().max(1.0) as i64;
            if distance > target {
                return Ok(step);
            }
            if distance >= 1.0 {
                sized = Some(step);
            }
        }
        Err(ActionError::handled(
            "no motion was detected: check there's a sample in focus and the stage moves",
        ))
    }
}

/// A frame drawn wholly after the call.
async fn fresh(camera: &dyn CameraApi) -> Result<RgbImage, ActionError> {
    camera.grab_frame().await?;
    camera.grab_frame().await
}

fn gray(frame: &RgbImage) -> Gray {
    Gray::from_rgb8(
        frame.width() as usize,
        frame.height() as usize,
        frame.as_raw(),
    )
}

/// How far the field of view moved from `before` to `after`, in pixels
/// (against the content's shift), and the registration's confidence.
fn field_shift(before: &Gray, after: &Gray) -> ([f64; 2], f64) {
    let shift = register(before, after, &Options::default());
    ([-shift.dx, -shift.dy], shift.confidence)
}

fn too_little_detail() -> ActionError {
    ActionError::handled(
        "the image has too little detail to track: check there's a sample in focus",
    )
}

/// A move of `steps` along `axis` (0 for x, 1 for y).
fn along(axis: usize, steps: i64) -> Position {
    if axis == 0 {
        Position::new(steps, 0, 0)
    } else {
        Position::new(0, steps, 0)
    }
}
