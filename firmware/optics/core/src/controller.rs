//! The LED channels and the interlock, the camera triggers and the exposure signals, with the
//! matrix, the camera port, the filter slider and sequences alongside them.

use core::fmt::Write;

use crate::matrix::Pattern;
use crate::port::Port;
use crate::sequence::{Advance, Light, Sequence};
use crate::slider::Slider;
use crate::{CAMERAS, CHANNELS, Line, PROTOCOL_VERSION};

/// How long the door's signal must stay high before the door counts as shut. It opens at once.
pub const DOOR_SETTLE_US: u64 = 50_000;
/// The longest trigger pulse.
pub const MAX_TRIGGER_US: u64 = 1_000_000;
/// The longest wait from an exposure's end to a sequence's next step.
pub const MAX_STEP_DELAY_US: u64 = 1_000_000;

/// What the pins read.
#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    /// The door's rail is live, so the door's switch is closed.
    pub door_shut: bool,
    /// Each camera's exposure input. An output that conducts pulls it low, which is assumed to
    /// mean exposing until the cameras are in hand: ToupTek publishes no pinout.
    pub exposure_low: [bool; CAMERAS],
    /// The camera port reports position 2.
    pub port_at: bool,
    /// The slider's home switch is pressed. It's normally closed, so a broken wire reads the same.
    pub slider_home: bool,
}

/// What the pins should be.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outputs {
    pub leds: [bool; CHANNELS],
    pub triggers: [bool; CAMERAS],
    pub matrix: Pattern,
    pub port_set: bool,
    pub slider_on: bool,
    pub slider_dir: bool,
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
    Port(u8),
    PortFault,
    SliderHomed,
    SliderAt(&'static str),
    SliderFault(&'static str),
    SeqStep {
        step: usize,
        at_us: u64,
    },
    SeqDone,
    SeqStopped,
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
            Self::Port(position) => write!(line, "! port {position}"),
            Self::PortFault => write!(line, "! port fault"),
            Self::SliderHomed => write!(line, "! slider homed"),
            Self::SliderAt(cell) => write!(line, "! slider {cell}"),
            Self::SliderFault(why) => write!(line, "! slider fault {why}"),
            Self::SeqStep { step, at_us } => write!(line, "! seq step {} {at_us}", step + 1),
            Self::SeqDone => write!(line, "! seq done"),
            Self::SeqStopped => write!(line, "! seq stopped"),
        };
        line
    }
}

/// Why a command failed. Its code is what the client sees.
#[derive(Clone, Copy)]
pub(crate) enum Refused {
    UnknownCommand,
    BadArgument,
    DoorOpen,
    Busy,
    Full,
    NoSteps,
    NotHomed,
}

impl Refused {
    const fn code(self) -> &'static str {
        match self {
            Self::UnknownCommand => "unknown-command",
            Self::BadArgument => "bad-argument",
            Self::DoorOpen => "door-open",
            Self::Busy => "busy",
            Self::Full => "full",
            Self::NoSteps => "no-steps",
            Self::NotHomed => "not-homed",
        }
    }
}

