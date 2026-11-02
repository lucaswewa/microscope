//! Preloaded sequences of light states, stepped by a camera's exposure signal, so that frames need
//! no round trip to the PC.

use core::fmt::Write;

use crate::controller::Refused;
use crate::matrix::Pattern;
use crate::{CHANNELS, Line};

/// Enough for one LED at a time across the whole matrix.
pub const MAX_STEPS: usize = 64;
/// How long a step's light has to settle before the sequence triggers the camera. The matrix's
/// frame takes about 0.5 ms, and this leaves room for the LED drivers. The bench should measure it.
pub const LIGHT_SETTLE_US: u64 = 2_000;

/// One step's light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Light {
    pub leds: [bool; CHANNELS],
    pub matrix: Pattern,
}

/// What a running sequence has just decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Show { step: usize, light: Light },
    Finished,
}

#[derive(Clone, Copy, Debug)]
struct Run {
    camera: usize,
    cycles: u32,
    delay_us: u64,
    trigger_us: u64,
    next: usize,
    cycle: u32,
    advance_at_us: Option<u64>,
    trigger_at_us: Option<u64>,
}

/// The steps, and how far a run through them has got.
#[derive(Debug)]
pub struct Sequence {
    steps: [Light; MAX_STEPS],
    len: usize,
    run: Option<Run>,
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            steps: [Light::default(); MAX_STEPS],
            len: 0,
            run: None,
        }
    }
}

impl Sequence {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    /// Whether any step lights an epi LED, which the door's interlock applies to.
    pub fn uses_leds(&self) -> bool {
        self.steps[..self.len]
            .iter()
            .any(|step| step.leds.contains(&true))
    }

    /// Whether the sequence is the one triggering the camera.
    pub fn triggers(&self) -> bool {
        self.run.is_some_and(|run| run.trigger_us > 0)
    }

    pub(crate) fn add(&mut self, light: Light) -> Result<(), Refused> {
        let step = self.steps.get_mut(self.len).ok_or(Refused::Full)?;
        *step = light;
        self.len += 1;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Starts at the first step, which `poll` shows straight away.
    pub fn start(
        &mut self,
        camera: usize,
        cycles: u32,
        delay_us: u64,
        trigger_us: u64,
        now_us: u64,
    ) {
        self.run = Some(Run {
            camera,
            cycles,
            delay_us,
            trigger_us,
            next: 0,
            cycle: 1,
            advance_at_us: Some(now_us),
            trigger_at_us: None,
        });
    }

    pub fn stop(&mut self) {
        self.run = None;
    }

    /// The camera's exposure ended, so the next step comes after the delay, which gives the last
    /// rows time to read out. An end that comes while waiting changes nothing.
    pub fn exposure_ended(&mut self, camera: usize, now_us: u64) {
        if let Some(run) = &mut self.run
            && run.camera == camera
            && run.advance_at_us.is_none()
        {
            run.advance_at_us = Some(now_us + run.delay_us);
        }
    }

    /// Moves to the next step if it's due. After a run's last cycle, it finishes.
    pub fn poll(&mut self, now_us: u64) -> Option<Advance> {
        let run = self.run.as_mut()?;
        if run.advance_at_us.is_none_or(|at| now_us < at) {
            return None;
        }
        run.advance_at_us = None;
        if run.next == self.len {
            run.next = 0;
            run.cycle = run.cycle.saturating_add(1);
            if run.cycles != 0 && run.cycle > run.cycles {
                self.run = None;
                return Some(Advance::Finished);
            }
        }
        let step = run.next;
        run.next += 1;
        if run.trigger_us > 0 {
            run.trigger_at_us = Some(now_us + LIGHT_SETTLE_US);
        }
        Some(Advance::Show {
            step,
            light: self.steps[step],
        })
    }

    /// The camera and the width of a trigger pulse that's now due.
    pub fn take_trigger(&mut self, now_us: u64) -> Option<(usize, u64)> {
        let run = self.run.as_mut()?;
        if run.trigger_at_us.is_none_or(|at| now_us < at) {
            return None;
        }
        run.trigger_at_us = None;
        Some((run.camera, run.trigger_us))
    }

    pub fn deadline(&self) -> Option<u64> {
        let run = self.run.as_ref()?;
        [run.advance_at_us, run.trigger_at_us]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn write_status(&self, reply: &mut Line) {
        let _ = write!(reply, "ok steps={}", self.len);
        let _ = match &self.run {
            Some(run) => write!(
                reply,
                " state=running camera={} step={} cycle={}",
                run.camera + 1,
                run.next,
                run.cycle
            ),
            None => write!(reply, " state=idle"),
        };
    }
}
