//! The LED channels and the interlock, the camera triggers and the exposure signals.

use core::fmt::Write;

use crate::{CAMERAS, CHANNELS, Line, PROTOCOL_VERSION};

/// How long the door's signal must stay high before the door counts as shut. It opens at once.
pub const DOOR_SETTLE_US: u64 = 50_000;
/// The longest trigger pulse.
pub const MAX_TRIGGER_US: u64 = 1_000_000;

/// What the pins read.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    /// The door's rail is live, so the door's switch is closed.
    pub door_shut: bool,
    /// Each camera's exposure input. An output that conducts pulls it low, which is assumed to
    /// mean exposing until the cameras are in hand: ToupTek publishes no pinout.
    pub exposure_low: [bool; CAMERAS],
}

/// What the pins should be.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outputs {
    pub leds: [bool; CHANNELS],
    pub triggers: [bool; CAMERAS],
}

/// Something to tell the client without being asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    DoorOpen,
    DoorShut,
    Exposure {
        camera: usize,
        started: bool,
        at_us: u64,
    },
}

impl Event {
    pub fn to_line(self) -> Line {
        let mut line = Line::new();
        // Every event fits in a line.
        let _ = match self {
            Self::DoorOpen => write!(line, "! door open"),
            Self::DoorShut => write!(line, "! door shut"),
            Self::Exposure {
                camera,
                started,
                at_us,
            } => {
                let edge = if started { "start" } else { "end" };
                write!(line, "! exposure {} {edge} {at_us}", camera + 1)
            }
        };
        line
    }
}

/// Why a command failed. Its code is what the client sees.
#[derive(Clone, Copy)]
enum Refused {
    UnknownCommand,
    BadArgument,
    DoorOpen,
    Busy,
}

impl Refused {
    const fn code(self) -> &'static str {
        match self {
            Self::UnknownCommand => "unknown-command",
            Self::BadArgument => "bad-argument",
            Self::DoorOpen => "door-open",
            Self::Busy => "busy",
        }
    }
}

/// The state of the light path and the triggers. An LED is on only if a command lit it after the
/// door last counted as shut.
#[derive(Debug, Default)]
pub struct Controller {
    door_shut: bool,
    high_since_us: Option<u64>,
    leds: [bool; CHANNELS],
    trigger_ends_us: [Option<u64>; CAMERAS],
    exposing: [bool; CAMERAS],
    exposures: [u32; CAMERAS],
    dropped: u32,
}

