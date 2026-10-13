//! The `stage_measure` Thing (P30): measures how far the stage travels in x
//! and y by watching the image, and recentres the stage on that range. The
//! behaviour spec opens the P30 notes.

use microscope_core::mapping::{Matrix2, apply};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;

use crate::autofocus::{Autofocus, AutofocusActions, SweepStart};
use crate::hardware::{CameraApi, Position, StageApi};
use crate::mapping::{CameraStageMapping, MIN_CONFIDENCE, along, field_shift, fresh, gray};

/// A check step, as a share of the frame's shorter side: small enough for
/// the frames before and after it to overlap well.
const CHECK_SHARE: f64 = 0.15;
/// A big step, in check steps.
const BIG_STEP: i64 = 10;
/// The share of a check step the stage must move to be still moving.
const STILL_MOVING: f64 = 0.65;
/// The share of a check step the stage must move to be moving again.
const MOVING_AGAIN: f64 = 0.1;
/// The most check steps back from an end.
const MAX_STEPS_BACK: usize = 15;
/// The most big steps towards an end.
const MAX_BIG_STEPS: usize = 500;
/// The most steps aside, past fields without detail, while finding an end.
const MAX_SIDESTEPS: i64 = 10;
/// How far autofocus sweeps, in steps.
const FOCUS_DZ: i64 = 1000;

/// How far one axis travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AxisRange {
    /// Its lowest position, in steps.
    pub min: i64,
    /// Its highest position, in steps.
    pub max: i64,
    /// How far the focus moved in z at the lowest and highest positions,
    /// from where the test started, when autofocus could measure it.
    pub focus_shift: Option<[i64; 2]>,
}

/// How far the stage travels in x and y.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RangeOfMotion {
    /// In x.
    pub x: AxisRange,
    /// In y.
    pub y: AxisRange,
}

/// Measures the stage's range of motion, and recentres it.
#[derive(Thing)]
pub struct StageMeasure {
    /// The camera, whose frames show how far the stage really moved.
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    /// The stage it measures.
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
    /// The mapping, which turns the image's motion into stage steps.
    #[slot(default = "camera_stage_mapping")]
    mapping: Slot<CameraStageMapping>,
    /// Autofocus, if there is one: it keeps the image sharp, and measures
    /// the focus at the ends.
    #[slot]
    autofocus: OptSlot<Autofocus>,
    /// The range the last test measured, in the stage's steps.
    #[setting(default = None, readonly)]
    calibrated_range: Prop<Option<RangeOfMotion>>,
}

#[thing_impl]
impl StageMeasure {
    /// Finds both ends of x and of y by watching the image, keeps the range
    /// as `calibrated_range`, and returns it. The sample must have detail
    /// over the whole range.
    #[action]
    async fn perform_rom_test(&self) -> Result<RangeOfMotion, ActionError> {
        let probe = self.probe()?;
        let focus = self.focus().await?;
        let start = self.stage.position().await?;
        let range = RangeOfMotion {
            x: self.axis_range(&probe, start, focus, 0).await?,
            y: self.axis_range(&probe, start, focus, 1).await?,
        };
        self.calibrated_range
            .set(Some(range))
            .map_err(ActionError::handled)?;
        Ok(range)
    }

    /// Moves x and y to the middle of the measured range and makes that the
    /// stage's zero, shifting the range to match. Returns where the middle
    /// was.
    #[action]
    async fn perform_recentre(&self) -> Result<Position, ActionError> {
        let range = self
            .calibrated_range
            .get()
            .ok_or_else(|| ActionError::handled("measure the range of motion first"))?;
        let middle = |axis: AxisRange| (axis.min + axis.max).div_euclid(2);
        let z = self.stage.position().await?.z;
        let centre = Position::new(middle(range.x), middle(range.y), z);
        self.stage.move_absolute(centre).await?;
        self.stage.set_zero().await?;
        let shift = |axis: AxisRange, by: i64| AxisRange {
            min: axis.min - by,
            max: axis.max - by,
            ..axis
        };
        let centred = RangeOfMotion {
            x: shift(range.x, centre.x),
            y: shift(range.y, centre.y),
        };
        self.calibrated_range
            .set(Some(centred))
            .map_err(ActionError::handled)?;
        Ok(centre)
    }
}

