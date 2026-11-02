//! The interlock, the LED channels, the triggers and the exposure signals, as a client sees them.

use optics_core::{
    CAMERAS, CHANNELS, Controller, DOOR_SETTLE_US, Event, Inputs, MAX_LINE, MAX_TRIGGER_US,
};

const SHUT: Inputs = Inputs {
    door_shut: true,
    exposure_low: [false; CAMERAS],
    port_at: false,
    slider_home: false,
};
const OPEN: Inputs = Inputs {
    door_shut: false,
    exposure_low: [false; CAMERAS],
    port_at: false,
    slider_home: false,
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

/// A controller that has seen the door shut for long enough.
fn with_door_shut() -> Controller {
    let mut controller = Controller::new();
    poll(&mut controller, 0, SHUT);
    assert_eq!(
        poll(&mut controller, DOOR_SETTLE_US, SHUT),
        [Event::DoorShut]
    );
    controller
}

#[test]
fn everything_starts_off_with_the_door_counted_as_open() {
    let mut controller = Controller::new();
    assert_eq!(controller.outputs(), Default::default());
    assert_eq!(reply(&mut controller, "led 1 on", 0), "err door-open");
    assert_eq!(controller.outputs().leds, [false; CHANNELS]);
}

#[test]
fn the_door_counts_as_shut_only_after_it_settles() {
    let mut controller = Controller::new();
    assert_eq!(poll(&mut controller, 1000, SHUT), []);
    assert_eq!(controller.next_deadline_us(), Some(1000 + DOOR_SETTLE_US));
    assert_eq!(poll(&mut controller, 1000 + DOOR_SETTLE_US - 1, SHUT), []);
    assert_eq!(
        reply(&mut controller, "led 1 on", 1000 + DOOR_SETTLE_US - 1),
        "err door-open"
    );
    assert_eq!(
        poll(&mut controller, 1000 + DOOR_SETTLE_US, SHUT),
        [Event::DoorShut]
    );
    assert_eq!(
        reply(&mut controller, "led 1 on", 1000 + DOOR_SETTLE_US),
        "ok"
    );
    assert_eq!(controller.next_deadline_us(), None);
}

#[test]
fn a_bouncing_door_starts_settling_again() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, SHUT);
    poll(&mut controller, DOOR_SETTLE_US - 1, OPEN);
    assert_eq!(poll(&mut controller, DOOR_SETTLE_US, SHUT), []);
    assert_eq!(poll(&mut controller, 2 * DOOR_SETTLE_US - 1, SHUT), []);
    assert_eq!(
        poll(&mut controller, 2 * DOOR_SETTLE_US, SHUT),
        [Event::DoorShut]
    );
}

#[test]
fn opening_the_door_puts_out_every_led_at_once_and_shutting_it_relights_none() {
    let mut controller = with_door_shut();
    for channel in 1..=CHANNELS {
        assert_eq!(
            reply(&mut controller, &format!("led {channel} on"), 0),
            "ok"
        );
    }
    assert_eq!(controller.outputs().leds, [true; CHANNELS]);

    assert_eq!(
        poll(&mut controller, DOOR_SETTLE_US + 1, OPEN),
        [Event::DoorOpen]
    );
    assert_eq!(controller.outputs().leds, [false; CHANNELS]);
    assert_eq!(
        poll(&mut controller, DOOR_SETTLE_US + 2, OPEN),
        [],
        "says so once"
    );

    poll(&mut controller, 2 * DOOR_SETTLE_US, SHUT);
    assert_eq!(
        poll(&mut controller, 3 * DOOR_SETTLE_US, SHUT),
        [Event::DoorShut]
    );
    assert_eq!(controller.outputs().leds, [false; CHANNELS]);
}