/// The state of the light path and the triggers. An epi LED is on only if a command or a sequence
/// lit it after the door last counted as shut.
#[derive(Debug, Default)]
pub struct Controller {
    door_shut: bool,
    high_since_us: Option<u64>,
    leds: [bool; CHANNELS],
    matrix: Pattern,
    trigger_ends_us: [Option<u64>; CAMERAS],
    exposing: [bool; CAMERAS],
    exposures: [u32; CAMERAS],
    dropped: u32,
    port: Port,
    slider: Slider,
    sequence: Sequence,
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
            matrix: self.matrix,
            port_set: self.port.set_high(),
            slider_on: self.slider.enabled(),
            slider_dir: self.slider.dir_high(),
        }
    }

    /// Takes what the pins read, and passes each event to `emit`, which returns whether it could
    /// queue it.
    pub fn poll(&mut self, now_us: u64, inputs: Inputs, mut emit: impl FnMut(Event) -> bool) {
        let dropped = &mut self.dropped;
        let mut send = |event| {
            if !emit(event) {
                *dropped = dropped.saturating_add(1);
            }
        };
        if !inputs.door_shut {
            self.leds = [false; CHANNELS];
            self.high_since_us = None;
            if core::mem::take(&mut self.door_shut) {
                send(Event::DoorOpen);
            }
            if self.sequence.running() && self.sequence.uses_leds() {
                self.sequence.stop();
                self.matrix = Pattern::OFF;
                send(Event::SeqStopped);
            }
        } else if !self.door_shut {
            let since = *self.high_since_us.get_or_insert(now_us);
            if now_us - since >= DOOR_SETTLE_US {
                self.door_shut = true;
                send(Event::DoorShut);
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
                } else {
                    self.sequence.exposure_ended(camera, now_us);
                }
                send(Event::Exposure {
                    camera,
                    started,
                    at_us: now_us,
                });
            }
        }
        match self.sequence.poll(now_us) {
            Some(Advance::Show { step, light }) => {
                self.leds = light.leds;
                self.matrix = light.matrix;
                send(Event::SeqStep {
                    step,
                    at_us: now_us,
                });
            }
            Some(Advance::Finished) => {
                self.leds = [false; CHANNELS];
                self.matrix = Pattern::OFF;
                send(Event::SeqDone);
            }
            None => {}
        }
        if let Some((camera, width)) = self.sequence.take_trigger(now_us) {
            self.trigger_ends_us[camera] = Some(now_us + width);
        }
        if let Some(event) = self.port.poll(now_us, inputs.port_at) {
            send(event);
        }
        if let Some(event) = self.slider.poll(now_us, inputs.slider_home) {
            send(event);
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
            .chain([
                settled,
                self.sequence.deadline(),
                self.port.deadline(),
                self.slider.deadline(),
            ])
            .flatten()
            .min()
    }

    /// Counts a line that the USB side had no room for.
    pub fn count_dropped(&mut self) {
        self.dropped = self.dropped.saturating_add(1);
    }

    /// The USB connection went away, so nothing can switch the lights off any more, or read a
    /// sequence's events. The slider finishes its move, which ends by itself.
    pub fn host_lost(&mut self) {
        self.sequence.stop();
        self.lights_off();
    }

    /// Whether the slider's driver needs setting up now, over its UART.
    pub fn slider_wants_driver(&self) -> bool {
        self.slider.wants_driver()
    }

    /// Reports whether setting up the slider's driver worked.
    pub fn driver_configured(&mut self, ok: bool, now_us: u64) {
        self.slider.driver_configured(ok, now_us);
    }

    /// How many step pulses to send the slider's driver now, after setting its direction.
    pub fn take_steps(&mut self) -> u32 {
        self.slider.take_steps()
    }

    /// Runs a command, and returns the line to reply with.
    pub fn command(&mut self, command: &str, now_us: u64) -> Line {
        let mut words = command.split_ascii_whitespace();
        let words = [(); 7].map(|()| words.next());
        let mut reply = Line::new();
        let done = match words {
            [Some("version"), None, ..] => {
                let firmware = env!("CARGO_PKG_VERSION");
                let _ = write!(
                    reply,
                    "ok optima-optics protocol={PROTOCOL_VERSION} firmware={firmware}"
                );
                Ok(())
            }
            [Some("status"), None, ..] => {
                self.write_status(&mut reply);
                Ok(())
            }
            [Some("led"), Some("all"), Some("off"), None, ..] => {
                self.stop_sequence();
                self.leds = [false; CHANNELS];
                Ok(())
            }
            [Some("led"), Some(channel), Some(state), None, ..] => self.set_led(channel, state),
            [Some("trigger"), Some(camera), Some(width), None, ..] => {
                self.trigger(camera, width, now_us)
            }
            [Some("matrix"), Some("status"), None, ..] => {
                let Pattern { red, green, blue } = self.matrix;
                let _ = write!(
                    reply,
                    "ok red={red:016x} green={green:016x} blue={blue:016x}"
                );
                Ok(())
            }
            [Some("matrix"), Some("off"), None, ..] => self.set_matrix(Pattern::OFF),
            [Some("matrix"), Some(red), Some(green), Some(blue), None, ..] => {
                Pattern::parse(red, green, blue).and_then(|pattern| self.set_matrix(pattern))
            }
            [Some("port"), Some("status"), None, ..] => {
                self.port.write_status(&mut reply);
                Ok(())
            }
            [Some("port"), Some(position), None, ..] => self.go_port(position, now_us),
            [Some("slider"), Some("status"), None, ..] => {
                self.slider.write_status(&mut reply);
                Ok(())
            }
            [Some("slider"), Some("stop"), None, ..] => {
                self.slider.stop();
                Ok(())
            }
            [Some("slider"), Some("home"), None, ..] => {
                self.no_sequence().and_then(|()| self.slider.home())
            }
            [Some("slider"), Some(cell), None, ..] => self
                .no_sequence()
                .and_then(|()| self.slider.go(cell, now_us)),
            [Some("seq"), Some("status"), None, ..] => {
                self.sequence.write_status(&mut reply);
                Ok(())
            }
            [Some("seq"), Some("stop"), None, ..] => {
                self.stop_sequence();
                Ok(())
            }
            [Some("seq"), Some("clear"), None, ..] => self.no_sequence().map(|()| {
                self.sequence.clear();
            }),
            [
                Some("seq"),
                Some("add"),
                Some(leds),
                Some(red),
                Some(green),
                Some(blue),
                None,
            ] => self.add_step(leds, red, green, blue),
            [
                Some("seq"),
                Some("start"),
                Some(camera),
                Some(cycles),
                Some(delay),
                Some(trigger),
                None,
            ] => self.start_sequence(camera, cycles, delay, trigger, now_us),
            [
                Some(
                    "version" | "status" | "led" | "trigger" | "matrix" | "port" | "slider" | "seq",
                ),
                ..,
            ] => Err(Refused::BadArgument),
            _ => Err(Refused::UnknownCommand),
        };
        match done {
            Ok(()) if reply.as_str().is_empty() => {
                let _ = write!(reply, "ok");
            }
            Ok(()) => {}
            Err(refused) => {
                let _ = write!(reply, "err {}", refused.code());
            }
        }
        reply
    }

    /// The reply to a line that `LineBuffer` rejected.
    pub fn rejected_line() -> Line {
        let mut reply = Line::new();
        let _ = write!(reply, "err bad-line");
        reply
    }

    fn lights_off(&mut self) {
        self.leds = [false; CHANNELS];
        self.matrix = Pattern::OFF;
    }

    /// However a sequence ends, its light goes out.
    fn stop_sequence(&mut self) {
        if self.sequence.running() {
            self.sequence.stop();
            self.lights_off();
        }
    }

    /// A running sequence owns the light path, whose light must stay constant through a frame.
    fn no_sequence(&self) -> Result<(), Refused> {
        if self.sequence.running() {
            Err(Refused::Busy)
        } else {
            Ok(())
        }
    }

    fn set_led(&mut self, channel: &str, state: &str) -> Result<(), Refused> {
        let index = number(channel, CHANNELS as u64)? as usize - 1;
        let on = match state {
            "on" => true,
            "off" => false,
            _ => return Err(Refused::BadArgument),
        };
        self.no_sequence()?;
        if on && !self.door_shut {
            return Err(Refused::DoorOpen);
        }
        self.leds[index] = on;
        Ok(())
    }

    fn trigger(&mut self, camera: &str, width: &str, now_us: u64) -> Result<(), Refused> {
        let index = number(camera, CAMERAS as u64)? as usize - 1;
        let width = number(width, MAX_TRIGGER_US)?;
        if self.trigger_ends_us[index].is_some() || self.sequence.triggers() {
            return Err(Refused::Busy);
        }
        self.trigger_ends_us[index] = Some(now_us + width);
        Ok(())
    }

    fn set_matrix(&mut self, pattern: Pattern) -> Result<(), Refused> {
        self.no_sequence()?;
        self.matrix = pattern;
        Ok(())
    }

    fn go_port(&mut self, position: &str, now_us: u64) -> Result<(), Refused> {
        let position = number(position, 2)?;
        self.no_sequence()?;
        if self.port.moving() {
            return Err(Refused::Busy);
        }
        self.port.go(position, now_us);
        Ok(())
    }

    fn add_step(&mut self, leds: &str, red: &str, green: &str, blue: &str) -> Result<(), Refused> {
        let light = Light {
            leds: led_bits(leds)?,
            matrix: Pattern::parse(red, green, blue)?,
        };
        self.no_sequence()?;
        self.sequence.add(light)
    }

    fn start_sequence(
        &mut self,
        camera: &str,
        cycles: &str,
        delay: &str,
        trigger: &str,
        now_us: u64,
    ) -> Result<(), Refused> {
        let camera = number(camera, CAMERAS as u64)? as usize - 1;
        let cycles = count(cycles, u32::MAX.into())? as u32;
        let delay_us = count(delay, MAX_STEP_DELAY_US)?;
        let trigger_us = count(trigger, MAX_TRIGGER_US)?;
        self.no_sequence()?;
        if self.exposing[camera]
            || self.trigger_ends_us[camera].is_some()
            || self.port.moving()
            || self.slider.moving()
        {
            return Err(Refused::Busy);
        }
        if self.sequence.is_empty() {
            return Err(Refused::NoSteps);
        }
        if self.sequence.uses_leds() && !self.door_shut {
            return Err(Refused::DoorOpen);
        }
        self.sequence
            .start(camera, cycles, delay_us, trigger_us, now_us);
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
    count(text, max).and_then(|n| {
        if n == 0 {
            Err(Refused::BadArgument)
        } else {
            Ok(n)
        }
    })
}

/// A number from 0 to `max`.
fn count(text: &str, max: u64) -> Result<u64, Refused> {
    text.parse()
        .ok()
        .filter(|n| *n <= max)
        .ok_or(Refused::BadArgument)
}

/// A digit for each channel, such as `1000`.
fn led_bits(text: &str) -> Result<[bool; CHANNELS], Refused> {
    let mut leds = [false; CHANNELS];
    if text.len() != CHANNELS {
        return Err(Refused::BadArgument);
    }
    for (led, digit) in leds.iter_mut().zip(text.bytes()) {
        *led = match digit {
            b'1' => true,
            b'0' => false,
            _ => return Err(Refused::BadArgument),
        };
    }
    Ok(leds)
}
