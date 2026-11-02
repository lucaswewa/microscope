//! Preloaded sequences, stepped by a camera's exposure signal.

use optics_core::{
    CAMERAS, Controller, DOOR_SETTLE_US, Event, Inputs, LIGHT_SETTLE_US, Line, MAX_LINE, MAX_STEPS,
    Pattern,
};

const SHUT: Inputs = Inputs {
    door_shut: true,
    exposure_low: [false; CAMERAS],
    port_at: false,
    slider_home: false,
};
const EXPOSING_1: Inputs = Inputs {
    exposure_low: [true, false],
    ..SHUT
};
const EXPOSING_2: Inputs = Inputs {
    exposure_low: [false, true],
    ..SHUT
};
const OPEN: Inputs = Inputs {
    door_shut: false,
    ..SHUT
};

fn poll(controller: &mut Controller, now: u64, inputs: Inputs) -> Vec<Event> {
    let mut events = Vec::new();
    controller.poll(now, inputs, |event| {
        events.push(event);
        true
    });
    events
}

fn reply(controller: &mut Controller, command: &str, now: u64) -> String {
    controller.command(command, now).as_str().to_string()
}

fn with_door_shut() -> Controller {
    let mut controller = Controller::new();
    poll(&mut controller, 0, SHUT);
    assert_eq!(
        poll(&mut controller, DOOR_SETTLE_US, SHUT),
        [Event::DoorShut]
    );
    controller
}

/// Steps through each epi LED, then a matrix pattern.
fn with_steps(controller: &mut Controller) {
    for step in [
        "seq add 1000 0 0 0",
        "seq add 0100 0 0 0",
        "seq add 0000 f 0 0",
    ] {
        assert_eq!(reply(controller, step, 0), "ok", "{step}");
    }
}

/// One whole exposure of camera 1, starting at `now`.
fn expose(controller: &mut Controller, now: u64) -> Vec<Event> {
    poll(controller, now, EXPOSING_1);
    poll(controller, now + 10, SHUT)
}

fn exposure_end(camera: usize, at_us: u64) -> Event {
    Event::Exposure {
        camera,
        started: false,
        at_us,
    }
}

#[test]
fn it_shows_each_step_in_turn_on_the_exposures_ends_then_finishes() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    assert_eq!(reply(&mut controller, "seq start 1 1 0 0", 1000), "ok");
    assert_eq!(
        poll(&mut controller, 1000, SHUT),
        [Event::SeqStep {
            step: 0,
            at_us: 1000
        }]
    );
    assert_eq!(
        controller.outputs().leds,
        [true, false, false, false],
        "first step"
    );

    assert_eq!(
        expose(&mut controller, 2000),
        [
            exposure_end(0, 2010),
            Event::SeqStep {
                step: 1,
                at_us: 2010
            }
        ]
    );
    assert_eq!(controller.outputs().leds, [false, true, false, false]);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);

    expose(&mut controller, 3000);
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(controller.outputs().matrix.red, 0xF);

    assert_eq!(
        expose(&mut controller, 4000),
        [exposure_end(0, 4010), Event::SeqDone]
    );
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
    assert_eq!(
        reply(&mut controller, "seq status", 5000),
        "ok steps=3 state=idle"
    );
}

#[test]
fn the_light_stays_constant_while_a_camera_exposes() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 1 0 0", 0);
    poll(&mut controller, 0, SHUT);
    let light = controller.outputs();
    assert_eq!(poll(&mut controller, 100, EXPOSING_1).len(), 1);
    assert_eq!(controller.outputs(), light);
    assert_eq!(poll(&mut controller, 200, EXPOSING_1), []);
    assert_eq!(controller.outputs(), light);
}