/// Any mix of commands and door movements, from a small generator: no LED is ever on unless the
/// door has been seen high for the whole settling time.
#[test]
fn no_sequence_lights_an_led_without_a_settled_door() {
    let commands = [
        "led 1 on",
        "led 2 on",
        "led 3 on",
        "led 4 on",
        "led 2 off",
        "led all off",
        "status",
        "seq add 1100 0 0 0",
        "seq add 0011 ff 0 0",
        "seq start 1 0 0 0",
        "seq start 2 2 100 0",
        "seq stop",
        "seq clear",
    ];
    let (mut state, mut now) = (0x2545_F491_u32, 0_u64);
    let (mut controller, mut high_since) = (Controller::new(), None);
    let mut lit = 0;
    for _ in 0..200_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        now += u64::from(state % 40_000);
        let door_shut = state & 0x300 != 0;
        high_since = if door_shut {
            high_since.or(Some(now))
        } else {
            None
        };
        poll(
            &mut controller,
            now,
            Inputs {
                door_shut,
                exposure_low: [state & 0x10_0000 != 0, state & 0x20_0000 != 0],
                ..Default::default()
            },
        );
        reply(
            &mut controller,
            commands[(state >> 12) as usize % commands.len()],
            now,
        );
        if controller.outputs().leds.contains(&true) {
            lit += 1;
            let settled = high_since.is_some_and(|since| now - since >= DOOR_SETTLE_US);
            assert!(
                settled,
                "lit at {now} with the door high since {high_since:?}"
            );
        }
    }
    assert!(
        lit > 1000,
        "the test must reach lit states, and reached {lit}"
    );
}

#[test]
fn leds_follow_their_own_commands() {
    let mut controller = with_door_shut();
    assert_eq!(reply(&mut controller, "led 2 on", 0), "ok");
    assert_eq!(reply(&mut controller, "led 4 on", 0), "ok");
    assert_eq!(controller.outputs().leds, [false, true, false, true]);
    assert_eq!(reply(&mut controller, "led 2 off", 0), "ok");
    assert_eq!(controller.outputs().leds, [false, false, false, true]);
    assert_eq!(reply(&mut controller, "led all off", 0), "ok");
    assert_eq!(controller.outputs().leds, [false; CHANNELS]);
}

#[test]
fn an_led_can_be_switched_off_with_the_door_open() {
    let mut controller = Controller::new();
    assert_eq!(reply(&mut controller, "led 1 off", 0), "ok");
    assert_eq!(reply(&mut controller, "led all off", 0), "ok");
}

#[test]
fn losing_the_host_puts_out_the_leds() {
    let mut controller = with_door_shut();
    reply(&mut controller, "led 3 on", 0);
    controller.host_lost();
    assert_eq!(controller.outputs().leds, [false; CHANNELS]);
}

#[test]
fn a_trigger_is_high_for_its_width() {
    let mut controller = Controller::new();
    assert_eq!(reply(&mut controller, "trigger 2 500", 1000), "ok");
    assert_eq!(controller.outputs().triggers, [false, true]);
    assert_eq!(controller.next_deadline_us(), Some(1500));
    poll(&mut controller, 1499, OPEN);
    assert_eq!(controller.outputs().triggers, [false, true]);
    poll(&mut controller, 1500, OPEN);
    assert_eq!(controller.outputs().triggers, [false, false]);
    assert_eq!(controller.next_deadline_us(), None);
}

#[test]
fn a_trigger_refuses_a_second_pulse_until_the_first_ends() {
    let mut controller = Controller::new();
    assert_eq!(reply(&mut controller, "trigger 1 100", 0), "ok");
    assert_eq!(reply(&mut controller, "trigger 1 100", 50), "err busy");
    assert_eq!(
        reply(&mut controller, "trigger 2 100", 50),
        "ok",
        "the other camera is free"
    );
    poll(&mut controller, 100, OPEN);
    assert_eq!(reply(&mut controller, "trigger 1 100", 100), "ok");
}

#[test]
fn the_nearest_deadline_comes_first() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, SHUT);
    reply(&mut controller, "trigger 1 900", 0);
    reply(&mut controller, "trigger 2 300", 0);
    assert_eq!(controller.next_deadline_us(), Some(300));
    poll(&mut controller, 300, SHUT);
    assert_eq!(controller.next_deadline_us(), Some(900));
    poll(&mut controller, 900, SHUT);
    assert_eq!(controller.next_deadline_us(), Some(DOOR_SETTLE_US));
}

#[test]
fn exposure_edges_become_events_and_starts_are_counted() {
    let mut controller = Controller::new();
    let low = |camera: usize| {
        let mut inputs = OPEN;
        inputs.exposure_low[camera] = true;
        inputs
    };
    assert_eq!(
        poll(&mut controller, 10, low(1)),
        [Event::Exposure {
            camera: 1,
            started: true,
            at_us: 10
        }]
    );
    assert_eq!(poll(&mut controller, 20, low(1)), [], "no edge, no event");
    assert_eq!(
        poll(&mut controller, 30, OPEN),
        [Event::Exposure {
            camera: 1,
            started: false,
            at_us: 30
        }]
    );
    poll(&mut controller, 40, low(1));
    poll(&mut controller, 50, low(0));
    let status = reply(&mut controller, "status", 50);
    assert!(status.contains("exposing=10 exposures=1,2"), "{status}");
}

