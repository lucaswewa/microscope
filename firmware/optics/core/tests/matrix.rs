//! The LED matrix's frames and its commands.

use optics_core::{Controller, FRAME_BYTES, Inputs, LEDS, Pattern};

const ROW_1_COLUMN_1: u64 = 1 << 63;

fn reply(controller: &mut Controller, command: &str) -> String {
    controller.command(command, 0).as_str().to_string()
}

fn led(frame: &[u8; FRAME_BYTES], index: usize) -> [u8; 4] {
    frame[4 + index * 4..4 + index * 4 + 4].try_into().unwrap()
}

/// Which LEDs along the chain a mask lights, seen in the frame's red bytes.
fn lit_in_chain(mask: u64) -> Vec<usize> {
    let frame = Pattern {
        red: mask,
        ..Pattern::OFF
    }
    .frame();
    (0..LEDS).filter(|&i| led(&frame, i)[3] != 0).collect()
}

/// The mask's bit for a row and column, each counted from 1.
fn bit(row: u32, column: u32) -> u64 {
    1 << (63 - ((row - 1) * 8 + column - 1))
}

#[test]
fn a_frame_has_its_start_its_leds_and_its_end() {
    let frame = Pattern::OFF.frame();
    assert_eq!(frame.len(), 4 + 64 * 4 + 8);
    assert_eq!(frame[..4], [0; 4]);
    assert_eq!(frame[FRAME_BYTES - 8..], [0; 8]);
    for index in 0..LEDS {
        assert_eq!(led(&frame, index), [0xFF, 0, 0, 0], "LED {index}");
    }
}

#[test]
fn each_colour_goes_in_its_own_byte_fully_on() {
    let frame = Pattern {
        blue: ROW_1_COLUMN_1,
        ..Pattern::OFF
    }
    .frame();
    assert_eq!(led(&frame, 0), [0xFF, 0xFF, 0, 0]);
    let frame = Pattern {
        green: ROW_1_COLUMN_1,
        ..Pattern::OFF
    }
    .frame();
    assert_eq!(led(&frame, 0), [0xFF, 0, 0xFF, 0]);
    let frame = Pattern {
        red: ROW_1_COLUMN_1,
        ..Pattern::OFF
    }
    .frame();
    assert_eq!(led(&frame, 0), [0xFF, 0, 0, 0xFF]);
}

#[test]
fn the_chain_runs_along_each_row_in_turn_reversing_each_time() {
    assert_eq!(lit_in_chain(bit(1, 1)), [0]);
    assert_eq!(lit_in_chain(bit(1, 8)), [7]);
    assert_eq!(lit_in_chain(bit(2, 8)), [8]);
    assert_eq!(lit_in_chain(bit(2, 1)), [15]);
    assert_eq!(lit_in_chain(bit(3, 1)), [16]);
    assert_eq!(lit_in_chain(bit(8, 8)), [56]);
    assert_eq!(lit_in_chain(bit(8, 1)), [63]);
}

#[test]
fn every_bit_lights_its_own_led() {
    let mut seen = [false; LEDS];
    for shift in 0..64 {
        let lit = lit_in_chain(1 << shift);
        assert_eq!(lit.len(), 1, "bit {shift}");
        assert!(!seen[lit[0]], "bit {shift} repeats LED {}", lit[0]);
        seen[lit[0]] = true;
    }
    assert_eq!(lit_in_chain(u64::MAX).len(), LEDS);
}

#[test]
fn the_commands_set_and_clear_the_pattern() {
    let mut controller = Controller::new();
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
    assert_eq!(reply(&mut controller, "matrix 8000000000000000 0 ff"), "ok");
    assert_eq!(
        controller.outputs().matrix,
        Pattern {
            red: ROW_1_COLUMN_1,
            green: 0,
            blue: 0xFF
        }
    );
    assert_eq!(
        reply(&mut controller, "matrix status"),
        "ok red=8000000000000000 green=0000000000000000 blue=00000000000000ff"
    );
    assert_eq!(reply(&mut controller, "matrix off"), "ok");
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
}

#[test]
fn hex_digits_are_read_in_either_case_up_to_sixteen() {
    let mut controller = Controller::new();
    assert_eq!(reply(&mut controller, "matrix FFFFFFFFFFFFFFFF aB 0"), "ok");
    assert_eq!(controller.outputs().matrix.red, u64::MAX);
    assert_eq!(controller.outputs().matrix.green, 0xAB);
}

#[test]
fn bad_masks_change_nothing() {
    let mut controller = Controller::new();
    reply(&mut controller, "matrix 1 2 3");
    for bad in [
        "matrix",
        "matrix 1",
        "matrix 1 2",
        "matrix 1 2 3 4",
        "matrix 1 2 g",
        "matrix +1 2 3",
        "matrix -1 2 3",
        "matrix 10000000000000000 2 3",
        "matrix on",
    ] {
        assert_eq!(reply(&mut controller, bad), "err bad-argument", "{bad}");
    }
    assert_eq!(
        controller.outputs().matrix,
        Pattern {
            red: 1,
            green: 2,
            blue: 3
        }
    );
}

#[test]
fn the_matrix_ignores_the_door_but_not_a_lost_host() {
    let mut controller = Controller::new();
    assert_eq!(reply(&mut controller, "matrix 1 0 0"), "ok");
    controller.poll(1, Inputs::default(), |_| true);
    assert_eq!(controller.outputs().matrix.red, 1);
    controller.host_lost();
    assert_eq!(controller.outputs().matrix, Pattern::OFF);
}
