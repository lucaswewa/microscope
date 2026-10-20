//! The sidecar contract (ADR-0032), through the built binary: with
//! `--managed`, the ready line once serving, exit code 75 for a restart, 0
//! for a shutdown, and a graceful stop when standard input closes. With
//! `--service`, the same but the stop. Without either, none of that but the
//! shutdown.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const SERVER: &str = env!("CARGO_BIN_EXE_microscope-server");
const CONFIG: &str =
    r#"{"things": {"system": "microscope.system:MicroscopeSystem"}, "api_prefix": "/api/v1"}"#;
/// `teta-wot`'s default shutdown grace period.
const GRACE: Duration = Duration::from_secs(5);
/// How long a server may take to say where it is.
const START: Duration = Duration::from_secs(30);

/// A server on a free port, in a folder of its own, the line that said
/// where it is, and what it prints after that.
struct Running {
    child: Child,
    stdin: Option<ChildStdin>,
    line: String,
    lines: Receiver<String>,
    reader: Option<JoinHandle<()>>,
    _folder: tempfile::TempDir,
}

/// Starts a server, with `--managed`, `--service` or neither, and waits for
/// the line that says it's serving: the ready line, or else where it's
/// listening.
fn start(supervised: Option<&str>) -> Running {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let mut command = Command::new(SERVER);
    command.args(["--port", "0", "-j", CONFIG]);
    command.args(supervised);
    let mut child = (command.current_dir(folder.path()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the server binary runs");
    let prefix = if supervised.is_some() {
        "MICROSCOPE_READY "
    } else {
        "listening on "
    };
    // A supervisor keeps reading what the server prints.
    let stdout = BufReader::new(child.stdout.take().expect("piped"));
    let (sender, lines) = mpsc::channel();
    let reader = Some(std::thread::spawn(move || {
        for line in stdout.lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    }));
    let deadline = Instant::now() + START;
    let line = loop {
        let Ok(line) = lines.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        else {
            let _ = child.kill();
            panic!("the server didn't say where it is within {START:?}");
        };
        if let Some(line) = line.strip_prefix(prefix) {
            break line.to_owned();
        }
    };
    let stdin = child.stdin.take();
    Running {
        child,
        stdin,
        line,
        lines,
        reader,
        _folder: folder,
    }
}

impl Running {
    fn address(&self) -> String {
        let url = match serde_json::from_str::<serde_json::Value>(&self.line) {
            Ok(ready) => ready["url"].as_str().expect("a URL").to_owned(),
            Err(_) => self.line.clone(),
        };
        url.strip_prefix("http://").expect("an http URL").to_owned()
    }

    /// Sends a request, and returns the answer's status line and body.
    fn request(&self, method: &str, path: &str) -> (String, String) {
        let mut stream = TcpStream::connect(self.address()).expect("connects");
        let body = if method == "POST" { "{}" } else { "" };
        let length = body.len();
        write!(
            stream,
            "{method} /api/v1/{path} HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\n\
             Content-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        )
        .expect("writes");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("reads");
        let (head, body) = answer.split_once("\r\n\r\n").expect("an HTTP answer");
        (
            head.lines().next().unwrap_or_default().to_owned(),
            body.to_owned(),
        )
    }

    /// What the server printed after the line that said where it is, once
    /// it has stopped.
    fn printed(&mut self) -> Vec<String> {
        (self.reader.take().expect("read once").join()).expect("the reader doesn't panic");
        self.lines.try_iter().collect()
    }

    /// The exit code, which must come within `within`.
    fn exit_code(&mut self, within: Duration) -> i32 {
        let asked = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("waits") {
                return status.code().expect("an exit code");
            }
            if asked.elapsed() > within {
                let _ = self.child.kill();
                panic!("the server didn't stop within {within:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

// A server left running by a failed test is stopped.
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_managed_server_says_where_it_is_once_ready() {
    let server = start(Some("--managed"));
    let ready: serde_json::Value = serde_json::from_str(&server.line).expect("JSON");
    assert_eq!(ready["pid"], server.child.id());
    assert_eq!(ready["version"], env!("CARGO_PKG_VERSION"));
    let url = ready["url"].as_str().expect("a URL");
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    assert_ne!(url, "http://127.0.0.1:0");
    assert_eq!(server.request("GET", "system/managed").1, "true");
    assert_eq!(server.request("GET", "health").0, "HTTP/1.1 200 OK");
}

#[test]
fn closing_standard_input_stops_a_managed_server_with_0() {
    let mut server = start(Some("--managed"));
    drop(server.stdin.take());
    assert_eq!(server.exit_code(GRACE), 0);
}

#[test]
fn a_restart_stops_a_managed_server_with_75() {
    let mut server = start(Some("--managed"));
    assert_eq!(
        server.request("POST", "system/restart").0,
        "HTTP/1.1 201 Created"
    );
    assert_eq!(server.exit_code(GRACE), 75);
}

#[test]
fn a_shutdown_stops_a_managed_server_with_0() {
    let mut server = start(Some("--managed"));
    assert_eq!(
        server.request("POST", "system/shutdown").0,
        "HTTP/1.1 201 Created"
    );
    assert_eq!(server.exit_code(GRACE), 0);
}

#[test]
fn a_service_ignores_standard_input_and_restarts_with_75() {
    let mut server = start(Some("--service"));
    drop(server.stdin.take());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(server.request("GET", "system/managed").1, "true");
    assert_eq!(
        server.request("POST", "system/restart").0,
        "HTTP/1.1 201 Created"
    );
    assert_eq!(server.exit_code(GRACE), 75);
}

#[test]
fn an_unmanaged_server_ignores_standard_input_and_wont_restart() {
    let mut server = start(None);
    drop(server.stdin.take());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(server.request("GET", "system/managed").1, "false");
    let (_, invocation) = server.request("POST", "system/restart");
    let id = serde_json::from_str::<serde_json::Value>(&invocation).expect("JSON")["id"]
        .as_str()
        .expect("an id")
        .to_owned();
    std::thread::sleep(Duration::from_millis(300));
    let (_, record) = server.request("GET", &format!("action_invocations/{id}"));
    assert!(record.contains(r#""status":"error""#), "{record}");
    assert!(record.contains("supervisor"), "{record}");
    // Still serving, and it stops when asked.
    assert_eq!(
        server.request("POST", "system/shutdown").0,
        "HTTP/1.1 201 Created"
    );
    assert_eq!(server.exit_code(GRACE), 0);
    let printed = server.printed();
    assert!(
        !printed
            .iter()
            .any(|line| line.starts_with("MICROSCOPE_READY")),
        "{printed:?}"
    );
}