impl Controller {
    /// Starts with the door counted as open, and everything off.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn outputs(&self) -> Outputs {
        Outputs {
            leds: self.leds,
            triggers: self.trigger_ends_us.map(|end| end.is_some()),
        }
    }

    /// Takes what the pins read, and passes each event to `emit`, which returns whether it could
    /// queue it.
    pub fn poll(&mut self, now_us: u64, inputs: Inputs, mut emit: impl FnMut(Event) -> bool) {
        let mut send = |dropped: &mut u32, event| {
            if !emit(event) {
                *dropped = dropped.saturating_add(1);
            }
        };
        if !inputs.door_shut {
            self.leds = [false; CHANNELS];
            self.high_since_us = None;
            if core::mem::take(&mut self.door_shut) {
                send(&mut self.dropped, Event::DoorOpen);
            }
        } else if !self.door_shut {
            let since = *self.high_since_us.get_or_insert(now_us);
            if now_us - since >= DOOR_SETTLE_US {
                self.door_shut = true;
                send(&mut self.dropped, Event::DoorShut);
            }
        }
        for camera in 0..CAMERAS {
            if self.trigger_ends_us[camera].is_some_and(|end| now_us >= end) {
                self.trigger_ends_us[camera] = None;
            }
            let started = inputs.exposure_low[camera];
            if started != self.exposing[camera] {
                self.exposing[camera] = started;
                if started {
                    self.exposures[camera] = self.exposures[camera].wrapping_add(1);
                }
                send(
                    &mut self.dropped,
                    Event::Exposure {
                        camera,
                        started,
                        at_us: now_us,
                    },
                );
            }
        }
    }

    /// When `poll` next has something to do, even if no input changes.
    pub fn next_deadline_us(&self) -> Option<u64> {
        let settled = self
            .high_since_us
            .filter(|_| !self.door_shut)
            .map(|since| since + DOOR_SETTLE_US);
        self.trigger_ends_us
            .into_iter()
            .chain([settled])
            .flatten()
            .min()
    }

    /// Counts a line that the USB side had no room for.
    pub fn count_dropped(&mut self) {
        self.dropped = self.dropped.saturating_add(1);
    }

    /// The USB connection went away, so nothing can switch the LEDs off any more.
    pub fn host_lost(&mut self) {
        self.leds = [false; CHANNELS];
    }

    /// Runs a command, and returns the line to reply with.
    pub fn command(&mut self, command: &str, now_us: u64) -> Line {
        let mut words = command.split_ascii_whitespace();
        let mut reply = Line::new();
        let done = match [words.next(), words.next(), words.next(), words.next()] {
            [Some("version"), None, None, None] => {
                let firmware = env!("CARGO_PKG_VERSION");
                let _ = write!(
                    reply,
                    "ok optima-optics protocol={PROTOCOL_VERSION} firmware={firmware}"
                );
                return reply;
            }
            [Some("status"), None, None, None] => {
                self.write_status(&mut reply);
                return reply;
            }
            [Some("led"), Some("all"), Some("off"), None] => {
                self.leds = [false; CHANNELS];
                Ok(())
            }
            [Some("led"), Some(channel), Some(state), None] => self.set_led(channel, state),
            [Some("trigger"), Some(camera), Some(width), None] => {
                self.trigger(camera, width, now_us)
            }
            [Some("version" | "status" | "led" | "trigger"), ..] => Err(Refused::BadArgument),
            _ => Err(Refused::UnknownCommand),
        };
        let _ = match done {
            Ok(()) => write!(reply, "ok"),
            Err(refused) => write!(reply, "err {}", refused.code()),
        };
        reply
    }

    /// The reply to a line that `LineBuffer` rejected.
    pub fn rejected_line() -> Line {
        let mut reply = Line::new();
        let _ = write!(reply, "err bad-line");
        reply
    }

    fn set_led(&mut self, channel: &str, state: &str) -> Result<(), Refused> {
        let index = number(channel, CHANNELS as u64)? - 1;
        match state {
            "on" if !self.door_shut => Err(Refused::DoorOpen),
            "on" | "off" => {
                self.leds[index as usize] = state == "on";
                Ok(())
            }
            _ => Err(Refused::BadArgument),
        }
    }

    fn trigger(&mut self, camera: &str, width: &str, now_us: u64) -> Result<(), Refused> {
        let index = number(camera, CAMERAS as u64)? as usize - 1;
        let width = number(width, MAX_TRIGGER_US)?;
        if self.trigger_ends_us[index].is_some() {
            return Err(Refused::Busy);
        }
        self.trigger_ends_us[index] = Some(now_us + width);
        Ok(())
    }

    fn write_status(&self, reply: &mut Line) {
        let door = if self.door_shut { "shut" } else { "open" };
        let triggers = self.outputs().triggers;
        let [first, second] = self.exposures;
        // The longest status fits in a line, as a test checks.
        let _ = write!(reply, "ok door={door} leds=");
        write_bits(reply, &self.leds);
        let _ = write!(reply, " triggers=");
        write_bits(reply, &triggers);
        let _ = write!(reply, " exposing=");
        write_bits(reply, &self.exposing);
        let _ = write!(
            reply,
            " exposures={first},{second} dropped={}",
            self.dropped
        );
    }
}

fn write_bits(reply: &mut Line, bits: &[bool]) {
    for &bit in bits {
        let _ = reply.write_char(if bit { '1' } else { '0' });
    }
}

/// A number from 1 to `max`.
fn number(text: &str, max: u64) -> Result<u64, Refused> {
    text.parse()
        .ok()
        .filter(|n| (1..=max).contains(n))
        .ok_or(Refused::BadArgument)
}
