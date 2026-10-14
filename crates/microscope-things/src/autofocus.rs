//! The `autofocus` Thing: OpenFlexure-style autofocus, driven by the live
//! stream (ADR-0022, and the behaviour spec in the P21 notes).
//!
//! Sharpness is the JPEG size of the camera's small preview frames: a sharp
//! image has more detail, which compresses less. While the stage moves in z,
//! a monitor records each frame's size and when it was taken, and samples
//! the stage's z every 20 ms. A frame's z is interpolated between the
//! samples either side of it.
//!
//! Stacks (P36) take images a set distance apart in z: a smart stack until
//! the sharpest is well inside its latest images, a basic stack a set
//! number. They save them in the data folder. The behaviour spec opens the
//! P36 notes.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use microscope_core::stack::{self, Counts, Verdict, judge};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;
use tokio::task::JoinHandle;

use crate::capture::{self, MappingRecord};
use crate::data;
use crate::hardware::{CameraApi, Capture, CaptureMode, Position, StageApi};

/// How often the monitor samples the stage's z while it moves.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(20);

/// How long to wait, after a move, for the frames taken during it.
const FRAME_WAIT: Duration = Duration::from_secs(1);

/// How many sweeps `looping_autofocus` makes before giving up.
const MAX_SWEEPS: u32 = 10;

/// How long a stack waits after each move.
const SETTLE: Duration = Duration::from_millis(50);
/// How many images below its window a smart stack starts.
const START_BELOW: i64 = 5;
/// How many images beyond its window a smart stack takes before giving up.
const MAX_EXTRA: usize = 15;
/// How many attempts a smart stack makes.
const ATTEMPTS: u32 = 3;

/// A frame's sharpness, and the z it was taken at.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FocusPoint {
    /// The stage's z, in steps, when the frame was taken.
    pub z: f64,
    /// The frame's JPEG size, in bytes: larger is sharper.
    pub sharpness: u64,
}

/// What a focus routine measured, and where it ended.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct FocusCurve {
    /// The frames taken while it measured, in order.
    pub points: Vec<FocusPoint>,
    /// The z it chose and moved to, if it chose one.
    pub focus_z: Option<i64>,
    /// How many sweeps it made.
    pub sweeps: u32,
}

/// Where a sweep starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SweepStart {
    /// Half the range below, so the sweep is centred on where the stage is.
    #[default]
    Centre,
    /// Where the stage is.
    Base,
}

/// A stack's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct StackSettings {
    /// How many images to save: odd, and no more than the images tested.
    pub images_to_save: u32,
    /// How many of the latest images are judged together: odd, 3 to 9.
    pub min_images_to_test: u32,
    /// The distance between images, in steps.
    pub dz: i64,
    /// How each image is captured.
    pub capture_mode: CaptureMode,
}

impl Default for StackSettings {
    fn default() -> Self {
        Self {
            images_to_save: 1,
            min_images_to_test: 9,
            dz: 50,
            capture_mode: CaptureMode::Standard,
        }
    }
}

/// Where a basic stack's images are, about the stage's z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StackOrigin {
    /// From it, upwards.
    Start,
    /// Centred on it.
    #[default]
    Centre,
    /// Ending at it.
    End,
}

/// What a smart stack did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StackResult {
    /// Whether it found focus.
    pub success: bool,
    /// Its sharpest image's z, in steps.
    pub sharpest_z: i64,
    /// The images it saved, by name, lowest first.
    pub saved: Vec<String>,
    /// How many attempts it made.
    pub attempts: u32,
}

/// An image of a stack, kept until it's saved.
struct Shot {
    capture: Capture,
    at: Position,
    sharpness: f64,
}

/// One attempt at a smart stack.
struct Attempt {
    /// Its latest images, lowest first.
    shots: VecDeque<Shot>,
    /// The sharpest's index, if it found focus.
    focused: Option<usize>,
    /// The z it started at.
    first_z: i64,
}

/// `stack`'s counts, made odd and consistent, logged if that changed them.
pub fn counts(stack: &StackSettings) -> Result<Counts, ActionError> {
    if stack.dz <= 0 {
        return Err(ActionError::handled(
            "dz must be a positive number of steps",
        ));
    }
    let (test, save) = (stack.min_images_to_test, stack.images_to_save);
    let counts = Counts::new(test, save);
    if (counts.test, counts.save) != (test as usize, save as usize) {
        tracing::warn!(
            "testing {} images and saving {}, not {test} and {save}: both must be odd, \
             and no more saved than tested",
            counts.test,
            counts.save
        );
    }
    Ok(counts)
}

