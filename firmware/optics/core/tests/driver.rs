//! The slider driver's datagrams, against a fake TMC2209 on the wire. The check values came from
//! the CRC table in Analog Devices' TMC-API.

use std::collections::BTreeMap;

use optics_core::STEPS_PER_MM;
use optics_core::tmc2209::{
    DriverWire, SETUP, configure, crc, parse_reply, read_datagram, write_datagram,
};

const IFCNT: u8 = 0x02;

fn reply_to_ifcnt(count: u32) -> [u8; 8] {
    let [a, b, c, d] = count.to_be_bytes();
    let mut reply = [0x05, 0xFF, IFCNT, a, b, c, d, 0];
    reply[7] = crc(&reply[..7]);
    reply
}

#[test]
fn the_crc_matches_analog_devices_table() {
    assert_eq!(crc(&[0x05, 0x00, 0x02]), 0x8F);
    assert_eq!(crc(&[0x05, 0xFF, 0x02, 0, 0, 0, 7]), 0xE2);
    assert_eq!(crc(&[]), 0);
}

#[test]
fn a_read_request_is_four_bytes() {
    assert_eq!(read_datagram(IFCNT), [0x05, 0x00, 0x02, 0x8F]);
}

#[test]
fn a_write_is_eight_bytes_with_the_value_high_byte_first() {
    assert_eq!(
        write_datagram(0x00, 0x1C4),
        [0x05, 0x00, 0x80, 0x00, 0x00, 0x01, 0xC4, 0x16]
    );
}

#[test]
fn the_setup_is_what_the_register_definitions_say() {
    let datagrams = SETUP.map(|(register, value)| write_datagram(register, value));
    assert_eq!(
        datagrams,
        [
            // GCONF: SpreadCycle, PDN_UART and the microsteps by UART, and the step filter.
            [0x05, 0x00, 0x80, 0x00, 0x00, 0x01, 0xC4, 0x16],
            // IHOLD_IRUN: hold 7, run 14, a hold delay of 1.
            [0x05, 0x00, 0x90, 0x00, 0x01, 0x0E, 0x07, 0x57],
            // CHOPCONF: as its field-by-field test below.
            [0x05, 0x00, 0xEC, 0x16, 0x02, 0x80, 0x53, 0x5A],
        ]
    );
}

#[test]
fn the_run_current_stays_under_the_motors_setting() {
    let (_, ihold_irun) = SETUP[1];
    let irun = (ihold_irun >> 8) & 0x1F;
    let (_, chopconf) = SETUP[2];
    assert_eq!(chopconf >> 17 & 1, 1, "the 0.18 V range");
    let amps = (f64::from(irun) + 1.0) / 32.0 * 0.18 / (0.11 + 0.02) / 2f64.sqrt();
    assert!((0.45..=0.47).contains(&amps), "{amps} A");
}

#[test]
fn the_microsteps_match_the_sliders_steps_per_millimetre() {
    let (_, chopconf) = SETUP[2];
    let mres = (chopconf >> 24) & 0xF;
    let microsteps = 256 >> mres;
    assert_eq!(microsteps, 4);
    assert_eq!(200 * microsteps / 2, STEPS_PER_MM);
    assert_eq!(chopconf & 0xF, 3, "TOFF nonzero, or the driver stays off");
    assert_eq!(chopconf >> 28 & 1, 1, "interpolated to 256");
}

#[test]
fn a_reply_must_be_good_and_about_the_right_register() {
    let good = reply_to_ifcnt(7);
    assert_eq!(good[7], 0xE2);
    assert_eq!(parse_reply(IFCNT, &good), Some(7));
    assert_eq!(parse_reply(0x00, &good), None);
    for byte in 0..8 {
        let mut bad = good;
        bad[byte] ^= 0x01;
        assert_eq!(parse_reply(IFCNT, &bad), None, "byte {byte}");
    }
}

/// On one wire, the firmware's own bytes come back too, with a good CRC and the driver's address.
#[test]
fn a_well_formed_datagram_that_isnt_a_reply_is_refused() {
    let with_good_crc = |sync: u8, address: u8| {
        let mut reply = [sync, address, IFCNT, 0, 0, 0, 7, 0];
        reply[7] = crc(&reply[..7]);
        reply
    };
    assert_eq!(parse_reply(IFCNT, &with_good_crc(0x05, 0xFF)), Some(7));
    assert_eq!(parse_reply(IFCNT, &with_good_crc(0x05, 0x00)), None);
    assert_eq!(parse_reply(IFCNT, &with_good_crc(0x06, 0xFF)), None);
}

/// Answers as a TMC2209 does: only to good datagrams for its address, counting the writes.
#[derive(Default)]
struct FakeDriver {
    registers: BTreeMap<u8, u32>,
    count: u32,
    /// Ignore the write with this index, as if its CRC had failed on the wire.
    ignore_write: Option<usize>,
    writes_seen: usize,
    silent: bool,
    corrupt_replies: bool,
}

impl DriverWire for FakeDriver {
    fn exchange(&mut self, send: &[u8], reply: &mut [u8]) -> bool {
        if self.silent {
            return reply.is_empty();
        }
        let (body, check) = send.split_at(send.len() - 1);
        assert_eq!(check[0], crc(body), "a bad CRC from the firmware");
        assert_eq!(body[..2], [0x05, 0x00]);
        if body[2] & 0x80 != 0 {
            assert_eq!(send.len(), 8);
            assert!(reply.is_empty());
            let ignored = self.ignore_write == Some(self.writes_seen);
            self.writes_seen += 1;
            if !ignored {
                let value = u32::from_be_bytes([body[3], body[4], body[5], body[6]]);
                self.registers.insert(body[2] & 0x7F, value);
                self.count = (self.count + 1) & 0xFF;
            }
            true
        } else {
            assert_eq!((send.len(), reply.len()), (4, 8));
            assert_eq!(body[2], IFCNT);
            reply.copy_from_slice(&reply_to_ifcnt(self.count));
            if self.corrupt_replies {
                reply[7] ^= 0xFF;
            }
            true
        }
    }
}

#[test]
fn configuring_writes_each_register_and_checks_the_count() {
    let mut driver = FakeDriver::default();
    assert!(configure(&mut driver));
    for (register, value) in SETUP {
        assert_eq!(driver.registers[&register], value);
    }
    assert_eq!(driver.count, 3);
}

#[test]
fn the_count_wraps_like_the_drivers() {
    let mut driver = FakeDriver {
        count: 254,
        ..Default::default()
    };
    assert!(configure(&mut driver));
    assert_eq!(driver.count, 1);
}

#[test]
fn a_write_the_driver_missed_fails_the_check() {
    for write in 0..SETUP.len() {
        let mut driver = FakeDriver {
            ignore_write: Some(write),
            ..Default::default()
        };
        assert!(!configure(&mut driver), "write {write}");
    }
}

#[test]
fn no_driver_or_a_garbled_reply_fails_the_check() {
    let mut driver = FakeDriver {
        silent: true,
        ..Default::default()
    };
    assert!(!configure(&mut driver));
    let mut driver = FakeDriver {
        corrupt_replies: true,
        ..Default::default()
    };
    assert!(!configure(&mut driver));
}
