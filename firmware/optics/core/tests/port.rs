//! The camera port, as a client sees it.

use optics_core::{Controller, Event, Inputs, PORT_TIMEOUT_US};

fn poll(controller: &mut Controller, now: u64, port_at: bool) -> Vec<Event> {
    let mut events = Vec::new();
    let inputs = Inputs {
        port_at,
        ..Default::default()
    };
    controller.poll(now, inputs, |event| {
        events.push(event);
        true
    });
    events
}

fn reply(controller: &mut Controller, command: &str, now: u64) -> String {
    controller.command(command, now).as_str().to_string()
}

#[test]
fn at_power_up_the_level_matches_the_mirror_so_it_stays_put() {
    for at in [false, true] {
        let mut controller = Controller::new();
        assert_eq!(poll(&mut controller, 0, at), []);
        assert_eq!(controller.outputs().port_set, at);
        assert_eq!(poll(&mut controller, 1, at), []);
    }
    let mut controller = Controller::new();
    poll(&mut controller, 0, true);
    assert_eq!(
        reply(&mut controller, "port status", 0),
        "ok port=2 state=ready"
    );
}

#[test]
fn a_move_sets_the_level_then_reports_arriving() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, false);
    assert_eq!(reply(&mut controller, "port 2", 10), "ok");
    assert!(controller.outputs().port_set);
    assert_eq!(
        reply(&mut controller, "port status", 10),
        "ok port=1 state=moving"
    );
    assert_eq!(poll(&mut controller, 20, false), []);
    assert_eq!(poll(&mut controller, 500_000, true), [Event::Port(2)]);
    assert_eq!(
        reply(&mut controller, "port status", 500_000),
        "ok port=2 state=ready"
    );
    assert_eq!(poll(&mut controller, 500_001, true), []);

    assert_eq!(reply(&mut controller, "port 1", 600_000), "ok");
    assert!(!controller.outputs().port_set);
    assert_eq!(poll(&mut controller, 600_001, false), [Event::Port(1)]);
}

#[test]
fn going_where_it_already_is_still_reports_arriving() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, true);
    assert_eq!(reply(&mut controller, "port 2", 1), "ok");
    assert_eq!(poll(&mut controller, 2, true), [Event::Port(2)]);
}

#[test]
fn a_mirror_that_never_arrives_is_a_fault_until_the_next_move() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, false);
    reply(&mut controller, "port 2", 100);
    assert_eq!(controller.next_deadline_us(), Some(100 + PORT_TIMEOUT_US));
    assert_eq!(poll(&mut controller, 100 + PORT_TIMEOUT_US - 1, false), []);
    assert_eq!(
        poll(&mut controller, 100 + PORT_TIMEOUT_US, false),
        [Event::PortFault]
    );
    assert_eq!(controller.next_deadline_us(), None);
    assert_eq!(
        reply(&mut controller, "port status", 0),
        "ok port=1 state=fault"
    );
    assert_eq!(reply(&mut controller, "port 1", 10), "ok");
    assert_eq!(
        reply(&mut controller, "port status", 10),
        "ok port=1 state=moving"
    );
    assert_eq!(poll(&mut controller, 20, false), [Event::Port(1)]);
    assert_eq!(
        reply(&mut controller, "port status", 20),
        "ok port=1 state=ready"
    );
}

#[test]
fn a_second_move_waits_for_the_first() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, false);
    reply(&mut controller, "port 2", 1);
    assert_eq!(reply(&mut controller, "port 1", 2), "err busy");
    assert!(controller.outputs().port_set);
}

#[test]
fn bad_positions_are_refused() {
    let mut controller = Controller::new();
    poll(&mut controller, 0, false);
    for bad in ["port", "port 0", "port 3", "port x", "port 1 2"] {
        assert_eq!(reply(&mut controller, bad, 1), "err bad-argument", "{bad}");
    }
    assert!(!controller.outputs().port_set);
}
