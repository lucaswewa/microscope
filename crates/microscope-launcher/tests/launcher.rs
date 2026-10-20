//! The launcher with the real server (ADR-0033): it starts the server and
//! says where it is, starts it again after a restart and after a crash,
//! refuses a second launcher for its folder, ends when the server stops or
//! can't start, and doesn't let the server outlive it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Once;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const LAUNCHER: &str = env!("CARGO_BIN_EXE_microscope-launcher");
/// A small configuration, so the server starts quickly.
const CONFIG: &str =
    r#"{"things": {"system": "microscope.system:MicroscopeSystem"}, "api_prefix": "/api/v1"}"#;

/// Builds the server beside the launcher, once: Cargo builds only this
/// crate's binaries for its tests.
fn build_server() {
    static BUILT: Once = Once::new();
    BUILT.call_once(|| {
        let status = Command::new(env!("CARGO"))
            .args(["build", "-p", "microscope-server", "--locked"])
            .status()
            .expect("cargo runs");
        assert!(status.success(), "the server builds");
    });
}

/// A launcher, in a folder of its own, and the lines it prints.
struct Launcher {
    child: Child,
    lines: Receiver<String>,
    root: tempfile::TempDir,
}

/// Starts a launcher for `config`, preferring `port`: 0 for any.
fn launch(config: &str, port: u16) -> Launcher {
    launch_with(config, port, &[])
}

/// As `launch`, with `more` arguments.
fn launch_with(config: &str, port: u16, more: &[&str]) -> Launcher {
    build_server();
    let root = tempfile::tempdir().expect("a folder");
    let file = root.path().join("config.json");
    std::fs::write(&file, config).expect("written");
    let (child, lines) = start(root.path(), &file, port, more);
    Launcher { child, lines, root }
}

fn start(root: &Path, config: &Path, port: u16, more: &[&str]) -> (Child, Receiver<String>) {
    let mut child = Command::new(LAUNCHER)
        .arg("--root")
        .arg(root)
        .arg("-c")
        .arg(config)
        .args(["--port", &port.to_string(), "--no-browser"])
        .args(more)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the launcher runs");
    let stdout = BufReader::new(child.stdout.take().expect("piped"));
    let (sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in stdout.lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });
    (child, lines)
}

impl Launcher {
    /// The next line that starts with `prefix`, without it.
    fn next(&self, prefix: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = (self.lines.recv_timeout(left)).unwrap_or_else(|_| panic!("no {prefix:?}"));
            if let Some(rest) = line.strip_prefix(prefix) {
                return rest.to_owned();
            }
        }
    }

    /// The next ready line's address and process ID.
    fn ready(&self) -> (String, u32) {
        let ready: serde_json::Value =
            serde_json::from_str(&self.next("MICROSCOPE_READY ")).expect("JSON");
        let url = ready["url"].as_str().expect("a URL");
        let address = url.strip_prefix("http://").expect("http").to_owned();
        (address, ready["pid"].as_u64().expect("a pid") as u32)
    }

    fn exit_code(&mut self, within: Duration) -> i32 {
        let deadline = Instant::now() + within;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait().expect("waits") {
                return status.code().expect("an exit code");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the launcher didn't end within {within:?}");
    }
}

