//! Lines of text, in fixed buffers.

use core::fmt;

/// The longest line either side sends, in bytes, without its line feed.
pub const MAX_LINE: usize = 120;

/// A line of ASCII text.
#[derive(Clone, Debug)]
pub struct Line {
    bytes: [u8; MAX_LINE + 1],
    len: usize,
}

impl Line {
    pub const fn new() -> Self {
        Self {
            bytes: [0; MAX_LINE + 1],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    /// The bytes to send, with a line feed added after the text.
    pub fn to_send(&mut self) -> &[u8] {
        if self.bytes[..self.len].last() != Some(&b'\n') {
            self.bytes[self.len] = b'\n';
            self.len += 1;
        }
        &self.bytes[..self.len]
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for Line {
    /// Takes what fits, and reports an error if that isn't all of it.
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for &byte in text.as_bytes() {
            if self.len == MAX_LINE {
                return Err(fmt::Error);
            }
            self.bytes[self.len] = byte;
            self.len += 1;
        }
        Ok(())
    }
}

/// What the USB side passes to the controller.
#[derive(Clone, Debug)]
pub enum Request {
    /// A command.
    Line(Line),
    /// A line that was too long, or wasn't printable ASCII.
    Rejected,
    /// The USB connection went away.
    HostLost,
}

/// Collects the bytes from USB, which can arrive in any pieces, into lines.
#[derive(Default)]
pub struct LineBuffer {
    line: Line,
    rejected: bool,
}

impl LineBuffer {
    pub const fn new() -> Self {
        Self {
            line: Line::new(),
            rejected: false,
        }
    }

    /// Takes the next byte, and returns a request if it ends a line. Empty lines are ignored.
    pub fn push(&mut self, byte: u8) -> Option<Request> {
        match byte {
            b'\n' => {
                let line = core::mem::take(&mut self.line);
                if core::mem::take(&mut self.rejected) {
                    Some(Request::Rejected)
                } else if line.len == 0 {
                    None
                } else {
                    Some(Request::Line(line))
                }
            }
            b'\r' => None,
            b' '..=b'~' if self.line.len < MAX_LINE => {
                self.line.bytes[self.line.len] = byte;
                self.line.len += 1;
                None
            }
            _ => {
                self.rejected = true;
                None
            }
        }
    }

    /// Forgets a line that has only partly arrived.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}
