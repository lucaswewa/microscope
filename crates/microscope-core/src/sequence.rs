//! Sequences (P41, ADR-0031): a plan of time points and the steps run at
//! each, and when the next is due. A point runs within its own interval or
//! not at all, so a sequence that falls behind skips points rather than
//! running them late. The behaviour spec opens the P41 notes.

use std::fmt;
use std::time::Duration;

/// A step run at each time point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// An autofocus over `dz` steps, centred on where z is.
    Autofocus {
        /// Its range, in steps.
        dz: i64,
    },
    /// A capture.
    Capture,
    /// Reserved for later: a z-stack of `images`, `dz` steps apart.
    ZStack {
        /// How many images.
        images: u32,
        /// The distance between them, in steps.
        dz: i64,
    },
}

/// What a sequence does: a time axis, and the steps at each point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencePlan {
    /// The time between points.
    pub interval: Duration,
    /// How long it runs: its points are within it.
    pub duration: Duration,
    /// The steps at each point, in order.
    pub steps: Vec<Step>,
    /// Reserved for later: the stage positions visited at each point, x, y
    /// and z. Empty, for where the stage is.
    pub positions: Vec<[i64; 3]>,
}

/// Why a plan can't run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    /// Its interval is 0.
    NoInterval,
    /// It has a z-stack or positions.
    Unsupported,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoInterval => "the interval must be more than 0",
            Self::Unsupported => "z-stacks and multiple positions aren't supported yet",
        })
    }
}

impl std::error::Error for PlanError {}

/// What a sequence does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// Wait this long for the next point.
    Wait(Duration),
    /// Run point `index` now, skipping the `skipped` before it, whose
    /// intervals have passed.
    Run {
        /// The point's number, from 0.
        index: u64,
        /// How many points before it are skipped.
        skipped: u64,
    },
    /// Every point has run or been skipped: the last `skipped` were skipped.
    Done {
        /// How many points at the end are skipped.
        skipped: u64,
    },
}

impl SequencePlan {
    /// A timelapse: a capture every `interval` for `duration`, after an
    /// autofocus over `autofocus_dz` steps if there's one.
    pub fn timelapse(
        interval: Duration,
        duration: Duration,
        autofocus_dz: Option<i64>,
    ) -> SequencePlan {
        let autofocus = autofocus_dz.map(|dz| Step::Autofocus { dz });
        SequencePlan {
            interval,
            duration,
            steps: autofocus.into_iter().chain([Step::Capture]).collect(),
            positions: Vec::new(),
        }
    }

    /// Refuses a plan this version can't run.
    pub fn check(&self) -> Result<(), PlanError> {
        if self.interval.is_zero() {
            return Err(PlanError::NoInterval);
        }
        let stack = (self.steps.iter()).any(|step| matches!(step, Step::ZStack { .. }));
        if stack || !self.positions.is_empty() {
            return Err(PlanError::Unsupported);
        }
        Ok(())
    }

    /// How many time points: one at the start, then one each interval
    /// within the duration.
    pub fn points(&self) -> u64 {
        let count = self.duration.as_nanos() / self.interval.as_nanos().max(1);
        u64::try_from(count).unwrap_or(u64::MAX).saturating_add(1)
    }

    /// When point `index` is due, from the start.
    pub fn at(&self, index: u64) -> Duration {
        let nanos = self.interval.as_nanos() * u128::from(index);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// What to do `elapsed` after the start, with `next` the first point not
    /// yet run or skipped.
    pub fn due(&self, elapsed: Duration, next: u64) -> Due {
        let points = self.points();
        if next >= points {
            return Due::Done { skipped: 0 };
        }
        let at = self.at(next);
        if elapsed < at {
            return Due::Wait(at - elapsed);
        }
        // The point whose interval `elapsed` is in.
        let current = elapsed.as_nanos() / self.interval.as_nanos().max(1);
        let current = u64::try_from(current).unwrap_or(u64::MAX);
        if current >= points {
            return Due::Done {
                skipped: points - next,
            };
        }
        Due::Run {
            index: current,
            skipped: current - next,
        }
    }
}