/// `folder` in the data folder, made if it's missing.
fn stack_folder(server: &Server, folder: &str) -> Result<PathBuf, ActionError> {
    let folder = data::resolve(&data::data_folder(server), folder);
    let folder = folder.map_err(ActionError::handled)?;
    std::fs::create_dir_all(&folder).map_err(ActionError::handled)?;
    Ok(folder)
}

/// Saves `capture` in `folder`, named for where it was taken. Returns the
/// name.
fn save(
    folder: &Path,
    capture: &Capture,
    at: Position,
    mapping: Option<MappingRecord>,
) -> Result<String, ActionError> {
    let name = format!("img_{}_{}_{}.jpeg", at.x, at.y, at.z);
    capture::save_to(&folder.join(&name), capture, mapping)?;
    Ok(name)
}

/// The JPEG size of the next preview frame taken from now on.
async fn next_sharpness(stream: &MjpegStream) -> Result<f64, ActionError> {
    let (after, mut frames) = (Utc::now(), stream.frames());
    let next = async {
        while let Some(frame) = frames.next().await {
            if frame.timestamp > after {
                return Some(frame.data.len() as f64);
            }
        }
        None
    };
    match tokio::time::timeout(FRAME_WAIT, next).await {
        Ok(Some(size)) => Ok(size),
        _ => Err(ActionError::handled(
            "no preview frame arrived: is the camera streaming?",
        )),
    }
}

/// When a frame was taken, and its JPEG size.
type Taken = (DateTime<Utc>, u64);

/// Watches sharpness while the stage moves (see the module's description).
struct Monitor {
    frames: Arc<Mutex<Vec<Taken>>>,
    task: JoinHandle<()>,
    /// The stage's z over time.
    track: Vec<(DateTime<Utc>, f64)>,
}

