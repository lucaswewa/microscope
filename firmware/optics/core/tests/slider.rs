//! The filter slider, against a simulated clock and a simulated switch.

use optics_core::{AWAY_IS_HIGH, CELLS, Controller, Event, Inputs, STEPS_PER_MM};

const SPEED_LIMIT: f64 = 10.0 * STEPS_PER_MM as f64;
const ACCELERATION_LIMIT: f64 = 50.0 * STEPS_PER_MM as f64;

/// Runs a controller, with a slider whose position is counted from its pulses.
struct Rig {
    controller: Controller,
    now: u64,
    /// Steps from where the switch trips, from the pulses and their direction.
    position: i64,
    /// When the slider is this close to the switch, or past it, the switch is pressed.
    switch_at: Option<i64>,
    /// Overrides what the switch reads.
    forced: Option<bool>,
    events: Vec<Event>,
    /// When each pulse was handed over.
    pulses: Vec<u64>,
}

impl Rig {
    fn new(position: i64) -> Self {
        Self {
            controller: Controller::new(),
            now: 1000,
            position,
            switch_at: Some(0),
            forced: None,
            events: Vec::new(),
            pulses: Vec::new(),
        }
    }

    /// A slider that has already homed, with the record of that cleared.
    fn homed() -> Self {
        let mut rig = Self::new(STEPS_PER_MM as i64);
        rig.send("slider home");
        rig.driver(true);
        rig.run_until(|event| *event == Event::SliderHomed);
        rig.settle();
        rig.events.clear();
        rig.pulses.clear();
        rig
    }

    fn send(&mut self, command: &str) -> String {
        self.controller
            .command(command, self.now)
            .as_str()
            .to_string()
    }

    fn pressed(&self) -> bool {
        self.forced
            .unwrap_or_else(|| self.switch_at.is_some_and(|at| self.position <= at))
    }

    fn poll(&mut self) {
        let inputs = Inputs {
            slider_home: self.pressed(),
            ..Default::default()
        };
        let events = &mut self.events;
        self.controller.poll(self.now, inputs, |event| {
            events.push(event);
            true
        });
        let outputs = self.controller.outputs();
        let away = outputs.slider_dir == AWAY_IS_HIGH;
        for _ in 0..self.controller.take_steps() {
            assert!(outputs.slider_on, "a step with the driver off");
            self.position += if away { 1 } else { -1 };
            self.pulses.push(self.now);
        }
    }

    fn driver(&mut self, ok: bool) {
        self.poll();
        assert!(self.controller.slider_wants_driver());
        self.controller.driver_configured(ok, self.now);
    }

    /// Follows the controller's own deadlines until an event matches, as the firmware's loop does.
    fn run_until(&mut self, done: impl Fn(&Event) -> bool) {
        for _ in 0..1_000_000 {
            self.poll();
            if self.events.iter().any(&done) {
                return;
            }
            let tick = self.now + 10_000;
            self.now = self
                .controller
                .next_deadline_us()
                .map_or(tick, |due| due.min(tick))
                .max(self.now + 1);
        }
        panic!("no such event: {:?}", self.events);
    }

    /// Lets the motor's off-delay run out.
    fn settle(&mut self) {
        self.now += 1_000_000;
        self.poll();
    }
}

#[test]
fn moves_wait_until_it_has_homed() {
    let mut rig = Rig::new(0);
    assert_eq!(rig.send("slider a"), "err not-homed");
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=0 cell=none fault=none"
    );
    assert!(!rig.controller.outputs().slider_on);
}

#[test]
fn homing_sets_up_the_driver_seeks_the_switch_and_stops_on_it() {
    let mut rig = Rig::new(10 * STEPS_PER_MM as i64);
    rig.poll();
    assert!(!rig.controller.slider_wants_driver());
    assert_eq!(rig.send("slider home"), "ok");
    assert_eq!(
        rig.send("slider status"),
        "ok state=homing homed=0 cell=none fault=none"
    );
    rig.driver(true);
    assert!(rig.controller.outputs().slider_on);
    assert_ne!(rig.controller.outputs().slider_dir, AWAY_IS_HIGH);
    rig.run_until(|event| *event == Event::SliderHomed);
    assert_eq!(rig.position, 0);
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=1 cell=none fault=none"
    );
}

#[test]
fn the_motor_gets_a_moment_to_start_and_goes_off_after_a_move() {
    let mut rig = Rig::new(STEPS_PER_MM as i64);
    rig.send("slider home");
    rig.driver(true);
    let enabled_at = rig.now;
    rig.run_until(|event| *event == Event::SliderHomed);
    // 5 ms for the driver, then the 10 ms that a ramp's first step takes.
    assert_eq!(rig.pulses[0], enabled_at + 15_000);
    assert!(rig.controller.outputs().slider_on, "still on just after");
    let off = rig.now + 250_000;
    assert_eq!(rig.controller.next_deadline_us(), Some(off));
    rig.now = off - 1;
    rig.poll();
    assert!(rig.controller.outputs().slider_on);
    rig.now = off;
    rig.poll();
    assert!(!rig.controller.outputs().slider_on);
}

