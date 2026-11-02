//! The LED matrix: 8 × 8 APA102s, with each colour of each LED fully on or fully off, because
//! dimming would flicker across a rolling shutter.

use crate::controller::Refused;

pub const LEDS: usize = 64;
/// What one update sends: a start frame, a frame per LED, and the end. Eight bytes of zeros at the
/// end clock the data through the whole chain, and suit the APA102's SK9822 lookalikes too.
pub const FRAME_BYTES: usize = 4 + LEDS * 4 + 8;

/// A bit per LED, for each colour. Row 1 is the top byte, and column 1 is the top bit of its byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pattern {
    pub red: u64,
    pub green: u64,
    pub blue: u64,
}

impl Pattern {
    pub const OFF: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };

    /// Reads a mask for each colour, of up to 16 hex digits.
    pub(crate) fn parse(red: &str, green: &str, blue: &str) -> Result<Self, Refused> {
        Ok(Self {
            red: mask(red)?,
            green: mask(green)?,
            blue: mask(blue)?,
        })
    }

    /// The bytes that show this pattern, once sent to the panel over SPI.
    pub fn frame(&self) -> [u8; FRAME_BYTES] {
        let mut frame = [0; FRAME_BYTES];
        let (leds, _) = frame[4..4 + LEDS * 4].as_chunks_mut::<4>();
        for (led, bytes) in leds.iter_mut().enumerate() {
            let on = |colour: u64| {
                if colour & chain_bit(led) != 0 {
                    0xFF
                } else {
                    0
                }
            };
            // Three marker bits and full global brightness, then blue, green and red.
            *bytes = [0xFF, on(self.blue), on(self.green), on(self.red)];
        }
        frame
    }
}

/// The mask's bit for the `led`th LED along the panel's chain. Pololu's 2532 is serpentine, each
/// row running the opposite way to the one before. That it starts at row 1, column 1 is assumed,
/// like the Adafruit 2734's whole layout, until a panel is in hand.
fn chain_bit(led: usize) -> u64 {
    let row = led / 8;
    let column = if row.is_multiple_of(2) {
        led % 8
    } else {
        7 - led % 8
    };
    1 << (63 - (row * 8 + column))
}

fn mask(text: &str) -> Result<u64, Refused> {
    // `from_str_radix` alone would take a leading plus sign.
    if !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Refused::BadArgument);
    }
    u64::from_str_radix(text, 16).map_err(|_| Refused::BadArgument)
}