#[test]
fn it_runs_the_cycles_asked_for() {
    let mut controller = with_door_shut();
    reply(&mut controller, "seq add 1000 0 0 0", 0);
    reply(&mut controller, "seq add 0100 0 0 0", 0);
    reply(&mut controller, "seq start 1 2 0 0", 0);
    poll(&mut controller, 0, SHUT);
    let mut leds = vec![controller.outputs().leds[0]];
    for frame in 1..=3 {
        let events = expose(&mut controller, frame * 1000);
        assert_eq!(events.len(), 2, "{events:?}");
        leds.push(controller.outputs().leds[0]);
    }
    assert_eq!(leds, [true, false, true, false]);
    assert_eq!(
        reply(&mut controller, "seq status", 0),
        "ok steps=2 state=running camera=1 step=2 cycle=2"
    );
    assert_eq!(expose(&mut controller, 9000)[1], Event::SeqDone);
}

#[test]
fn no_cycles_runs_until_stopped_and_stopping_puts_out_its_light() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    for frame in 1..=99 {
        let events = expose(&mut controller, frame * 1000);
        assert!(matches!(events[1], Event::SeqStep { .. }), "{events:?}");
    }
    assert!(
        controller.outputs().leds[0],
        "99 frames is 33 cycles, so step 1 again"
    );
    assert_eq!(reply(&mut controller, "seq stop", 200_000), "ok");
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(poll(&mut controller, 200_001, EXPOSING_1).len(), 1);
    assert_eq!(
        poll(&mut controller, 200_002, SHUT).len(),
        1,
        "no more steps"
    );
}

#[test]
fn stopping_when_idle_leaves_the_light_alone() {
    let mut controller = with_door_shut();
    reply(&mut controller, "led 1 on", 0);
    reply(&mut controller, "matrix 1 0 0", 0);
    assert_eq!(reply(&mut controller, "seq stop", 0), "ok");
    assert!(controller.outputs().leds[0]);
    assert_eq!(controller.outputs().matrix.red, 1);
}

#[test]
fn the_step_waits_for_the_delay_and_a_further_end_changes_nothing() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 500 0", 0);
    poll(&mut controller, 0, SHUT);
    assert_eq!(expose(&mut controller, 1000), [exposure_end(0, 1010)]);
    assert_eq!(controller.next_deadline_us(), Some(1510));
    expose(&mut controller, 1100);
    assert_eq!(controller.next_deadline_us(), Some(1510));
    assert_eq!(poll(&mut controller, 1509, SHUT), []);
    assert!(controller.outputs().leds[0]);
    assert_eq!(
        poll(&mut controller, 1510, SHUT),
        [Event::SeqStep {
            step: 1,
            at_us: 1510
        }]
    );
    assert!(controller.outputs().leds[1]);
}

#[test]
fn only_the_chosen_cameras_signal_steps_it() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 2 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    expose(&mut controller, 1000);
    assert!(controller.outputs().leds[0], "camera 1 doesn't step it");
    poll(&mut controller, 2000, EXPOSING_2);
    assert_eq!(poll(&mut controller, 2010, SHUT).len(), 2);
    assert!(controller.outputs().leds[1], "camera 2 does");
}

#[test]
fn it_can_trigger_the_camera_once_each_steps_light_has_settled() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    assert_eq!(reply(&mut controller, "seq start 1 0 0 300", 1000), "ok");
    poll(&mut controller, 1000, SHUT);
    assert_eq!(controller.outputs().triggers, [false; 2]);
    assert_eq!(controller.next_deadline_us(), Some(1000 + LIGHT_SETTLE_US));

    poll(&mut controller, 1000 + LIGHT_SETTLE_US - 1, SHUT);
    assert_eq!(controller.outputs().triggers, [false; 2]);
    poll(&mut controller, 1000 + LIGHT_SETTLE_US, SHUT);
    assert_eq!(controller.outputs().triggers, [true, false]);
    assert_eq!(reply(&mut controller, "trigger 2 10", 1), "err busy");

    poll(&mut controller, 1000 + LIGHT_SETTLE_US + 299, SHUT);
    assert_eq!(controller.outputs().triggers, [true, false]);
    poll(&mut controller, 1000 + LIGHT_SETTLE_US + 300, SHUT);
    assert_eq!(controller.outputs().triggers, [false; 2]);

    // The camera's exposure ends, the light changes, and the next trigger follows it.
    let now = expose(&mut controller, 10_000);
    assert_eq!(now.len(), 2);
    poll(&mut controller, 10_010 + LIGHT_SETTLE_US, SHUT);
    assert_eq!(controller.outputs().triggers, [true, false]);
}

