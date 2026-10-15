//! `microscope-launcher`: the managed local service (ADR-0033). One command
//! starts the microscope and opens the app.
//!
//! ```text
//! microscope-launcher [-c config.json] [--root PATH] [--port 5000] [--host 127.0.0.1] [--server PATH] [--no-browser]
//! ```
//!
//! - **It prepares** the root folder, `%LOCALAPPDATA%\Microscope` by
//!   default, and holds a lock there so only one launcher uses it. It writes
//!   `generated-config.json` there: the given configuration, or the
//!   simulated microscope's, with absolute settings, data and log folders.
//! - **It starts** `microscope-server` with `--managed` (ADR-0032), on the
//!   preferred port if it's free and on one the server chooses if not, and
//!   opens the app once the server says it's ready.
//! - **It keeps it running:** at once after a restart (exit code 75), after
//!   a growing delay after a crash, and not after it stops (0), can't start
//!   (2 or 3), or crashes too often ([`policy`]).
//! - **It stops it** on Ctrl-C, or when its window closes, by closing the
//!   server's standard input. A Windows Job Object ends the server if the
//!   launcher ends any other way.
//! - **Everything the server prints** goes to the console and to
//!   `logs\launcher.log` in the root.

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use tokio::sync::watch;

#[cfg(windows)]
mod job;
pub mod policy;
pub mod setup;
pub mod supervise;

/// The command-line options.
#[derive(Debug, Parser)]
#[command(
    name = "microscope-launcher",
    version,
    about = "Start the microscope server, keep it running, and open the app."
)]
pub struct Args {
    /// The server's configuration: by default, the simulated microscope's.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Where the settings, data and logs go: by default,
    /// `%LOCALAPPDATA%\Microscope`.
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,
    /// The port to prefer. If it's taken, the server chooses another.
    #[arg(long, default_value_t = 5000)]
    pub port: u16,
    /// The address the server listens on: this computer alone by default.
    /// `0.0.0.0` lets other computers reach it, with no password, so only on
    /// a network you trust.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// The server binary: by default, `microscope-server` beside the
    /// launcher.
    #[arg(long, value_name = "PATH")]
    pub server: Option<PathBuf>,
    /// Don't open the app in a browser.
    #[arg(long)]
    pub no_browser: bool,
}

/// Parses the command line, then runs the server until it stops or the
/// launcher is stopped, and returns the exit code.
pub async fn main() -> ExitCode {
    let args = Args::parse();
    let (stopping, stop) = watch::channel(false);
    tokio::spawn(async move {
        stopped().await;
        let _ = stopping.send(true);
    });
    match prepare(args) {
        Ok(options) => supervise::run(options, stop).await,
        Err(message) => {
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The root folder, its lock (held for the launcher's life), the generated
/// configuration and the log.
fn prepare(args: Args) -> Result<supervise::Options, String> {
    let root = args.root.unwrap_or_else(setup::default_root);
    let logs = root.join("logs");
    std::fs::create_dir_all(&logs).map_err(|e| format!("can't create {}: {e}", logs.display()))?;
    let root = std::path::absolute(&root).map_err(|e| e.to_string())?;
    std::mem::forget(setup::lock(&root)?);
    let text = match &args.config {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("can't read {}: {e}", path.display()))?,
        None => setup::SIMULATION.to_owned(),
    };
    let config = setup::resolve(&text, &root)?;
    let generated = root.join("generated-config.json");
    let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&generated, text)
        .map_err(|e| format!("can't write {}: {e}", generated.display()))?;
    let file = (OpenOptions::new().create(true).append(true))
        .open(logs.join("launcher.log"))
        .map_err(|e| format!("can't open the launcher's log: {e}"))?;
    Ok(supervise::Options {
        server: args.server.map_or_else(setup::default_server, Ok)?,
        config: generated,
        port: args.port,
        host: args.host,
        open_browser: !args.no_browser,
        log: supervise::Log::new(file),
    })
}

/// Completes on Ctrl-C or Ctrl-Break, or when the console window closes.
async fn stopped() {
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_close};
        let (mut close, mut brk) = (ctrl_close().ok(), ctrl_break().ok());
        tokio::select! {
            Ok(()) = tokio::signal::ctrl_c() => {}
            Some(()) = async { close.as_mut()?.recv().await } => {}
            Some(()) = async { brk.as_mut()?.recv().await } => {}
            else => std::future::pending().await,
        }
    }
    #[cfg(not(windows))]
    let _ = tokio::signal::ctrl_c().await;
}