impl Drop for Launcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// POSTs to one of the server's actions, and returns the status line.
fn post(address: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("connects");
    write!(
        stream,
        "POST /api/v1/{path} HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\n\
         Content-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    )
    .expect("writes");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("reads");
    answer.lines().next().unwrap_or_default().to_owned()
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
    let filter = format!("PID eq {pid}");
    let output = (Command::new("tasklist")
        .args(["/FI", &filter, "/NH"])
        .output())
    .expect("runs");
    String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
    // A zombie has ended, and only waits for its parent to notice.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
    stat.is_ok_and(|stat| {
        !stat
            .rsplit(')')
            .next()
            .is_some_and(|s| s.trim_start().starts_with('Z'))
    })
}

/// Ends process `pid` at once, as a crash would.
fn kill(pid: u32) {
    let (program, options) = if cfg!(windows) {
        ("taskkill", ["/F", "/PID"])
    } else {
        ("kill", ["-s", "KILL"])
    };
    let killed = (Command::new(program).args(options).arg(pid.to_string()))
        .stdout(Stdio::null())
        .status();
    assert!(killed.expect("runs").success());
}

#[test]
fn it_keeps_the_server_running_until_it_stops() {
    // A port kept for these checks: the other tests take any, so as not to
    // race for it.
    let mut launcher = launch(CONFIG, 5097);
    let (address, first) = launcher.ready();
    // It says where the microscope is, as it would open it in a browser.
    let at = launcher.next("launcher: the microscope is at ");
    assert_eq!(at, format!("http://{address}"));
    // The configuration it gave the server, with absolute folders.
    let root = launcher.root.path().to_owned();
    let generated = std::fs::read_to_string(root.join("generated-config.json")).expect("read");
    let generated: serde_json::Value = serde_json::from_str(&generated).expect("JSON");
    assert_eq!(
        generated["settings_folder"],
        serde_json::json!(root.join("settings"))
    );
    // One launcher at a time for a folder: a second ends at once.
    let mut second = Launcher {
        child: (Command::new(LAUNCHER).arg("--root").arg(&root))
            .args(["--port", "0", "--no-browser"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("runs"),
        lines: mpsc::channel().1,
        root: tempfile::tempdir().expect("a folder"),
    };
    assert_eq!(second.exit_code(Duration::from_secs(10)), 1);
    let mut said = String::new();
    let stderr = second.child.stderr.take().expect("piped");
    BufReader::new(stderr)
        .read_to_string(&mut said)
        .expect("read");
    assert!(said.contains("already running"), "{said}");

    // A restart: started again at once.
    assert_eq!(post(&address, "system/restart"), "HTTP/1.1 201 Created");
    let (_, restarted) = launcher.ready();
    assert_ne!(restarted, first);
    // A crash: started again after a second.
    kill(restarted);
    assert_eq!(
        launcher.next("launcher: starting the server again in "),
        "1s"
    );
    let (address, crashed) = launcher.ready();
    assert_ne!(crashed, restarted);

    // A shutdown ends the launcher too.
    assert_eq!(post(&address, "system/shutdown"), "HTTP/1.1 201 Created");
    assert_eq!(launcher.exit_code(Duration::from_secs(15)), 0);
    let log = std::fs::read_to_string(root.join("logs/launcher.log")).expect("read");
    assert_eq!(log.matches("MICROSCOPE_READY").count(), 3, "{log}");
}

#[test]
fn a_restart_keeps_the_port_the_server_chose() {
    // The preferred port is taken, so the server chooses one; after a
    // restart, the app finds it there again.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("binds");
    let port = taken.local_addr().expect("an address").port();
    let launcher = launch(CONFIG, port);
    let (address, _) = launcher.ready();
    assert!(!address.ends_with(&format!(":{port}")), "{address}");
    assert_eq!(post(&address, "system/restart"), "HTTP/1.1 201 Created");
    assert_eq!(launcher.ready().0, address);
}

#[test]
fn the_server_listens_where_asked() {
    // IPv6's loopback, which the default, 127.0.0.1, isn't.
    let launcher = launch_with(CONFIG, 0, &["--host", "[::1]"]);
    let (address, _) = launcher.ready();
    assert!(address.starts_with("[::1]:"), "{address}");
}

#[test]
fn the_server_doesnt_outlive_the_launcher() {
    let mut launcher = launch(CONFIG, 0);
    let (_, pid) = launcher.ready();
    assert!(alive(pid));
    launcher.child.kill().expect("killed");
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) {
        assert!(
            Instant::now() < deadline,
            "the server outlived the launcher"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(unix)]
#[test]
fn sigterm_stops_the_launcher_and_its_server() {
    let mut launcher = launch(CONFIG, 0);
    let (_, pid) = launcher.ready();
    let launcher_pid = launcher.child.id().to_string();
    let sent = Command::new("kill").args(["-TERM", &launcher_pid]).status();
    assert!(sent.expect("runs").success());
    assert_eq!(launcher.exit_code(Duration::from_secs(15)), 0);
    assert!(!alive(pid), "the server outlived the launcher");
}

#[test]
fn a_server_that_cant_start_isnt_started_again() {
    let mut launcher = launch(r#"{"things": {"x": "no.such:Thing"}}"#, 0);
    assert_eq!(launcher.exit_code(Duration::from_secs(30)), 3);
}
