//! The filter slider: a stepper on a lead screw, with a home switch near cell (c).

use core::fmt::Write;

use crate::Line;
use crate::controller::{Event, Refused};

/// A 2 mm lead (assumed, like the changer's), 200 full steps a turn and 4 microsteps a step.
pub const STEPS_PER_MM: u32 = 400;
/// Each cell's name and its nominal distance from the switch: the quad cube, open and the 50:50
/// plate, from the CAD's 40 mm pitch and the switch 1 mm past (c).
pub const CELLS: [(&str, u32); 3] = [
    ("a", 81 * STEPS_PER_MM),
    ("b", 41 * STEPS_PER_MM),
    ("c", STEPS_PER_MM),
];
/// Whether a high DIR moves away from the switch. Assumed: it depends on the motor's wiring.
pub const AWAY_IS_HIGH: bool = true;

/// The objective changer's speed and acceleration, on the same kind of drive.
const SPEED: u32 = 10 * STEPS_PER_MM;
const ACCELERATION: u32 = 50 * STEPS_PER_MM;
/// Homing is slower, because it stops dead on the switch.
const HOME_SPEED: u32 = 5 * STEPS_PER_MM;
/// More than the whole travel, so that a switch that never trips can't run on for ever.
const SEEK_STEPS: u32 = 90 * STEPS_PER_MM;
/// Further than a pressed switch can need to release. If it doesn't, its wire may be broken.
const LEAVE_STEPS: u32 = 5 * STEPS_PER_MM;
/// How far past the release to go, so that a bouncing switch can't count as a new press.
const CLEAR_STEPS: u32 = STEPS_PER_MM;
/// The driver needs a moment after its enable before it takes steps.
const ENABLE_SETTLE_US: u64 = 5_000;
/// The motor switches off this long after a move, as the changer's does: its screw doesn't
/// back-drive.
const OFF_AFTER_US: u64 = 250_000;
/// The most steps one `poll` hands over, when something has held up the control loop.
const MAX_BURST: u32 = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    /// Homing: waiting for the firmware to set up the driver.
    Driver,
    /// Homing: moving off a switch that was already pressed.
    Leaving,
    /// Homing: going a little further.
    Clearing,
    /// Homing: moving toward the switch until it trips.
    Seeking,
    Moving(usize),
}

/// A move's timing: speeding up, a steady run, then slowing down.
#[derive(Clone, Copy, Debug, Default)]
struct Ramp {
    steps: u32,
    speed: u32,
    ramp_steps: u32,
}

impl Ramp {
    fn new(steps: u32, speed: u32) -> Self {
        Self {
            steps,
            speed,
            ramp_steps: (speed * speed / (2 * ACCELERATION)).min(steps / 2),
        }
    }

    /// When step `n`, from 1 to `steps`, falls, in µs after the move starts.
    fn time_us(&self, n: u32) -> u64 {
        let ramped = ramp_time_us(self.ramp_steps);
        let steady = |steps: u32| u64::from(steps) * 1_000_000 / u64::from(self.speed);
        if n <= self.ramp_steps {
            ramp_time_us(n)
        } else if n <= self.steps - self.ramp_steps {
            ramped + steady(n - self.ramp_steps)
        } else {
            2 * ramped + steady(self.steps - 2 * self.ramp_steps) - ramp_time_us(self.steps - n)
        }
    }
}

/// How long it takes to cover `steps` from rest, at the acceleration: √(2 × steps / acceleration).
fn ramp_time_us(steps: u32) -> u64 {
    (2 * u64::from(steps) * 1_000_000_000_000 / u64::from(ACCELERATION)).isqrt()
}

/// When each step is due. Its time comes from the step's number, so a late `poll` catches up
/// instead of stretching the move.
#[derive(Clone, Copy, Debug, Default)]
struct Run {
    ramp: Ramp,
    start_us: u64,
    done: u32,
}

impl Run {
    fn next_us(&self) -> u64 {
        if self.done >= self.ramp.steps {
            self.start_us
        } else {
            self.start_us + self.ramp.time_us(self.done + 1)
        }
    }
}

#[derive(Debug, Default)]
pub struct Slider {
    phase: Phase,
    run: Run,
    away: bool,
    /// In steps from the switch, once homed.
    position: Option<u32>,
    cell: Option<usize>,
    pressed: bool,
    enabled: bool,
    off_at_us: Option<u64>,
    steps_due: u32,
    fault: Option<&'static str>,
    event: Option<Event>,
}

