//! Zaber's ASCII protocol (ADR-0042), as its documentation describes it.
//!
//! - **A command** is `/device axis id command`, ended with a newline:
//!   `/1 1 7 get pos`. Device 0 is every device, and axis 0 the whole
//!   device. The id, 0 to 99, comes back in the reply, which is how the
//!   reply is found among the other lines on the port.
//! - **A reply** is `@device axis id flag status warning data`:
//!   `@01 1 07 OK BUSY -- 1200`. The flag is `OK`, or `RJ` with the reason
//!   in the data; the status `IDLE` or `BUSY`; the warning `--`, or the
//!   axis's most important warning flag.
//! - **Other lines,** alerts (`!`) and info (`#`), are passed over.

use std::fmt;
use std::io;
use std::time::{Duration, Instant};

use crate::drivers::serial::Line;

/// How long a device may take to reply.
pub const REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// An axis on the chain: its device's number, and its own on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    /// The device, from 1.
    pub device: u8,
    /// The axis on it, from 1.
    pub axis: u8,
}

/// A device's reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Who replied.
    pub from: Address,
    /// The command's id.
    pub id: Option<u8>,
    /// Whether the command was accepted (`OK`, not `RJ`).
    pub accepted: bool,
    /// Whether the axis is moving (`BUSY`).
    pub busy: bool,
    /// Its most important warning flag, such as `FS`.
    pub warning: Option<String>,
    /// The data: a value, or why the command was refused.
    pub data: String,
}

impl Reply {
    /// Reads `line`, if it's a reply.
    pub fn parse(line: &str) -> Option<Reply> {
        // A checksum, where the device sends them, ends the line: `:8D`.
        let line = match line.trim_end().rsplit_once(':') {
            Some((body, sum)) if sum.len() == 2 && sum.bytes().all(|b| b.is_ascii_hexdigit()) => {
                body
            }
            _ => line,
        };
        let words: Vec<&str> = line.strip_prefix('@')?.split_whitespace().collect();
        let from = Address {
            device: words.first()?.parse().ok()?,
            axis: words.get(1)?.parse().ok()?,
        };
        let id = words.get(2)?.parse().ok();
        let rest = &words[if id.is_some() { 3 } else { 2 }..];
        let [flag, status, warning, data @ ..] = rest else {
            return None;
        };
        Some(Reply {
            from,
            id,
            accepted: match *flag {
                "OK" => true,
                "RJ" => false,
                _ => return None,
            },
            busy: match *status {
                "BUSY" => true,
                "IDLE" => false,
                _ => return None,
            },
            warning: (*warning != "--").then(|| (*warning).to_owned()),
            data: data.join(" "),
        })
    }

    /// The data as a number.
    pub fn number(&self) -> Result<i64, ZaberError> {
        (self.data.parse()).map_err(|_| ZaberError::Unreadable(self.data.clone()))
    }
}

/// What went wrong on the chain.
#[derive(Debug)]
pub enum ZaberError {
    /// The port failed.
    Port(io::Error),
    /// No reply came.
    NoReply(String),
    /// The device refused the command.
    Refused {
        /// The command.
        command: String,
        /// The device's reason, such as `BADDATA`.
        reason: String,
    },
    /// A reply's data wasn't what the command gives.
    Unreadable(String),
}

impl fmt::Display for ZaberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZaberError::Port(error) => write!(f, "the stage's port failed: {error}"),
            ZaberError::NoReply(command) => write!(
                f,
                "the stage didn't reply to `{command}`: is it on, and on this port?"
            ),
            ZaberError::Refused { command, reason } => {
                write!(f, "the stage refused `{command}`: {}", refusal(reason))
            }
            ZaberError::Unreadable(data) => write!(f, "the stage's reply `{data}` isn't a number"),
        }
    }
}

impl std::error::Error for ZaberError {}

impl From<io::Error> for ZaberError {
    fn from(error: io::Error) -> Self {
        ZaberError::Port(error)
    }
}

/// What a refusal's reason means.
fn refusal(reason: &str) -> String {
    let meaning = match reason {
        "BADDATA" => "a value it can't take, such as a position beyond its travel",
        "BADCOMMAND" => "a command it doesn't know",
        "BADAXIS" => "an axis it doesn't have",
        "AGAIN" => "it's busy: try again",
        "PARKED" => "it's parked",
        "DRIVERDISABLED" => "its driver is disabled",
        _ => return reason.to_owned(),
    };
    format!("{meaning} ({reason})")
}