/// What measuring needs: the mapping's matrix, and the check step and the
/// frame's shorter side, in stage steps.
struct Probe {
    matrix: Matrix2,
    check: i64,
    frame: i64,
}

impl StageMeasure {
    fn probe(&self) -> Result<Probe, ActionError> {
        let (matrix, size) = ThingRef::thing(&self.mapping)
            .calibrated()
            .ok_or_else(|| ActionError::handled("calibrate the camera–stage mapping first"))?;
        let (w, h) = self.camera.frame_size();
        if [w, h] != size {
            return Err(ActionError::handled(
                "the camera's frames aren't the size the camera–stage mapping was calibrated at",
            ));
        }
        let scale = (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0])
            .abs()
            .sqrt();
        let frame = f64::from(w.min(h)) * scale;
        Ok(Probe {
            matrix,
            check: (CHECK_SHARE * frame).round().max(1.0) as i64,
            frame: frame.round() as i64,
        })
    }

    /// Both ends of `axis` (0 for x, 1 for y), from `start`.
    async fn axis_range(
        &self,
        probe: &Probe,
        start: Position,
        focus: Option<i64>,
        axis: usize,
    ) -> Result<AxisRange, ActionError> {
        let (max, high) = self.find_end(probe, start, focus, axis, 1).await?;
        let (min, low) = self.find_end(probe, start, focus, axis, -1).await?;
        Ok(AxisRange {
            min,
            max,
            focus_shift: low.zip(high).map(|(low, high)| [low, high]),
        })
    }

    /// Finds the end of `axis` in `direction` (±1) from `start`, then goes
    /// back there: the end's position on the axis, and how far the focus
    /// moved in z there from `focus`. Where a check step can't be measured,
    /// it steps aside to a fresh field and carries on.
    async fn find_end(
        &self,
        probe: &Probe,
        start: Position,
        focus: Option<i64>,
        axis: usize,
        direction: i64,
    ) -> Result<(i64, Option<i64>), ActionError> {
        let stage = &**self.stage;
        let check = probe.check as f64;
        let (mut big_steps, mut sidesteps) = (0, 0);
        let end = 'search: loop {
            loop {
                // A move the stage refuses at its end isn't an error: the image decides.
                let _ = stage
                    .move_relative(along(axis, direction * BIG_STEP * probe.check))
                    .await;
                match self
                    .measured_move(probe, axis, direction * probe.check)
                    .await?
                {
                    Some(moved) if moved * (direction as f64) < STILL_MOVING * check => break,
                    Some(_) => {}
                    None => self.sidestep(probe, axis, &mut sidesteps).await?,
                }
                big_steps += 1;
                if big_steps == MAX_BIG_STEPS {
                    let here = stage.position().await?;
                    return Err(ActionError::handled(format!(
                        "no end of travel within {MAX_BIG_STEPS} big steps, at {}",
                        at(here)
                    )));
                }
            }
            // Back until it moves again: the end is where that step started from.
            for _ in 0..MAX_STEPS_BACK {
                match self
                    .measured_move(probe, axis, -direction * probe.check)
                    .await?
                {
                    Some(moved) if -moved * (direction as f64) > MOVING_AGAIN * check => {
                        let here = stage.position().await?.to_array()[axis];
                        break 'search here - moved.round() as i64;
                    }
                    Some(_) => {}
                    // From a fresh field: back to the end, and step back again.
                    None => {
                        self.sidestep(probe, axis, &mut sidesteps).await?;
                        continue 'search;
                    }
                }
            }
            let here = stage.position().await?;
            return Err(ActionError::handled(format!(
                "the stage didn't move again after reaching the end of its travel, at {}",
                at(here)
            )));
        };
        let shift = self.focus().await?.zip(focus).map(|(z, z0)| z - z0);
        stage.move_absolute(start).await?;
        Ok((end, shift))
    }

    /// Moves `steps` along `axis`, and measures from the image how far the
    /// stage really moved along it. A registration is trusted if it's sure,
    /// or if it found just the move asked for. If it isn't, the move is made
    /// again once, after autofocusing; if that isn't either, it's `None`.
    async fn measured_move(
        &self,
        probe: &Probe,
        axis: usize,
        steps: i64,
    ) -> Result<Option<f64>, ActionError> {
        let camera = &**self.camera;
        for attempt in 0..2 {
            if attempt == 1 && self.focus().await?.is_none() {
                break;
            }
            let before = gray(&fresh(camera).await?);
            let _ = self.stage.move_relative(along(axis, steps)).await;
            let found = field_shift(&before, &gray(&fresh(camera).await?));
            if let Some(moved) = trusted(probe, axis, steps, found) {
                return Ok(Some(moved));
            }
        }
        Ok(None)
    }

    /// Steps across `axis` to a field that may have more detail: a frame to
    /// one side of where the first step aside began, then a frame to the
    /// other, then two frames, and so on. After `MAX_SIDESTEPS`, it's an
    /// error.
    async fn sidestep(
        &self,
        probe: &Probe,
        axis: usize,
        sidesteps: &mut i64,
    ) -> Result<(), ActionError> {
        if *sidesteps == MAX_SIDESTEPS {
            let here = self.stage.position().await?;
            return Err(ActionError::handled(format!(
                "the image has too little detail to track at {}: the sample must cover the whole range, in focus",
                at(here)
            )));
        }
        // The k-th step aside ends 1, −1, 2, −2, … frames from the start.
        let offset = |k: i64| if k % 2 == 1 { (k + 1) / 2 } else { -k / 2 };
        *sidesteps += 1;
        let frames = offset(*sidesteps) - offset(*sidesteps - 1);
        let _ = self
            .stage
            .move_relative(along(1 - axis, frames * probe.frame))
            .await;
        Ok(())
    }

    /// Autofocuses, if there's autofocus: the focus's z.
    async fn focus(&self) -> Result<Option<i64>, ActionError> {
        let Some(autofocus) = self.autofocus.get() else {
            return Ok(None);
        };
        AutofocusActions::looping_autofocus(autofocus, FOCUS_DZ, SweepStart::Centre).await?;
        Ok(Some(self.stage.position().await?.z))
    }
}