#[test]
fn a_client_may_trigger_by_hand_if_the_sequence_doesnt() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    assert_eq!(reply(&mut controller, "trigger 1 10", 1), "ok");
    assert_eq!(controller.outputs().triggers, [true, false]);
}

#[test]
fn it_wont_start_without_steps_or_during_an_exposure_or_a_pulse() {
    let mut controller = with_door_shut();
    assert_eq!(
        reply(&mut controller, "seq start 1 0 0 0", 0),
        "err no-steps"
    );
    with_steps(&mut controller);
    poll(&mut controller, 1, EXPOSING_1);
    assert_eq!(reply(&mut controller, "seq start 1 0 0 0", 2), "err busy");
    assert_eq!(reply(&mut controller, "seq start 2 1 0 0", 2), "ok");
    assert_eq!(reply(&mut controller, "seq start 2 1 0 0", 2), "err busy");
    reply(&mut controller, "seq stop", 3);

    poll(&mut controller, 4, SHUT);
    reply(&mut controller, "trigger 1 100", 4);
    assert_eq!(reply(&mut controller, "seq start 1 0 0 0", 5), "err busy");
}

#[test]
fn it_wont_start_while_the_light_path_is_moving() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "port 2", 1);
    assert_eq!(reply(&mut controller, "seq start 1 0 0 0", 2), "err busy");
    let arrived = Inputs {
        port_at: true,
        ..SHUT
    };
    assert_eq!(poll(&mut controller, 3, arrived), [Event::Port(2)]);
    assert_eq!(reply(&mut controller, "slider home", 4), "ok");
    assert_eq!(reply(&mut controller, "seq start 1 0 0 0", 5), "err busy");
}

#[test]
fn while_it_runs_it_owns_the_light_path() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    let light = controller.outputs();
    for command in [
        "led 1 off",
        "led 2 on",
        "matrix 1 2 3",
        "matrix off",
        "port 2",
        "slider home",
        "slider a",
        "seq add 1000 0 0 0",
        "seq clear",
        "seq start 1 0 0 0",
    ] {
        assert_eq!(reply(&mut controller, command, 1), "err busy", "{command}");
    }
    assert_eq!(controller.outputs(), light);
}

#[test]
fn led_all_off_always_works_and_stops_a_sequence_with_its_matrix() {
    let mut controller = with_door_shut();
    reply(&mut controller, "seq add 1000 f 0 0", 0);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    assert_eq!(controller.outputs().matrix.red, 0xF);
    assert_eq!(reply(&mut controller, "led all off", 1), "ok");
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
    assert_eq!(
        reply(&mut controller, "seq status", 1),
        "ok steps=1 state=idle"
    );
}

#[test]
fn epi_led_steps_obey_the_door() {
    let mut controller = Controller::new();
    with_steps(&mut controller);
    assert_eq!(
        reply(&mut controller, "seq start 1 0 0 0", 0),
        "err door-open"
    );

    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    let now = DOOR_SETTLE_US * 2;
    assert_eq!(
        poll(&mut controller, now, OPEN),
        [Event::DoorOpen, Event::SeqStopped]
    );
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
    assert_eq!(
        reply(&mut controller, "seq status", now),
        "ok steps=3 state=idle"
    );
}

#[test]
fn the_door_stopping_a_sequence_puts_out_its_matrix_too() {
    let mut controller = with_door_shut();
    reply(&mut controller, "seq add 1000 f 0 0", 0);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    assert_eq!(controller.outputs().matrix.red, 0xF);
    poll(&mut controller, 1, OPEN);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
}