/// What a warning flag means. `F` flags are faults, `W` flags warnings,
/// and `N` flags notices.
pub fn warning(flag: &str) -> String {
    let meaning = match flag {
        "FD" => "its driver is disabled",
        "FQ" => "its encoder failed",
        "FS" => "it stalled, and stopped",
        "FT" => "it twisted too far",
        "FE" => "a limit sensor failed",
        "FB" => "a stream left its bounds",
        "FP" => "it strayed from its path",
        "WR" => "it isn't homed, so it has no reference position",
        "WL" => "it hit a limit sensor unexpectedly",
        "WV" => "its supply voltage is out of range",
        "WT" => "it's too hot",
        "WM" => "something moved it while it was still",
        "NC" => "it's under manual control",
        "NI" => "a move was interrupted",
        _ => return format!("warning {flag}"),
    };
    format!("{meaning} ({flag})")
}

/// A chain of Zaber devices on one port.
pub struct Zaber {
    line: Box<dyn Line>,
    next_id: u8,
}

impl Zaber {
    /// The chain on `line`.
    pub fn new(line: Box<dyn Line>) -> Zaber {
        Zaber { line, next_id: 0 }
    }

    /// Sends `command` to `to`, and returns its reply, or an error if the
    /// device refused it.
    pub fn ask(&mut self, to: Address, command: &str) -> Result<Reply, ZaberError> {
        let id = self.next_id;
        self.next_id = (id + 1) % 100;
        (self.line).send(&format!("/{} {} {id} {command}", to.device, to.axis))?;
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self
                .line
                .receive(left)
                .map_err(|error| match error.kind() {
                    io::ErrorKind::TimedOut => ZaberError::NoReply(command.to_owned()),
                    _ => ZaberError::Port(error),
                })?;
            match Reply::parse(&line) {
                Some(reply) if reply.from == to && reply.id == Some(id) => {
                    return if reply.accepted {
                        Ok(reply)
                    } else {
                        let (command, reason) = (command.to_owned(), reply.data);
                        Err(ZaberError::Refused { command, reason })
                    };
                }
                _ => tracing::debug!("passed over from the stage: {line}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::serial::Script;

    const AXIS: Address = Address { device: 2, axis: 1 };

    #[test]
    fn replies_are_read_with_or_without_ids_and_checksums() {
        let reply = Reply::parse("@02 1 07 OK BUSY FS 1200").expect("a reply");
        assert_eq!(reply.from, AXIS);
        assert_eq!(reply.id, Some(7));
        assert!(reply.accepted && reply.busy);
        assert_eq!(reply.warning.as_deref(), Some("FS"));
        assert_eq!(reply.number().expect("a number"), 1200);
        let reply = Reply::parse("@01 0 RJ IDLE -- BADCOMMAND:4F").expect("a reply");
        assert_eq!((reply.id, reply.accepted), (None, false));
        assert_eq!(reply.data, "BADCOMMAND");
        for other in [
            "!02 1 IDLE --",
            "#02 1 07 cmd move",
            "@02 1 07 OK",
            "@xx 1 OK IDLE -- 0",
        ] {
            assert_eq!(Reply::parse(other), None, "{other}");
        }
    }

    #[test]
    fn a_reply_is_found_by_its_address_and_id() {
        let script = Script::new()
            .expect(
                "/2 1 0 get pos",
                &[
                    "!01 1 IDLE --",
                    "@01 1 00 OK IDLE -- 5",
                    "@02 1 00 OK IDLE -- 1200",
                ],
            )
            .expect("/2 1 1 move abs 99999999", &["@02 1 01 RJ IDLE -- BADDATA"]);
        let mut zaber = Zaber::new(Box::new(script.clone()));
        let reply = zaber.ask(AXIS, "get pos").expect("a reply");
        assert_eq!(reply.number().expect("a number"), 1200);
        let refused = zaber.ask(AXIS, "move abs 99999999").expect_err("refused");
        assert_eq!(
            refused.to_string(),
            "the stage refused `move abs 99999999`: a value it can't take, such as a position \
             beyond its travel (BADDATA)"
        );
        assert_eq!(script.remaining(), 0);
    }

    #[test]
    fn no_reply_is_an_error_that_says_what_to_check() {
        let mut zaber = Zaber::new(Box::new(Script::new().expect("/2 1 0 get pos", &[])));
        let error = zaber.ask(AXIS, "get pos").expect_err("no reply");
        assert!(matches!(error, ZaberError::NoReply(_)));
        assert!(
            error.to_string().contains("is it on, and on this port?"),
            "{error}"
        );
    }

    #[test]
    fn warnings_say_what_they_mean() {
        assert_eq!(warning("FS"), "it stalled, and stopped (FS)");
        assert_eq!(
            warning("WR"),
            "it isn't homed, so it has no reference position (WR)"
        );
        assert_eq!(warning("XY"), "warning XY");
    }
}