/// Where the stage is in x and y, for an error.
fn at(here: Position) -> String {
    format!("x {}, y {}", here.x, here.y)
}

/// How far the stage moved along `axis`, in steps, from a registration of a
/// move of `steps`: if the registration is sure, or found just that move, to
/// within a fifth of a check step.
fn trusted(
    probe: &Probe,
    axis: usize,
    steps: i64,
    (shift, confidence): ([f64; 2], f64),
) -> Option<f64> {
    let moved = apply(&probe.matrix, shift);
    let near = 0.2 * probe.check as f64;
    let asked = (moved[axis] - steps as f64).abs() < near && moved[1 - axis].abs() < near;
    (confidence >= MIN_CONFIDENCE || asked).then_some(moved[axis])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registration_is_trusted_if_sure_or_if_it_found_the_move_asked_for() {
        let probe = Probe {
            matrix: [[2.0, 0.0], [0.0, 2.0]],
            check: 100,
            frame: 600,
        };
        assert_eq!(trusted(&probe, 0, 100, ([30.0, 0.0], 0.9)), Some(60.0));
        assert_eq!(trusted(&probe, 0, 100, ([30.0, 0.0], 0.1)), None);
        assert_eq!(trusted(&probe, 0, 100, ([45.0, 5.0], 0.1)), Some(90.0));
        assert_eq!(trusted(&probe, 1, -100, ([0.0, -45.0], 0.1)), Some(-90.0));
        assert_eq!(trusted(&probe, 0, 100, ([45.0, 15.0], 0.1)), None);
    }
}
