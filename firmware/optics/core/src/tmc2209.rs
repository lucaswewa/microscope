//! What the firmware says to the slider's TMC2209 over its one-wire UART. The datagrams and
//! registers follow Analog Devices' TMC-API, and the sense resistor BTT's schematic. Analog's
//! datasheet wouldn't load, so the current formula is as the TMCStepper library's notes give it.

/// Passes bytes to the driver, and fills `reply` with what comes back. False if that fails.
pub trait DriverWire {
    fn exchange(&mut self, send: &[u8], reply: &mut [u8]) -> bool;
}

const SYNC: u8 = 0x05;
/// MS1 and MS2 are low on the carrier board.
const ADDRESS: u8 = 0;
/// The address a reply carries.
const REPLY_ADDRESS: u8 = 0xFF;
const WRITE: u8 = 0x80;
const IFCNT: u8 = 0x02;

/// Internal current reference, SpreadCycle, the UART in charge of PDN_UART and the microsteps, and
/// the step filter left on.
const GCONF: u32 = 1 << 2 | 1 << 6 | 1 << 7 | 1 << 8;
/// (14 + 1) / 32 × 0.18 V / (0.11 Ω + 0.02 Ω) / √2 = 459 mA RMS, under the 470 mA that the
/// objective changer's matching motor runs at. Half that when held, for the short time it is.
const IHOLD_IRUN: u32 = 7 | 14 << 8 | 1 << 16;
/// TOFF 3, HSTRT 5, HEND 0, TBL 1, the 0.18 V range, 4 microsteps a step, interpolated to 256.
const CHOPCONF: u32 = 3 | 5 << 4 | 1 << 15 | 1 << 17 | 6 << 24 | 1 << 28;

/// What the driver needs before the first move, as registers and values.
pub const SETUP: [(u8, u32); 3] = [(0x00, GCONF), (0x10, IHOLD_IRUN), (0x6C, CHOPCONF)];

/// CRC-8 with the polynomial 0x07, taking each byte's lowest bit first.
pub fn crc(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in bytes {
        let mut byte = byte;
        for _ in 0..8 {
            let feedback = (crc >> 7) ^ (byte & 1) != 0;
            crc <<= 1;
            if feedback {
                crc ^= 0x07;
            }
            byte >>= 1;
        }
    }
    crc
}

pub fn write_datagram(register: u8, value: u32) -> [u8; 8] {
    let [a, b, c, d] = value.to_be_bytes();
    let mut datagram = [SYNC, ADDRESS, register | WRITE, a, b, c, d, 0];
    datagram[7] = crc(&datagram[..7]);
    datagram
}

pub fn read_datagram(register: u8) -> [u8; 4] {
    let mut datagram = [SYNC, ADDRESS, register, 0];
    datagram[3] = crc(&datagram[..3]);
    datagram
}

/// A reply's value, if it's a good reply about this register.
pub fn parse_reply(register: u8, reply: &[u8; 8]) -> Option<u32> {
    let [sync, address, from, a, b, c, d, check] = *reply;
    let good =
        sync == SYNC && address == REPLY_ADDRESS && from == register && check == crc(&reply[..7]);
    good.then(|| u32::from_be_bytes([a, b, c, d]))
}

/// Writes `SETUP`. The driver counts the writes it accepts, so the count shows if they all landed.
pub fn configure(wire: &mut impl DriverWire) -> bool {
    let Some(before) = read_count(wire) else {
        return false;
    };
    for (register, value) in SETUP {
        if !wire.exchange(&write_datagram(register, value), &mut []) {
            return false;
        }
    }
    read_count(wire) == Some(before.wrapping_add(SETUP.len() as u8))
}

fn read_count(wire: &mut impl DriverWire) -> Option<u8> {
    let mut reply = [0; 8];
    if !wire.exchange(&read_datagram(IFCNT), &mut reply) {
        return None;
    }
    parse_reply(IFCNT, &reply).map(|count| count as u8)
}