#[test]
fn the_loop_wakes_for_whichever_of_its_timers_comes_first() {
    let mut controller = with_door_shut();
    with_steps(&mut controller);
    reply(&mut controller, "seq start 1 0 100000 300", 1000);
    poll(&mut controller, 1000, SHUT);
    // The exposure ends before the trigger's due, so both the trigger and a step are waiting.
    expose(&mut controller, 1100);
    assert_eq!(controller.next_deadline_us(), Some(1000 + LIGHT_SETTLE_US));
}

#[test]
fn matrix_only_steps_dont_depend_on_the_door() {
    let mut controller = Controller::new();
    reply(&mut controller, "seq add 0000 1 0 0", 0);
    reply(&mut controller, "seq add 0000 2 0 0", 0);
    assert_eq!(reply(&mut controller, "seq start 1 0 0 0", 0), "ok");
    assert_eq!(poll(&mut controller, 0, OPEN).len(), 1);
    assert_eq!(controller.outputs().matrix.red, 1);
    let exposing = Inputs {
        exposure_low: [true, false],
        ..OPEN
    };
    poll(&mut controller, 10, exposing);
    poll(&mut controller, 20, OPEN);
    assert_eq!(controller.outputs().matrix.red, 2);
}

#[test]
fn losing_usb_stops_it_and_puts_out_its_light() {
    let mut controller = with_door_shut();
    reply(&mut controller, "seq add 1000 f 0 0", 0);
    reply(&mut controller, "seq start 1 0 0 0", 0);
    poll(&mut controller, 0, SHUT);
    controller.host_lost();
    assert_eq!(controller.outputs().leds, [false; 4]);
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
    assert_eq!(
        reply(&mut controller, "seq status", 0),
        "ok steps=1 state=idle"
    );
}

#[test]
fn clearing_forgets_the_steps_and_the_list_has_a_limit() {
    let mut controller = Controller::new();
    for _ in 0..MAX_STEPS {
        assert_eq!(reply(&mut controller, "seq add 0000 1 0 0", 0), "ok");
    }
    assert_eq!(reply(&mut controller, "seq add 0000 1 0 0", 0), "err full");
    assert_eq!(
        reply(&mut controller, "seq status", 0),
        "ok steps=64 state=idle"
    );
    assert_eq!(reply(&mut controller, "seq clear", 0), "ok");
    assert_eq!(
        reply(&mut controller, "seq status", 0),
        "ok steps=0 state=idle"
    );
    assert_eq!(
        reply(&mut controller, "seq start 1 0 0 0", 0),
        "err no-steps"
    );
}

#[test]
fn bad_arguments_are_refused() {
    let mut controller = with_door_shut();
    for bad in [
        "seq",
        "seq add",
        "seq add 1000 0 0",
        "seq add 100 0 0 0",
        "seq add 10000 0 0 0",
        "seq add 1002 0 0 0",
        "seq add 1000 g 0 0",
        "seq add 1000 0 0 0 0",
        "seq start",
        "seq start 1 0 0",
        "seq start 0 0 0 0",
        "seq start 3 0 0 0",
        "seq start 1 4294967296 0 0",
        "seq start 1 0 1000001 0",
        "seq start 1 0 0 1000001",
        "seq start 1 0 0 0 0",
        "seq go",
    ] {
        assert_eq!(reply(&mut controller, bad, 0), "err bad-argument", "{bad}");
    }
}

#[test]
fn the_longest_query_replies_fit_in_a_line() {
    let longest = "ok steps=64 state=running camera=2 step=64 cycle=4294967295";
    assert!(longest.len() <= MAX_LINE);
    let longest = "ok red=ffffffffffffffff green=ffffffffffffffff blue=ffffffffffffffff";
    assert!(longest.len() <= MAX_LINE);
    let longest: Line = Controller::new().command("matrix status", 0);
    assert_eq!(longest.as_str().len(), 68);
    let longest = "ok state=moving homed=1 cell=none fault=home-hit";
    assert!(longest.len() <= MAX_LINE);
}