impl Monitor {
    fn start(stream: MjpegStream) -> Self {
        let frames = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&frames);
        let task = tokio::spawn(async move {
            let mut receiver = stream.frames();
            while let Some(frame) = receiver.next().await {
                let size = frame.data.len() as u64;
                seen.lock()
                    .expect("not poisoned")
                    .push((frame.timestamp, size));
            }
        });
        Self {
            frames,
            task,
            track: Vec::new(),
        }
    }

    /// Moves z by `dz`, sampling z as it goes. Returns when the move began
    /// and ended.
    async fn move_z(
        &mut self,
        stage: &dyn StageApi,
        dz: i64,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), ActionError> {
        let began = self.sample(stage).await?;
        let moving = stage.move_relative(Position::new(0, 0, dz));
        tokio::pin!(moving);
        loop {
            tokio::select! {
                moved = &mut moving => {
                    moved?;
                    break;
                }
                () = tokio::time::sleep(SAMPLE_INTERVAL) => {
                    self.sample(stage).await?;
                }
            }
        }
        Ok((began, self.sample(stage).await?))
    }

    async fn sample(&mut self, stage: &dyn StageApi) -> Result<DateTime<Utc>, ActionError> {
        let now = Utc::now();
        let z = stage.position().await?.z as f64;
        self.track.push((now, z));
        Ok(now)
    }

    /// The frames taken between `began` and `ended`, at their z. A frame
    /// arrives a little after it is taken, once drawn and encoded, so this
    /// first waits for one taken after `ended` (or a second, at most).
    async fn curve(&self, (began, ended): (DateTime<Utc>, DateTime<Utc>)) -> Vec<FocusPoint> {
        let waiting = tokio::time::Instant::now();
        while waiting.elapsed() < FRAME_WAIT {
            let last = self
                .frames
                .lock()
                .expect("not poisoned")
                .last()
                .map(|f| f.0);
            if last.is_some_and(|taken| taken > ended) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let frames = self.frames.lock().expect("not poisoned");
        frames
            .iter()
            .filter(|(taken, _)| (began..=ended).contains(taken))
            .map(|&(taken, sharpness)| FocusPoint {
                z: self.z_at(taken),
                sharpness,
            })
            .collect()
    }

    /// The stage's z at `time`, interpolated between the samples either side.
    fn z_at(&self, time: DateTime<Utc>) -> f64 {
        let next = self.track.partition_point(|(sampled, _)| *sampled <= time);
        let before = next.checked_sub(1).and_then(|i| self.track.get(i));
        match (before, self.track.get(next)) {
            (Some(&(t0, z0)), Some(&(t1, z1))) => {
                z0 + (z1 - z0) * (seconds(time - t0) / seconds(t1 - t0))
            }
            (Some(&(_, z)), None) | (None, Some(&(_, z))) => z,
            (None, None) => 0.0,
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn seconds(span: TimeDelta) -> f64 {
    span.num_microseconds().unwrap_or(i64::MAX) as f64 / 1e6
}

/// OpenFlexure-style autofocus: it sweeps the stage through focus while
/// watching the live stream's sharpness.
#[derive(Thing)]
pub struct Autofocus {
    /// The camera, whose small preview stream measures sharpness.
    #[slot(default = "camera")]
    camera: Slot<dyn CameraApi>,
    /// The stage, which focuses.
    #[slot(default = "stage")]
    stage: Slot<dyn StageApi>,
}

#[thing_impl]
impl Autofocus {
    /// Sweeps `dz` steps up through focus, then moves to the sharpest point,
    /// approaching it from below as the sweep did. With `start` "centre", the
    /// sweep is centred on where the stage is.
    #[action]
    async fn fast_autofocus(
        &self,
        #[param(default = 2000)] dz: i64,
        #[param(default)] start: SweepStart,
    ) -> Result<FocusCurve, ActionError> {
        let mut monitor = Monitor::start(self.camera.lores_stream());
        self.sweep(&mut monitor, dz, start).await
    }

    /// Autofocuses until the sharpest point is in the middle three fifths of
    /// a sweep: up to 10 sweeps, each centred on the last one's focus.
    /// Returns the last sweep.
    #[action]
    async fn looping_autofocus(
        &self,
        #[param(default = 2000)] dz: i64,
        #[param(default)] start: SweepStart,
    ) -> Result<FocusCurve, ActionError> {
        let focused = self.looping(dz, start).await?;
        focused.ok_or_else(|| {
            ActionError::handled(format!("no focus found within {MAX_SWEEPS} sweeps"))
        })
    }

    /// Takes images `dz` apart, upwards through focus, until the sharpest is
    /// well inside the latest `min_images_to_test`, then saves
    /// `images_to_save` about it in `folder`, a folder in the data folder.
    /// When focus is out of reach, it autofocuses over `autofocus_dz` and
    /// tries again, 3 times in all. With `save_on_failure`, it saves the
    /// images about the sharpest even without focus.
    #[action]
    async fn run_smart_stack(
        &self,
        ctx: ActionCtx,
        folder: String,
        #[param(default)] stack: StackSettings,
        #[param(default)] save_on_failure: bool,
        #[param(default = 2000)] autofocus_dz: i64,
    ) -> Result<StackResult, ActionError> {
        let folder = stack_folder(ctx.server(), &folder)?;
        let counts = counts(&stack)?;
        let mut attempts = 1;
        let (shots, sharpest, success) = loop {
            let attempt = self.attempt(&ctx, counts, &stack).await?;
            if let Some(sharpest) = attempt.focused {
                break (attempt.shots, sharpest, true);
            }
            let shots = attempt.shots;
            let sharpness: Vec<f64> = shots.iter().map(|shot| shot.sharpness).collect();
            let sharpest = stack::sharpest(&sharpness);
            if attempts == ATTEMPTS {
                break (shots, sharpest, false);
            }
            tracing::info!("attempt {attempts} found no focus, so autofocusing to try again");
            let here = self.stage.position().await?;
            let back = Position::new(0, 0, attempt.first_z - here.z);
            self.stage.move_relative(back).await?;
            let refocused = self.looping(autofocus_dz, SweepStart::Centre).await?;
            if refocused.is_none() {
                tracing::warn!("autofocus found no focus, so the stack stops trying");
                break (shots, sharpest, false);
            }
            attempts += 1;
        };
        let mut saved = Vec::new();
        if success || save_on_failure {
            let mapping = capture::mapping(ctx.server());
            for shot in shots.range(counts.to_save(sharpest, shots.len())) {
                saved.push(save(&folder, &shot.capture, shot.at, mapping.clone())?);
            }
        }
        Ok(StackResult {
            success,
            sharpest_z: shots.get(sharpest).map_or(0, |shot| shot.at.z),
            saved,
            attempts,
        })
    }

    /// Takes `images_to_save` images `dz` apart, upwards, from, centred on
    /// or ending at where the stage is, by `origin`, and saves them all in
    /// `folder`, a folder in the data folder. Returns their z positions.
    #[action]
    async fn run_basic_stack(
        &self,
        ctx: ActionCtx,
        folder: String,
        #[param(default)] stack: StackSettings,
        #[param(default)] origin: StackOrigin,
    ) -> Result<Vec<i64>, ActionError> {
        let folder = stack_folder(ctx.server(), &folder)?;
        let (n, dz) = (counts(&stack)?.save as i64, stack.dz);
        let below = match origin {
            StackOrigin::Start => 0,
            StackOrigin::Centre => (n - 1) * dz / 2,
            StackOrigin::End => (n - 1) * dz,
        };
        self.stage
            .move_relative(Position::new(0, 0, -below))
            .await?;
        let mapping = capture::mapping(ctx.server());
        let mut zs = Vec::new();
        for i in 0..n {
            if i > 0 {
                self.stage.move_relative(Position::new(0, 0, dz)).await?;
            }
            let (capture, at) = self.shot(&ctx, stack.capture_mode).await?;
            save(&folder, &capture, at, mapping.clone())?;
            zs.push(at.z);
        }
        Ok(zs)
    }

    /// Makes each move in `dz` in z, waiting `wait` seconds between them,
    /// and measures sharpness throughout.
    #[action]
    async fn z_move_and_measure_sharpness(
        &self,
        ctx: ActionCtx,
        dz: Vec<i64>,
        #[param(default)] wait: f64,
    ) -> Result<FocusCurve, ActionError> {
        let mut monitor = Monitor::start(self.camera.lores_stream());
        let mut points = Vec::new();
        for (i, step) in dz.into_iter().enumerate() {
            if i > 0 && wait > 0.0 {
                ctx.sleep(Duration::from_secs_f64(wait)).await?;
            }
            let span = monitor.move_z(&**self.stage, step).await?;
            points.extend(monitor.curve(span).await);
        }
        Ok(FocusCurve {
            points,
            ..FocusCurve::default()
        })
    }
}

impl Autofocus {
    /// `looping_autofocus`'s sweeps: the last, or none if it found no focus.
    async fn looping(&self, dz: i64, start: SweepStart) -> Result<Option<FocusCurve>, ActionError> {
        let mut monitor = Monitor::start(self.camera.lores_stream());
        let mut start = start;
        for sweeps in 1..=MAX_SWEEPS {
            let mut curve = self.sweep(&mut monitor, dz, start).await?;
            curve.sweeps = sweeps;
            let zs = curve.points.iter().map(|point| point.z);
            let low = zs.clone().fold(f64::INFINITY, f64::min);
            let high = zs.fold(f64::NEG_INFINITY, f64::max);
            let margin = dz as f64 / 5.0;
            let focus = curve.focus_z.unwrap_or_default() as f64;
            if low + margin < focus && focus < high - margin {
                return Ok(Some(curve));
            }
            start = SweepStart::Centre;
        }
        Ok(None)
    }

    /// One attempt at a smart stack, from below where the stage is.
    async fn attempt(
        &self,
        ctx: &ActionCtx,
        counts: Counts,
        stack: &StackSettings,
    ) -> Result<Attempt, ActionError> {
        let (stage, dz) = (&**self.stage, stack.dz);
        let (n, side) = (counts.test, counts.each_side());
        let below = START_BELOW * dz + (n as i64 - 1) * dz / 2;
        let first_z = stage.move_relative(Position::new(0, 0, -below)).await?.z;
        let mut shots = VecDeque::new();
        let mut focused = None;
        for taken in 1..=n + MAX_EXTRA {
            if taken > 1 {
                stage.move_relative(Position::new(0, 0, dz)).await?;
            }
            shots.push_back(self.judged_shot(ctx, stack.capture_mode).await?);
            // Only the images it might save: the window, and those below it.
            if shots.len() > n + side {
                shots.pop_front();
            }
            if taken < n {
                continue;
            }
            let window = shots.range(shots.len() - n..).map(|shot| shot.sharpness);
            match judge(&window.collect::<Vec<_>>()) {
                Verdict::Continue => continue,
                Verdict::Restart => break,
                Verdict::Focused(i) => {
                    let sharpest = shots.len() - n + i;
                    // Enough above it to save.
                    while shots.len() - 1 - sharpest < side {
                        stage.move_relative(Position::new(0, 0, dz)).await?;
                        shots.push_back(self.judged_shot(ctx, stack.capture_mode).await?);
                    }
                    focused = Some(sharpest);
                    break;
                }
            }
        }
        Ok(Attempt {
            shots,
            focused,
            first_z,
        })
    }

    /// Settles, then captures an image, with where it was taken.
    async fn shot(
        &self,
        ctx: &ActionCtx,
        mode: CaptureMode,
    ) -> Result<(Capture, Position), ActionError> {
        ctx.sleep(SETTLE).await?;
        let at = self.stage.position().await?;
        Ok((self.camera.capture_to_memory(mode).await?, at))
    }

    /// A shot, and its sharpness from the next preview frame.
    async fn judged_shot(&self, ctx: &ActionCtx, mode: CaptureMode) -> Result<Shot, ActionError> {
        let (capture, at) = self.shot(ctx, mode).await?;
        let sharpness = next_sharpness(&self.camera.lores_stream()).await?;
        Ok(Shot {
            capture,
            at,
            sharpness,
        })
    }

    /// One sweep up through focus, then the move to its sharpest point.
    async fn sweep(
        &self,
        monitor: &mut Monitor,
        dz: i64,
        start: SweepStart,
    ) -> Result<FocusCurve, ActionError> {
        if dz <= 0 {
            return Err(ActionError::handled(
                "dz must be a positive number of steps",
            ));
        }
        let stage = &**self.stage;
        if start == SweepStart::Centre {
            stage.move_relative(Position::new(0, 0, -dz / 2)).await?;
        }
        let span = monitor.move_z(stage, dz).await?;
        let points = monitor.curve(span).await;
        let Some(sharpest) = points.iter().max_by_key(|point| point.sharpness) else {
            return Err(ActionError::handled(
                "no frames arrived during the sweep: is the camera streaming?",
            ));
        };
        let focus_z = sharpest.z.round() as i64;
        // Back below the range, then up to the focus: approached from below,
        // as in the sweep, the stage's backlash lies the same way.
        let base = stage.move_relative(Position::new(0, 0, -dz)).await?;
        stage
            .move_relative(Position::new(0, 0, focus_z - base.z))
            .await?;
        Ok(FocusCurve {
            points,
            focus_z: Some(focus_z),
            sweeps: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest bytes a stream accepts as a JPEG.
    const JPEG: [u8; 4] = [0xff, 0xd8, 0xff, 0xd9];

    #[tokio::test]
    async fn frames_get_interpolated_z_and_late_frames_are_waited_for() {
        let stream = MjpegStream::new();
        let mut monitor = Monitor::start(stream.clone());
        let t0 = Utc::now();
        let at = move |ms: i64| t0 + TimeDelta::milliseconds(ms);
        // The stage moved from 0 to 100 steps in 100 ms.
        monitor.track = vec![(t0, 0.0), (at(100), 100.0)];
        tokio::time::sleep(Duration::from_millis(20)).await;
        stream.add_frame_at(JPEG.to_vec(), at(25)).expect("a frame");
        // A frame taken during the move arrives after it, then one taken after it.
        let later = stream.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(60)).await;
            later.add_frame_at(JPEG.to_vec(), at(90)).expect("a frame");
            tokio::time::sleep(Duration::from_millis(30)).await;
            later.add_frame_at(JPEG.to_vec(), at(150)).expect("a frame");
        });
        let curve = monitor.curve((t0, at(100))).await;
        let zs: Vec<f64> = curve.iter().map(|point| point.z.round()).collect();
        assert_eq!(zs, [25.0, 90.0]);
        assert!(curve.iter().all(|point| point.sharpness == 4));
    }

    #[tokio::test]
    async fn a_stacks_sharpness_is_from_a_frame_taken_after_it_asks() {
        let stream = MjpegStream::new();
        let later = stream.clone();
        let asked = Utc::now();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            // Taken before it asked, though it arrives after.
            let before = asked - TimeDelta::milliseconds(50);
            later.add_frame_at(JPEG.to_vec(), before).expect("a frame");
            tokio::time::sleep(Duration::from_millis(20)).await;
            let after = [0xff, 0xd8, 0, 0xff, 0xd9];
            later
                .add_frame_at(after.to_vec(), Utc::now())
                .expect("a frame");
        });
        assert_eq!(next_sharpness(&stream).await.expect("a frame"), 5.0);
    }
}