#[test]
fn events_read_as_the_protocol_says() {
    let line = |event: Event| event.to_line().as_str().to_string();
    assert_eq!(line(Event::DoorOpen), "! door open");
    assert_eq!(line(Event::DoorShut), "! door shut");
    assert_eq!(
        line(Event::Exposure {
            camera: 0,
            started: true,
            at_us: 7
        }),
        "! exposure 1 start 7"
    );
    assert_eq!(
        line(Event::Exposure {
            camera: 1,
            started: false,
            at_us: u64::MAX
        }),
        format!("! exposure 2 end {}", u64::MAX)
    );
}

#[test]
fn events_with_no_room_are_counted_and_lost_ones_change_nothing_else() {
    let mut controller = with_door_shut();
    reply(&mut controller, "led 1 on", 0);
    controller.poll(100_000, OPEN, |_| false);
    assert_eq!(
        controller.outputs().leds,
        [false; CHANNELS],
        "the interlock doesn't need room"
    );
    controller.count_dropped();
    let status = reply(&mut controller, "status", 100_000);
    assert!(
        status.ends_with("door=open leds=0000 triggers=00 exposing=00 exposures=0,0 dropped=2"),
        "{status}"
    );
}

#[test]
fn version_and_status_read_as_the_protocol_says() {
    let mut controller = with_door_shut();
    let version = format!(
        "ok optima-optics protocol=1 firmware={}",
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(reply(&mut controller, "version", 0), version);
    reply(&mut controller, "led 1 on", 0);
    reply(&mut controller, "trigger 2 100", 0);
    assert_eq!(
        reply(&mut controller, "status", 0),
        "ok door=shut leds=1000 triggers=01 exposing=00 exposures=0,0 dropped=0"
    );
}

/// Counting to `u32::MAX` would take too long, so this checks the longest status as text, and that
/// the format it's written in is the real one.
#[test]
fn the_longest_status_fits_in_a_line() {
    let longest = "ok door=open leds=0000 triggers=00 exposing=00 exposures=4294967295,4294967295 dropped=4294967295";
    assert!(longest.len() <= MAX_LINE, "{} bytes", longest.len());
    let real = reply(&mut Controller::new(), "status", 0).replace(",0", ",4294967295");
    let real = real
        .replace("=0,", "=4294967295,")
        .replace("dropped=0", "dropped=4294967295");
    assert_eq!(real, longest);
}

#[test]
fn bad_commands_are_refused_and_change_nothing() {
    let mut controller = with_door_shut();
    for (command, code) in [
        ("hello", "unknown-command"),
        ("LED 1 on", "unknown-command"),
        ("version now", "bad-argument"),
        ("status 1", "bad-argument"),
        ("led", "bad-argument"),
        ("led 1", "bad-argument"),
        ("led 0 on", "bad-argument"),
        ("led 5 on", "bad-argument"),
        ("led one on", "bad-argument"),
        ("led 1 bright", "bad-argument"),
        ("led 1 on now", "bad-argument"),
        ("led all on", "bad-argument"),
        ("trigger", "bad-argument"),
        ("trigger 1", "bad-argument"),
        ("trigger 0 100", "bad-argument"),
        ("trigger 3 100", "bad-argument"),
        ("trigger 1 0", "bad-argument"),
        ("trigger 1 -5", "bad-argument"),
        ("trigger 1 99999999999999999999999", "bad-argument"),
    ] {
        assert_eq!(
            reply(&mut controller, command, 0),
            format!("err {code}"),
            "{command}"
        );
        assert_eq!(controller.outputs(), Default::default(), "{command}");
    }
}

#[test]
fn the_longest_trigger_is_allowed_and_no_longer() {
    let mut controller = Controller::new();
    let too_long = format!("trigger 1 {}", MAX_TRIGGER_US + 1);
    assert_eq!(reply(&mut controller, &too_long, 0), "err bad-argument");
    assert_eq!(
        reply(&mut controller, &format!("trigger 1 {MAX_TRIGGER_US}"), 0),
        "ok"
    );
}

#[test]
fn extra_spaces_do_not_matter() {
    let mut controller = with_door_shut();
    assert_eq!(reply(&mut controller, "  led   2   on ", 0), "ok");
    assert_eq!(controller.outputs().leds, [false, true, false, false]);
}
