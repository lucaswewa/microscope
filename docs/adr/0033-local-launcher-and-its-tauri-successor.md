# ADR-0033: Local launcher and its Tauri successor

- Status: Accepted
- Date: 2026-10-14
- Phase: P45

## Context

- **Users shouldn't need a terminal and a configuration file** to use the microscope on their own computer (plan D5). One command, or one icon, should start it and open the app.
- **The server runs for hours** and can be asked to restart (`system.restart`, exit code 75), and, like any program, it can crash.
- **The sidecar contract** (ADR-0032) is how a supervisor runs the server: `--managed`, the ready line, standard input to stop it, and exit codes.
- **A Tauri desktop app** will take the launcher's place in a later milestone. It runs the same server as a sidecar.
- **Windows' console signals** reach every process on a console, and a child left running when its parent dies would hold the port and the microscope.

## Decision

- **`microscope-launcher`,** a binary crate that depends on no internal crate and not on `teta-wot` (ADR-0005). It runs `microscope-server`, which it finds beside itself, by the sidecar contract alone.
- **Its folder,** the root, is `%LOCALAPPDATA%\Microscope` (plan §5.9), or `--root`:
  - **a lock,** `launcher.lock`, held while it runs, so a second launcher for the same root ends at once with exit code 1;
  - **`generated-config.json`:** the configuration given with `--config`, or the simulated microscope's, built into the launcher, with its settings, data and log folders made absolute. Absolute paths it gives are kept; the others become `settings`, `data` and `logs` in the root;
  - **`logs\launcher.log`:** everything the server prints, and the launcher's own lines, with their times. They go to the console too.
- **Starting the server:** on the preferred port, 5000 by default, if it's free, or else on one the server chooses (`--port 0`). The port is chosen again at each start.
  - **The ready line** must come within 60 s. Then the launcher says where the microscope is, and opens it in the browser the first time, unless `--no-browser`.
  - **The server is in its own process group,** so the console's Ctrl-C reaches the launcher alone.
  - **A Job Object** that ends its processes when it closes holds the server, so the server ends with the launcher however the launcher ends. `win32job` provides it safely, so the workspace still forbids `unsafe` code (ADR-0005).
- **After the server exits** ([`policy`](../../crates/microscope-launcher/src/policy.rs)):

  | Exit | What the launcher does |
  |---|---|
  | 0, stopped | Ends, with 0 |
  | 75, a restart | Starts it again at once |
  | 2 or 3: its command line or configuration is wrong | Ends, with the same code |
  | Anything else, or no ready line in time: a crash | Starts it again after 1 s, doubling with each crash in the last minute; the sixth in a minute ends it, with 1 |

- **Stopping:** on Ctrl-C, Ctrl-Break or the console window closing, the launcher closes the server's standard input, waits up to 10 s for it to stop, ends it if it hasn't, and ends with 0.

## Consequences

- **One command** starts the microscope and opens the app, with its files in one place that survives updates of the program.
- **A crash loop ends** rather than spinning, and its causes are in `launcher.log`.
- **The Tauri app** replaces the launcher by doing the same: its own folder and configuration, the sidecar contract, and the same restart rules.
- **A server that hangs** without exiting isn't restarted: the launcher only sees exits. A health check could come later.
- **Twelve crates join `Cargo.lock`:** `win32job`, the `windows` crates it uses, and `thiserror` 1.

## Alternatives considered

- **Windows' own service manager:** it needs installation as an administrator, runs without the user's desktop, and can't open a browser for them.
- **A small `unsafe` module calling the Win32 Job Object functions directly,** with `windows-sys` already in `Cargo.lock`: no new crates, but the first exception to `forbid(unsafe_code)`.
- **No Job Object,** relying on the server stopping when its standard input closes, which it does when the launcher dies: a server that hangs could still outlive the launcher.
- **Signals to stop the server** rather than standard input: see ADR-0032.
- **A fixed port only:** a second program on port 5000 would stop the microscope from starting.
