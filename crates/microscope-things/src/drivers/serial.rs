//! Serial ports, for stages with a protocol of text lines (Zaber's, the
//! Sangaboard's), and a [`Script`] to test those protocols without one.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// A conversation in lines with a device: what a stage's protocol needs.
pub trait Line {
    /// Sends `line`, and the newline that ends it.
    fn send(&mut self, line: &str) -> io::Result<()>;
    /// The next line the device sends, without its line ending, within
    /// `timeout`.
    fn receive(&mut self, timeout: Duration) -> io::Result<String>;
}

/// The serial ports on this computer: `COM3` on Windows, `/dev/ttyUSB0` on
/// Linux.
pub fn ports() -> io::Result<Vec<String>> {
    let ports = serial2::SerialPort::available_ports()?;
    Ok(ports
        .iter()
        .map(|port| port.display().to_string())
        .collect())
}

/// A serial port, as a [`Line`].
pub struct SerialPort {
    port: serial2::SerialPort,
    received: Received,
}

impl SerialPort {
    /// Opens the port `name`, such as `COM3` or `/dev/ttyUSB0`, at `baud`
    /// bits a second, 8 data bits, no parity and 1 stop bit.
    pub fn open(name: &str, baud: u32) -> io::Result<SerialPort> {
        let port = serial2::SerialPort::open(name, baud)
            .map_err(|error| io::Error::new(error.kind(), format!("can't open {name}: {error}")))?;
        let received = Received::default();
        Ok(SerialPort { port, received })
    }
}

impl Line for SerialPort {
    fn send(&mut self, line: &str) -> io::Result<()> {
        self.port.write_all(format!("{line}\n").as_bytes())
    }

    fn receive(&mut self, timeout: Duration) -> io::Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(line) = self.received.line() {
                return Ok(line);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(no_answer(timeout));
            }
            self.port.set_read_timeout(left)?;
            let mut buffer = [0; 256];
            match self.port.read(&mut buffer) {
                Ok(read) => self.received.add(&buffer[..read]),
                Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
                Err(error) => return Err(error),
            }
        }
    }
}

fn no_answer(timeout: Duration) -> io::Error {
    let message = format!("the device didn't answer within {timeout:?}");
    io::Error::new(io::ErrorKind::TimedOut, message)
}

/// What a port has received, split into lines.
#[derive(Default)]
struct Received(Vec<u8>);

impl Received {
    fn add(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }

    /// The first whole line, without `\n` or `\r\n`.
    fn line(&mut self) -> Option<String> {
        let end = self.0.iter().position(|&byte| byte == b'\n')?;
        let line: Vec<u8> = self.0.drain(..=end).collect();
        let line = String::from_utf8_lossy(&line);
        Some(line.trim_end_matches(['\r', '\n']).to_owned())
    }
}

/// A fake device, as a [`Line`], for protocol tests: a script of the lines
/// a driver must send, each with the device's answers. Its clones share
/// the script, so a test keeps one to check what the driver sent.
#[derive(Clone, Default)]
pub struct Script(Arc<Mutex<Conversation>>);

#[derive(Default)]
struct Conversation {
    steps: VecDeque<Step>,
    answers: VecDeque<String>,
    sent: Vec<String>,
}

enum Step {
    Expect(String, Vec<String>),
    Fail(io::ErrorKind),
}

impl Script {
    /// An empty script: anything sent is unexpected.
    pub fn new() -> Script {
        Script::default()
    }

    /// Then the driver sends `line`, and the device answers `answers`.
    pub fn expect(self, line: &str, answers: &[&str]) -> Script {
        let answers = answers.iter().map(|&answer| answer.to_owned()).collect();
        self.lock()
            .steps
            .push_back(Step::Expect(line.to_owned(), answers));
        self
    }

    /// Then the device fails, as one unplugged does: the next line sent
    /// fails with `kind`.
    pub fn fail(self, kind: io::ErrorKind) -> Script {
        self.lock().steps.push_back(Step::Fail(kind));
        self
    }

    /// What the driver has sent.
    pub fn sent(&self) -> Vec<String> {
        self.lock().sent.clone()
    }

    /// How many of the script's steps haven't been reached.
    pub fn remaining(&self) -> usize {
        self.lock().steps.len()
    }

    fn lock(&self) -> MutexGuard<'_, Conversation> {
        self.0.lock().unwrap_or_else(|error| error.into_inner())
    }
}

impl Line for Script {
    fn send(&mut self, line: &str) -> io::Result<()> {
        let mut conversation = self.lock();
        conversation.sent.push(line.to_owned());
        match conversation.steps.pop_front() {
            Some(Step::Expect(expected, answers)) if expected == line => {
                conversation.answers.extend(answers);
                Ok(())
            }
            Some(Step::Expect(expected, _)) => Err(io::Error::other(format!(
                "sent {line:?} where the script expected {expected:?}"
            ))),
            Some(Step::Fail(kind)) => Err(io::Error::new(kind, "the device has gone")),
            None => Err(io::Error::other(format!(
                "sent {line:?} after the script's end"
            ))),
        }
    }

    fn receive(&mut self, timeout: Duration) -> io::Result<String> {
        (self.lock().answers.pop_front()).ok_or_else(|| no_answer(timeout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAIT: Duration = Duration::from_millis(10);

    #[test]
    fn a_script_answers_the_lines_it_expects_in_order() {
        let mut line = Script::new()
            .expect("/1 get pos", &["@01 0 OK IDLE -- 1000"])
            .expect("/1 move rel 10", &["@01 0 OK BUSY -- 0", "!01 0 IDLE --"]);
        line.send("/1 get pos").expect("sent");
        assert_eq!(
            line.receive(WAIT).expect("an answer"),
            "@01 0 OK IDLE -- 1000"
        );
        line.send("/1 move rel 10").expect("sent");
        assert_eq!(line.receive(WAIT).expect("an answer"), "@01 0 OK BUSY -- 0");
        assert_eq!(line.receive(WAIT).expect("an answer"), "!01 0 IDLE --");
        assert_eq!(line.remaining(), 0);
        assert_eq!(line.sent(), ["/1 get pos", "/1 move rel 10"]);
    }

    #[test]
    fn a_script_refuses_what_it_doesnt_expect() {
        let mut line = Script::new().expect("/home", &[]);
        let error = line.send("/stop").expect_err("unexpected");
        assert_eq!(
            error.to_string(),
            r#"sent "/stop" where the script expected "/home""#
        );
        let error = line.send("/home").expect_err("after the end");
        assert_eq!(error.to_string(), r#"sent "/home" after the script's end"#);
        let silent = line.receive(WAIT).expect_err("no answer");
        assert_eq!(silent.kind(), io::ErrorKind::TimedOut);
    }

    #[test]
    fn a_script_can_fail_as_an_unplugged_device_does() {
        let mut line = Script::new().fail(io::ErrorKind::BrokenPipe);
        let error = line.send("/get pos").expect_err("gone");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn what_a_port_receives_is_split_into_lines() {
        let mut received = Received::default();
        received.add(b"@01 0 OK ID");
        assert_eq!(received.line(), None);
        received.add(b"LE\r\n!01 0 IDLE\n@02");
        assert_eq!(received.line().as_deref(), Some("@01 0 OK IDLE"));
        assert_eq!(received.line().as_deref(), Some("!01 0 IDLE"));
        assert_eq!(received.line(), None);
    }
}
