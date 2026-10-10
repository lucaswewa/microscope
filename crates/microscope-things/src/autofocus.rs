//! The `autofocus` Thing: OpenFlexure-style autofocus, driven by the live
//! stream (ADR-0022, and the behaviour spec in the P21 notes).
//!
//! Sharpness is the JPEG size of the camera's small preview frames: a sharp
//! image has more detail, which compresses less. While the stage moves in z,
//! a monitor records each frame's size and when it was taken, and samples
//! the stage's z every 20 ms. A frame's z is interpolated between the
//! samples either side of it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use teta_wot::prelude::*;
use tokio::task::JoinHandle;

use crate::hardware::{CameraApi, Position, StageApi};

/// How often the monitor samples the stage's z while it moves.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(20);

/// How long to wait, after a move, for the frames taken during it.
const FRAME_WAIT: Duration = Duration::from_secs(1);

/// How many sweeps `looping_autofocus` makes before giving up.
const MAX_SWEEPS: u32 = 10;

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
                return Ok(curve);
            }
            start = SweepStart::Centre;
        }
        Err(ActionError::handled(format!(
            "no focus found within {MAX_SWEEPS} sweeps"
        )))
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
}