#[test]
fn a_motor_thats_still_on_needs_no_moment_to_start() {
    let mut rig = Rig::new(STEPS_PER_MM as i64);
    rig.send("slider home");
    rig.driver(true);
    rig.run_until(|event| *event == Event::SliderHomed);
    rig.pulses.clear();
    let started_at = rig.now;
    rig.send("slider c");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.pulses[0], started_at + 10_000);
}

#[test]
fn no_driver_is_a_fault_and_the_motor_stays_off() {
    let mut rig = Rig::new(100);
    rig.send("slider home");
    rig.driver(false);
    rig.poll();
    assert_eq!(rig.events, [Event::SliderFault("driver")]);
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=0 cell=none fault=driver"
    );
    assert!(!rig.controller.outputs().slider_on);
    assert!(rig.pulses.is_empty());
}

#[test]
fn a_switch_already_pressed_means_leaving_it_first() {
    let mut rig = Rig::new(-10);
    rig.send("slider home");
    rig.driver(true);
    assert_eq!(rig.controller.outputs().slider_dir, AWAY_IS_HIGH);
    rig.run_until(|event| *event == Event::SliderHomed);
    assert_eq!(rig.position, 0);
    assert!(
        rig.pulses.len() > 2 * STEPS_PER_MM as usize,
        "it cleared the switch"
    );
}

#[test]
fn a_switch_that_never_releases_is_a_fault() {
    let mut rig = Rig::new(0);
    rig.forced = Some(true);
    rig.send("slider home");
    rig.driver(true);
    rig.run_until(|event| matches!(event, Event::SliderFault(_)));
    assert_eq!(rig.events, [Event::SliderFault("stuck")]);
    // The last step is dropped, because the motor goes off as it falls due.
    assert_eq!(rig.pulses.len(), 5 * STEPS_PER_MM as usize - 1);
    assert!(!rig.controller.outputs().slider_on);
}

#[test]
fn a_switch_that_never_trips_is_a_fault_after_the_whole_travel() {
    let mut rig = Rig::new(0);
    rig.forced = Some(false);
    rig.send("slider home");
    rig.driver(true);
    rig.run_until(|event| matches!(event, Event::SliderFault(_)));
    assert_eq!(rig.events, [Event::SliderFault("no-home")]);
    assert_eq!(rig.pulses.len(), 90 * STEPS_PER_MM as usize - 1);
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=0 cell=none fault=no-home"
    );
}

#[test]
fn it_goes_to_each_cell_and_reports_arriving() {
    let mut rig = Rig::homed();
    for (name, target) in [CELLS[0], CELLS[2], CELLS[1], CELLS[0]] {
        rig.events.clear();
        assert_eq!(rig.send(&format!("slider {name}")), "ok");
        assert_eq!(
            rig.send("slider status").split(' ').nth(1),
            Some("state=moving")
        );
        rig.run_until(|event| matches!(event, Event::SliderAt(_)));
        assert_eq!(rig.events, [Event::SliderAt(name)]);
        assert_eq!(rig.position, i64::from(target), "cell {name}");
        assert_eq!(
            rig.send("slider status"),
            format!("ok state=idle homed=1 cell={name} fault=none")
        );
        rig.settle();
    }
}

#[test]
fn the_cells_are_a_pitch_apart_with_c_nearest_the_switch() {
    let mm = |steps: u32| steps / STEPS_PER_MM;
    assert_eq!(CELLS.map(|(name, _)| name), ["a", "b", "c"]);
    assert_eq!(CELLS.map(|(_, at)| mm(at)), [81, 41, 1]);
}

#[test]
fn going_where_it_already_is_reports_arriving_with_no_steps() {
    let mut rig = Rig::homed();
    rig.send("slider c");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    rig.events.clear();
    rig.pulses.clear();
    rig.send("slider c");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.events, [Event::SliderAt("c")]);
    assert!(rig.pulses.is_empty());
}

