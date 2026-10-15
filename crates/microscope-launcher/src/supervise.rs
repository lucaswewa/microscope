//! Running the server by the sidecar contract (ADR-0032): start it, wait
//! for its ready line, start it again by the [`Policy`], and stop it by
//! closing its standard input.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, watch};

use crate::policy::{Next, Policy};
use crate::setup;

/// How long the server may take to say it's ready.
pub const READY_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the server may take to stop once its standard input closes:
/// its grace period, and a little more.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// What the launcher runs, and how.
pub struct Options {
    /// The server binary.
    pub server: PathBuf,
    /// The configuration the launcher generated for it.
    pub config: PathBuf,
    /// The port to prefer.
    pub port: u16,
    /// Whether to open the app in a browser once the server is first ready.
    pub open_browser: bool,
    /// The launcher's log.
    pub log: Log,
}

/// The launcher's log: each line goes to the console and, with the time, to
/// a file.
#[derive(Clone)]
pub struct Log(Arc<Mutex<File>>);

impl Log {
    /// A log appended to `file`.
    pub fn new(file: File) -> Log {
        Log(Arc::new(Mutex::new(file)))
    }

    /// Writes `line`, to standard error if `error`.
    pub fn line(&self, line: &str, error: bool) {
        if error {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
        let time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let mut file = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(file, "{time} {line}");
    }

    /// Writes one of the launcher's own lines.
    pub fn say(&self, message: &str) {
        self.line(&format!("launcher: {message}"), false);
    }
}

/// Runs the server until it stops, the launcher is stopped, or it crashes
/// too often, and returns the launcher's exit code.
pub async fn run(options: Options, mut stop: watch::Receiver<bool>) -> ExitCode {
    let log = &options.log;
    let mut policy = Policy::default();
    let mut opened = !options.open_browser;
    // Once the server has served on a port, it's preferred, so the app
    // finds it there again after a restart.
    let mut port = options.port;
    loop {
        let code = match serve_once(&options, &mut port, &mut stop, &mut opened).await {
            Ok(Some(code)) => code,
            Ok(None) => return ExitCode::SUCCESS,
            Err(message) => {
                log.say(&message);
                None
            }
        };
        match policy.after_exit(code, Instant::now()) {
            Next::Stop(0) => {
                log.say("the server has stopped");
                return ExitCode::SUCCESS;
            }
            Next::Stop(code) => {
                log.say(&format!(
                    "the server can't be kept running (exit code {code})"
                ));
                return ExitCode::from(code);
            }
            Next::Restart(delay) => {
                log.say(&format!("starting the server again in {delay:?}"));
                tokio::select! {
                    () = tokio::time::sleep(delay) => {}
                    _ = stop.wait_for(|stop| *stop) => return ExitCode::SUCCESS,
                }
            }
        }
    }
}

/// Starts the server once, on `preferred` if it's free, and runs it until it
/// exits, returning its exit code, or `None` once the launcher has stopped
/// it. The port it serves on becomes `preferred`.
async fn serve_once(
    options: &Options,
    preferred: &mut u16,
    stop: &mut watch::Receiver<bool>,
    opened: &mut bool,
) -> Result<Option<Option<i32>>, String> {
    let log = &options.log;
    let port = setup::port(*preferred);
    let mut command = Command::new(&options.server);
    command
        .arg("-c")
        .arg(&options.config)
        .args(["--managed", "--port", &port.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // Ctrl-C reaches the launcher alone: it stops the server itself.
    #[cfg(windows)]
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    let mut child =
        (command.spawn()).map_err(|e| format!("can't start {}: {e}", options.server.display()))?;
    #[cfg(windows)]
    let _job = crate::job::contain(&child)?;
    let mut stdin = child.stdin.take();
    let (ready, mut url) = oneshot::channel();
    forward(child.stdout.take(), log.clone(), false, Some(ready));
    forward(child.stderr.take(), log.clone(), true, None);

    let deadline = tokio::time::sleep(READY_TIMEOUT);
    tokio::pin!(deadline);
    let mut waiting = true;
    loop {
        tokio::select! {
            status = child.wait() => {
                return Ok(Some(status.map_err(|e| e.to_string())?.code()));
            }
            // Without a ready line, the server's exit follows.
            address = &mut url, if waiting => {
                waiting = false;
                let Ok(address) = address else { continue };
                log.say(&format!("the microscope is at {address}"));
                let served = address.rsplit(':').next().and_then(|port| port.parse().ok());
                *preferred = served.unwrap_or(*preferred);
                if !std::mem::replace(opened, true) {
                    open(&address, log);
                }
            }
            () = &mut deadline, if waiting => {
                log.say(&format!("the server wasn't ready within {READY_TIMEOUT:?}"));
                let _ = child.kill().await;
                return Ok(Some(None));
            }
            _ = stop.wait_for(|stop| *stop) => {
                log.say("stopping the server");
                drop(stdin.take());
                shut(&mut child, log).await;
                return Ok(None);
            }
        }
    }
}

/// Waits for `child` to stop once its standard input has closed, and kills
/// it if it doesn't.
async fn shut(child: &mut Child, log: &Log) {
    if tokio::time::timeout(STOP_TIMEOUT, child.wait())
        .await
        .is_err()
    {
        log.say(&format!(
            "the server didn't stop within {STOP_TIMEOUT:?}, so it was ended"
        ));
        let _ = child.kill().await;
    }
}

/// Copies `stream`'s lines to the log, and sends the URL in the ready line
/// to `ready`.
fn forward(
    stream: Option<impl AsyncRead + Unpin + Send + 'static>,
    log: Log,
    error: bool,
    mut ready: Option<oneshot::Sender<String>>,
) {
    let Some(stream) = stream else { return };
    tokio::spawn(async move {
        let mut lines = BufReader::new(stream).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log.line(&line, error);
            let url = (line.strip_prefix("MICROSCOPE_READY "))
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
                .and_then(|ready| Some(ready["url"].as_str()?.to_owned()));
            if let Some(url) = url
                && let Some(ready) = ready.take()
            {
                let _ = ready.send(url);
            }
        }
    });
}

/// Opens `url` in the default browser.
fn open(url: &str, log: &Log) {
    #[cfg(windows)]
    let opened = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .stdout(Stdio::null())
        .spawn();
    #[cfg(not(windows))]
    let opened = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(e) = opened {
        log.say(&format!("can't open a browser ({e}): open {url} yourself"));
    }
}

/// The process creation flag that keeps the console's Ctrl-C from the server.
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
