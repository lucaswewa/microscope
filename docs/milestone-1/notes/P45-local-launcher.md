# P45: Local launcher

- Status: Done
- Pull request: [#58](https://github.com/lucaswewa/microscope/pull/58)
- ADRs: [ADR-0033](../../adr/0033-local-launcher-and-its-tauri-successor.md)
- Spec: [phases.md#p45](../phases.md#p45)

## Summary

`microscope-launcher`, the managed local service: one command starts the microscope and opens the app.

- **It prepares its folder,** `%LOCALAPPDATA%\Microscope`:
  - a single-instance lock;
  - a generated configuration, the simulated microscope's by default, with absolute settings, data and log folders;
  - a log of everything.
- **It starts the server** with `--managed`, on port 5000 if it's free. It waits for the ready line, then opens the browser.
- **It keeps it running:** at once after a restart, with a growing delay after a crash, and not after a stop, a configuration error or a crash loop.
- **It stops it** on Ctrl-C, Ctrl-Break or the window closing, by closing its standard input.
- **A Job Object** ends the server with the launcher, however the launcher ends.

## What was built

| Where | What |
|---|---|
| `crates/microscope-launcher/src/lib.rs` | The command line, preparing the folder, and the signals that stop the launcher |
| `crates/microscope-launcher/src/setup.rs` | The root, the generated configuration, the lock, the port, and where the server is |
| `crates/microscope-launcher/src/policy.rs` | What to do after the server exits: the restart and crash rules |
| `crates/microscope-launcher/src/supervise.rs` | Starting the server, its output, the ready line, the browser, stopping it, and the loop |
| `crates/microscope-launcher/src/job.rs` | The Job Object, through `win32job` |
| `crates/microscope-launcher/tests/launcher.rs` | The launcher with the real server |
| `Cargo.toml` | `win32job`, and tokio's `process` and `io-util` features |
| `README.md` | Running the launcher |
| `docs/adr/0033-local-launcher-and-its-tauri-successor.md` | ADR-0033 |

## How to try it

```powershell
npm --prefix web run build
cargo build -p microscope-server -p microscope-launcher
target\debug\microscope-launcher.exe --root $env:TEMP\microscope-try
```

`--root` keeps this try apart from `%LOCALAPPDATA%\Microscope`. If your own server is on port 5000, the launcher picks another port.

1. **It starts:** the server's lines scroll past, then "launcher: the microscope is at http://127.0.0.1:…", and the app opens in the browser.
2. **The folder** `%TEMP%\microscope-try` has `generated-config.json`, with absolute folders, and `settings`, `data` and `logs`, with `logs\launcher.log`.
3. **One at a time:** in another window, the same command ends at once: "another launcher is already running for …".
4. **Restart:** at the app's `/docs`, run `system/restart`. The server stops with 75, and the launcher starts it again at once. The app reconnects.
5. **A crash:** end the server in Task Manager. The launcher says "starting the server again in 1s", and does.
6. **Ctrl-C** in the launcher's window: it says "stopping the server", the server shuts down, and the launcher ends.
7. **The Job Object:** start it again, and end the launcher in Task Manager. The server ends with it.

## Design notes and deviations from the plan

- **The restart rules are a pure policy,** tested on their own: exit code 0 ends the launcher; 75 restarts at once; 2 and 3, a wrong command line or configuration, end it too, since starting again won't help. Anything else is a crash: a delay of 1 s, doubling for each crash within the last minute, and the sixth within a minute ends the launcher with 1.
- **The spec's "fake process"** is the pure policy: its tests need no process at all. The process side runs the real server in the integration tests.
- **The port** is chosen again at each start: 5000 if it's free at that moment, or else the server's choice. If another program takes 5000 in between, the server fails to start, which counts as a crash, and the next start chooses again. The integration tests met this when three launchers raced for one port, and recovered.
- **Configuration folders:** absolute paths in the given configuration are kept, and relative or missing ones become `settings`, `data` and `logs` in the root. A launcher has no working folder to be relative to.
- **The server is in its own process group,** so Ctrl-C reaches the launcher alone, which then stops the server by its standard input. Ctrl-Break and the window closing reach every process on the console, so the server may stop on its own too; either way, both end.
- **The browser** opens only the first time the server is ready, not after each restart: the app reconnects by itself (P11).
- **`win32job`** provides the Job Object safely, at the project owner's choice, so the workspace still forbids `unsafe` code. It adds twelve crates to `Cargo.lock`, all MIT OR Apache-2.0, and all released over two weeks ago.
- **A bug found by hand:** the launcher first dropped the channel for the ready line on the server's first line, so it never said where the microscope was or opened the browser. Reading only the server's ready line, the tests missed it. They now check the launcher's own line.
- **Size:** 845 changed lines of hand-written code, within L's 900 and my estimate of 700–850.

  | Part | Lines |
  |---|---|
  | `supervise.rs` | 222 |
  | `lib.rs` and `main.rs` | 140 |
  | `setup.rs`, without its tests | 78 |
  | `policy.rs`, without its tests | 53 |
  | `job.rs`, `Cargo.toml`s | 34 |
  | Tests | 318 |

## Tests

- **`policy.rs`:** a stop, restarts, which aren't crashes, a configuration error, crashes backing off until the sixth in a minute, and crashes a minute apart that don't add up.
- **`setup.rs`:** the generated folders, kept absolute ones, configurations that aren't JSON objects, the lock held by one launcher at a time, and the port when the preferred one is taken.
- **`tests/launcher.rs`,** with the real server, which the test builds once beside the launcher:
  - **keeping it running:**
    - the ready line, and the launcher's line saying where the microscope is;
    - the generated configuration;
    - a second launcher for the folder refused;
    - a restart, with a new process;
    - a crash, started again after 1 s;
    - a shutdown, which ends the launcher with 0;
    - its log;
  - **the server ends with the launcher** when the launcher is killed;
  - **a configuration error** ends the launcher with 3, without restarts.
- **By hand:** Ctrl-Break in the launcher's console stopped the server through its standard input, and the launcher ended with 0.
  - **Ctrl-C** couldn't be checked this way: processes started from my tools inherit an ignored Ctrl-C.
  - **The browser** wasn't opened in my checks, so as not to open windows on the project owner's desktop.

**Mutation checks:** I broke 11 behaviours, and a test failed for 9.

- **Caught:**
  - a restart delayed;
  - a configuration error retried;
  - no backoff;
  - crashes never forgotten;
  - a crash loop one crash early;
  - relative folders kept;
  - the lock not exclusive;
  - the preferred port taken anyway;
  - the ready line's channel dropped (the bug above).
- **Survived:**
  - **no Job Object:** a killed launcher closes the server's standard input too, so the server still ends;
  - **stopping without closing standard input:** the stop on Ctrl-C has no automated test, only the check by hand.

## Follow-ups and known gaps

- **Untested automatically:** stopping on Ctrl-C (checked by hand with Ctrl-Break), opening the browser, and ending a server that doesn't stop within 10 s.
- **The Job Object and standard input** both end the server when the launcher is killed, so the test can't tell which did.
- **A hung server** isn't restarted: the launcher only sees exits.
- **The About and Power tabs** (P46) show the managed status and offer Restart.
- **Packaging** (P49) puts the launcher, the server and the web app together.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