#[test]
fn a_move_keeps_within_its_speed_and_acceleration() {
    let mut rig = Rig::homed();
    rig.send("slider a");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.pulses.len(), CELLS[0].1 as usize);

    // Over a window of steps, because at top speed one step's rounding to 1 µs would swamp this.
    let speeds: Vec<(f64, f64)> = rig
        .pulses
        .windows(21)
        .step_by(20)
        .map(|steps| {
            let (first, last) = (steps[0] as f64, steps[20] as f64);
            ((first + last) / 2.0 / 1e6, 20.0 * 1e6 / (last - first))
        })
        .collect();
    let top = speeds.iter().map(|&(_, speed)| speed).fold(0.0, f64::max);
    assert!(top <= SPEED_LIMIT * 1.01, "{top} steps/s");
    assert!(
        top >= SPEED_LIMIT * 0.99,
        "{top} steps/s: it should reach its speed"
    );
    assert!(
        speeds[0].1 < SPEED_LIMIT / 3.0,
        "starts slowly: {}",
        speeds[0].1
    );
    let last = speeds[speeds.len() - 1].1;
    assert!(last < SPEED_LIMIT / 3.0, "ends slowly: {last}");
    for pair in speeds.windows(2) {
        let change = (pair[1].1 - pair[0].1).abs() / (pair[1].0 - pair[0].0);
        assert!(change <= ACCELERATION_LIMIT * 1.1, "{change} steps/s²");
    }
}

#[test]
fn a_short_move_never_reaches_top_speed_and_still_arrives() {
    // Homed, the slider is at 0, and cell (c) is 1 mm away: just enough for its ramps.
    let mut rig = Rig::homed();
    rig.send("slider c");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.pulses.len(), STEPS_PER_MM as usize);
    let duration = rig.pulses[rig.pulses.len() - 1] - rig.pulses[0];
    // Two ramps of 0.141 s each, less the 10 ms before the first step.
    assert!(duration.abs_diff(273_000) < 1000, "{duration} µs");
}

#[test]
fn a_late_poll_catches_up_in_bursts_and_loses_no_steps() {
    let mut rig = Rig::homed();
    rig.send("slider a");
    let mut bursts = 0;
    let mut most = 0;
    while !rig.events.iter().any(|e| matches!(e, Event::SliderAt(_))) {
        rig.now += 3_000;
        let before = rig.pulses.len();
        rig.poll();
        most = most.max(rig.pulses.len() - before);
        bursts += 1;
        assert!(bursts < 1_000_000);
    }
    assert!(most <= 8, "{most} pulses in one go");
    assert_eq!(rig.position, i64::from(CELLS[0].1));
}

#[test]
fn a_stop_halts_at_once_and_it_must_home_again() {
    let mut rig = Rig::homed();
    rig.send("slider a");
    rig.now += 500_000;
    rig.poll();
    let so_far = rig.pulses.len();
    assert!(so_far > 0);
    assert_eq!(rig.send("slider stop"), "ok");
    assert!(!rig.controller.outputs().slider_on);
    rig.now += 500_000;
    rig.poll();
    assert_eq!(rig.pulses.len(), so_far);
    assert!(rig.events.is_empty());
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=0 cell=none fault=none"
    );
    assert_eq!(rig.send("slider a"), "err not-homed");
}

#[test]
fn stopping_when_idle_keeps_its_place() {
    let mut rig = Rig::homed();
    assert_eq!(rig.send("slider stop"), "ok");
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=1 cell=none fault=none"
    );
}

#[test]
fn hitting_the_switch_on_the_way_to_a_cell_is_a_fault() {
    let mut rig = Rig::homed();
    rig.send("slider a");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    rig.settle();
    rig.events.clear();
    rig.send("slider b");
    rig.now += 300_000;
    rig.poll();
    rig.forced = Some(true);
    rig.now += 1000;
    rig.poll();
    assert_eq!(rig.events, [Event::SliderFault("home-hit")]);
    assert!(!rig.controller.outputs().slider_on);
    assert_eq!(
        rig.send("slider status"),
        "ok state=idle homed=0 cell=none fault=home-hit"
    );
}

#[test]
fn leaving_the_switch_toward_a_cell_is_no_fault() {
    let mut rig = Rig::homed();
    assert!(rig.pressed(), "homing leaves it on the switch");
    rig.send("slider b");
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.events, [Event::SliderAt("b")]);
}

#[test]
fn a_new_move_waits_for_the_last_and_bad_cells_are_refused() {
    let mut rig = Rig::homed();
    rig.send("slider a");
    assert_eq!(rig.send("slider b"), "err busy");
    assert_eq!(rig.send("slider home"), "err busy");
    for bad in ["slider", "slider d", "slider a b", "slider 1"] {
        assert_eq!(rig.send(bad), "err bad-argument", "{bad}");
    }
}

#[test]
fn losing_usb_lets_a_move_finish() {
    let mut rig = Rig::homed();
    rig.send("slider c");
    rig.controller.host_lost();
    rig.run_until(|event| matches!(event, Event::SliderAt(_)));
    assert_eq!(rig.events, [Event::SliderAt("c")]);
}