impl Slider {
    pub fn moving(&self) -> bool {
        self.phase != Phase::Idle
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn dir_high(&self) -> bool {
        self.away == AWAY_IS_HIGH
    }

    pub fn wants_driver(&self) -> bool {
        self.phase == Phase::Driver
    }

    /// How many step pulses to send now.
    pub fn take_steps(&mut self) -> u32 {
        core::mem::take(&mut self.steps_due)
    }

    pub fn deadline(&self) -> Option<u64> {
        match self.phase {
            Phase::Idle => self.off_at_us,
            Phase::Driver => None,
            _ => Some(self.run.next_us()),
        }
    }

    pub(crate) fn home(&mut self) -> Result<(), Refused> {
        if self.moving() {
            return Err(Refused::Busy);
        }
        self.phase = Phase::Driver;
        self.position = None;
        self.cell = None;
        self.fault = None;
        Ok(())
    }

    /// The firmware has tried to set up the driver. Homing starts, if that worked.
    pub fn driver_configured(&mut self, ok: bool, now_us: u64) {
        if !ok {
            self.fail("driver");
        } else if self.pressed {
            self.begin(Phase::Leaving, LEAVE_STEPS, HOME_SPEED, true, now_us);
        } else {
            self.begin(Phase::Seeking, SEEK_STEPS, HOME_SPEED, false, now_us);
        }
    }

    pub(crate) fn go(&mut self, name: &str, now_us: u64) -> Result<(), Refused> {
        let cell = CELLS
            .iter()
            .position(|(cell, _)| *cell == name)
            .ok_or(Refused::BadArgument)?;
        if self.moving() {
            return Err(Refused::Busy);
        }
        let position = self.position.ok_or(Refused::NotHomed)?;
        let target = CELLS[cell].1;
        self.cell = None;
        self.fault = None;
        let steps = target.abs_diff(position);
        self.begin(Phase::Moving(cell), steps, SPEED, target > position, now_us);
        Ok(())
    }

    /// Stops at once. The steps may slip when it stops dead, so it must home again.
    pub fn stop(&mut self) {
        if self.moving() {
            self.halt();
        }
    }

    pub fn poll(&mut self, now_us: u64, pressed: bool) -> Option<Event> {
        self.pressed = pressed;
        match self.phase {
            Phase::Idle => {
                if self.off_at_us.is_some_and(|at| now_us >= at) {
                    self.off_at_us = None;
                    self.enabled = false;
                }
            }
            Phase::Driver => {}
            Phase::Leaving if !pressed => {
                self.begin(Phase::Clearing, CLEAR_STEPS, HOME_SPEED, true, now_us);
            }
            Phase::Seeking if pressed => {
                self.position = Some(0);
                self.finish(Event::SliderHomed, now_us);
            }
            // No cell is at the switch, so this means the slider isn't where it should be.
            Phase::Moving(_) if pressed && !self.away => self.fail("home-hit"),
            _ => self.step(now_us),
        }
        self.event.take()
    }

    pub fn write_status(&self, reply: &mut Line) {
        let state = match self.phase {
            Phase::Idle => "idle",
            Phase::Moving(_) => "moving",
            _ => "homing",
        };
        let cell = self.cell.map_or("none", |cell| CELLS[cell].0);
        let homed = u8::from(self.position.is_some());
        let fault = self.fault.unwrap_or("none");
        let _ = write!(
            reply,
            "ok state={state} homed={homed} cell={cell} fault={fault}"
        );
    }

    fn begin(&mut self, phase: Phase, steps: u32, speed: u32, away: bool, now_us: u64) {
        let start_us = if self.enabled {
            now_us
        } else {
            now_us + ENABLE_SETTLE_US
        };
        self.phase = phase;
        self.away = away;
        self.enabled = true;
        self.off_at_us = None;
        self.run = Run {
            ramp: Ramp::new(steps, speed),
            start_us,
            done: 0,
        };
    }

    fn step(&mut self, now_us: u64) {
        let mut burst = 0;
        while self.run.done < self.run.ramp.steps && burst < MAX_BURST {
            if self.run.next_us() > now_us {
                break;
            }
            self.run.done += 1;
            burst += 1;
        }
        self.steps_due += burst;
        if let Some(position) = &mut self.position {
            if self.away {
                *position += burst;
            } else {
                *position -= burst;
            }
        }
        if self.run.done == self.run.ramp.steps {
            match self.phase {
                Phase::Leaving => self.fail("stuck"),
                Phase::Clearing => {
                    self.begin(Phase::Seeking, SEEK_STEPS, HOME_SPEED, false, now_us);
                }
                Phase::Seeking => self.fail("no-home"),
                Phase::Moving(cell) => {
                    self.cell = Some(cell);
                    self.finish(Event::SliderAt(CELLS[cell].0), now_us);
                }
                _ => {}
            }
        }
    }

    fn finish(&mut self, event: Event, now_us: u64) {
        self.phase = Phase::Idle;
        self.off_at_us = Some(now_us + OFF_AFTER_US);
        self.event = Some(event);
    }

    fn fail(&mut self, why: &'static str) {
        self.halt();
        self.fault = Some(why);
        self.event = Some(Event::SliderFault(why));
    }

    /// Switches the motor off, so any steps still to send would go nowhere.
    fn halt(&mut self) {
        self.phase = Phase::Idle;
        self.enabled = false;
        self.steps_due = 0;
        self.off_at_us = None;
        self.position = None;
        self.cell = None;
    }
}
