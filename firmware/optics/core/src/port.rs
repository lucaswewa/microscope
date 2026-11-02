//! The camera port: a flip mirror that follows a level, and reports where it is with another.

use core::fmt::Write;

use crate::Line;
use crate::controller::Event;

/// The longest a flip may take before it counts as failed. Assumed: the bench should check it.
pub const PORT_TIMEOUT_US: u64 = 3_000_000;

/// Position 1 sends the light to camera 1, and position 2 to camera 2.
#[derive(Debug, Default)]
pub struct Port {
    started: bool,
    set_high: bool,
    at_high: bool,
    due_us: Option<u64>,
    fault: bool,
}

impl Port {
    pub fn set_high(&self) -> bool {
        self.set_high
    }

    pub fn moving(&self) -> bool {
        self.due_us.is_some()
    }

    pub fn deadline(&self) -> Option<u64> {
        self.due_us
    }

    /// The mirror stays where it is at power-up: the first reading sets the level to match it.
    pub fn poll(&mut self, now_us: u64, at_high: bool) -> Option<Event> {
        self.at_high = at_high;
        if !core::mem::replace(&mut self.started, true) {
            self.set_high = at_high;
            return None;
        }
        let due = self.due_us?;
        if at_high == self.set_high {
            self.due_us = None;
            Some(Event::Port(self.position()))
        } else if now_us >= due {
            self.due_us = None;
            self.fault = true;
            Some(Event::PortFault)
        } else {
            None
        }
    }

    /// Goes to a position, which also reports arriving if the mirror is already there.
    pub fn go(&mut self, position: u64, now_us: u64) {
        self.set_high = position == 2;
        self.due_us = Some(now_us + PORT_TIMEOUT_US);
        self.fault = false;
    }

    pub fn write_status(&self, reply: &mut Line) {
        let state = match (self.moving(), self.fault) {
            (true, _) => "moving",
            (false, true) => "fault",
            (false, false) => "ready",
        };
        let _ = write!(reply, "ok port={} state={state}", self.position());
    }

    fn position(&self) -> u8 {
        if self.at_high { 2 } else { 1 }
    }
}
