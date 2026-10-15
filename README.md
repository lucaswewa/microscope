# microscope

Microscope control software. A Rust backend serves the microscope's camera, stage and the routines built on them (autofocus, calibration, slide scanning, timelapse) as [W3C Web of Things](https://www.w3.org/WoT/) Things over HTTP. A Vue 3 and TypeScript web app is the user interface.

- The backend is built on [`teta-wot`](https://github.com/lucaswewa/teta-wot), a Rust framework for Web of Things servers.
- The workflows and feature set are inspired by the [OpenFlexure Microscope](https://openflexure.org/), with an original interface and an independent implementation ([ADR-0002](docs/adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md)). This project is not affiliated with the OpenFlexure project.

## Status

**Milestone 1 is complete:** the whole application, against a simulated camera and XYZ stage, whose images depend on the stage position and focus.

- **Its features match** the OpenFlexure Microscope's: live view, stage control, autofocus, calibration, slide scanning with stitching, timelapse sequences, a gallery, and the Settings, Logging, About and Power pages.
- **It ships a local launcher,** and the app connects to remote microscopes too.
- **The [plan's phase overview](docs/milestone-1/implementation-plan.md#8-phase-overview)** lists every phase, and the [retrospective](docs/milestone-1/retrospective.md) looks back on them.
- **Next:** real cameras and stages follow in milestone 2 ([readiness checklist](docs/milestone-2/readiness.md)), and the web app will later be reused in a Tauri desktop app.

Milestone 1 targets Windows x86_64 ([ADR-0003](docs/adr/0003-milestone-1-platform-windows-x86-64.md)).

## Using the microscope

### Getting it

Download `microscope-<version>-windows-x86_64.zip` from the [releases](https://github.com/lucaswewa/microscope/releases), and unzip it anywhere. It holds:

- `microscope-launcher.exe`, which you run;
- `microscope-server.exe`, which serves the microscope and the app;
- the default configuration, in `configs\`;
- this README, the licence, `NOTICE`, and the licences of the third-party software it includes.

### Running it

1. **Double-click `microscope-launcher.exe`.** A console window opens, and so does the app, in your browser, at <http://127.0.0.1:5000/>, or at another port if 5000 is taken. The window shows the server's messages; keep it open while you work.
2. **The first time,** the calibration wizard opens: follow it to calibrate the camera and how the stage moves in the image.
3. **To stop,** close the window or press Ctrl+C in it, or use the app's Power tab.
4. **Restart,** in the Power tab, starts the server again, and the app reconnects by itself. If the server ever crashes, the launcher starts it again.

Only one launcher runs at a time; a second one says so and ends.

### Where your data lives

Everything is in `%LOCALAPPDATA%\Microscope\` (paste that into File Explorer's address bar):

| Folder or file | What |
|---|---|
| `data\` | Your captures, scans and sequences, which the Gallery shows |
| `settings\` | Calibrations and settings |
| `logs\` | The server's daily logs, and `launcher.log`, everything the window showed |
| `generated-config.json` | The configuration the launcher gives the server; it's written at each start |

### Connecting from another computer

The launcher's server only answers this computer, unless you ask otherwise:

```powershell
.\microscope-launcher.exe --host 0.0.0.0
```

Then, on another computer on the same network, open `http://<this computer's name>:5000/` in a browser. From an app that's already open, use **Settings → Connection → Another microscope** with the same address, such as `lab-pc:5000`.

**There's no password:** anyone who can reach the computer can use the microscope. Do this only on a network you trust. Windows may ask whether to let the server through its firewall.

### Resetting

- **Settings and calibrations:** stop the launcher, delete `%LOCALAPPDATA%\Microscope\settings\`, and start it again. The calibration wizard opens, as the first time. Your captures and scans are kept.
- **Everything,** your captures and scans included: delete the whole `%LOCALAPPDATA%\Microscope\` folder.

### Other options

`microscope-launcher.exe --help` lists them:

- `-c <file>` uses another configuration;
- `--root <folder>` keeps the data somewhere other than `%LOCALAPPDATA%\Microscope\`;
- `--port <number>` prefers another port;
- `--no-browser` doesn't open the app.

## Documentation

- [Milestone 1 goals](docs/milestone-1/goals.md)
- [Milestone 1 implementation plan](docs/milestone-1/implementation-plan.md) and [phase specifications](docs/milestone-1/phases.md)
- [Architecture decision records](docs/adr/README.md)
- [Implementation notes](docs/milestone-1/notes/README.md), one per phase, and the [milestone's retrospective](docs/milestone-1/retrospective.md)
- [Milestone 2 readiness checklist](docs/milestone-2/readiness.md)

## Development

### Prerequisites

- Windows x86_64.
- [rustup](https://rustup.rs/), with the MSVC toolchain and Visual Studio's C++ build tools. The repository pins its compiler (Rust 1.98.1) in `rust-toolchain.toml`; `rustup toolchain install` in the repository installs it.
- [Node.js](https://nodejs.org/) 24 LTS, with npm, for the web app.

### Build and check

These are the checks CI runs:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked   # CI sets RUSTDOCFLAGS=-D warnings
```

### Run the server

```powershell
cargo run -p microscope-server -- -c configs/simulation.json
```

It listens on <http://127.0.0.1:5000/> (`--host` and `--port` change that). The API is under `/api/v1` (for example <http://127.0.0.1:5000/api/v1/health> and <http://127.0.0.1:5000/api/v1/system/>), and the interactive API docs are at <http://127.0.0.1:5000/docs>. Ctrl-C stops it gracefully. `--help` lists the options: `teta-wot`'s, and `--webapp-dir`.

The server also serves the web app, at <http://127.0.0.1:5000/> ([ADR-0016](docs/adr/0016-serving-and-embedding-the-web-app.md)). A debug build reads it from `web/dist`, so build the web app first (`npm run build` in `web/`); without a build, `/` shows a placeholder page. `--webapp-dir <path>` serves it from another folder.

### Build a release

Build the web app first, then the server, which embeds `web/dist`:

```powershell
cd web
npm ci
npm run build
cd ..
cargo build --release -p microscope-server
```

`target/release/microscope-server.exe -c configs/simulation.json` then serves the API and the app on its own.

The zip a user downloads also needs the launcher's release build and [cargo-about](https://github.com/EmbarkStudios/cargo-about), which lists the Rust crates' licences ([ADR-0035](docs/adr/0035-release-packaging-and-third-party-notices.md)):

```powershell
cargo build --release -p microscope-server -p microscope-launcher --locked
cargo install cargo-about --version 0.9.2 --features cli --locked
./release/package.ps1 -Version 0.1.0
```

It's written to `release-out\`. Pushing a tag such as `v0.1.0` has the release workflow build it, check that its launcher starts, and publish it.

### Run the launcher

The launcher is the one command a user needs ([ADR-0033](docs/adr/0033-local-launcher-and-its-tauri-successor.md)). It keeps the server's settings, data and logs in `%LOCALAPPDATA%\Microscope`, starts the server on port 5000 (or another, if that's taken), and opens the app in the browser. It starts the server again when the server asks to restart, or after a crash. Ctrl-C stops both.

```powershell
cargo build -p microscope-server -p microscope-launcher
target\debug\microscope-launcher.exe
```

It runs the `microscope-server` beside it, so build both; a release build, after the web app, gives a server with the app embedded. `-c <config>` uses another configuration, `--no-browser` doesn't open the app, `--root <folder>` keeps everything in another folder, and `--help` lists the rest. Everything the server prints also goes to `logs\launcher.log` in that folder.

Logs go to the console, to daily files in `.microscope/logs/`, and to the server log at <http://127.0.0.1:5000/api/v1/log/>; `/api/v1/logfile/` downloads today's file. `--debug` adds DEBUG events from the application and `teta-wot` ([ADR-0008](docs/adr/0008-server-logging-and-log-endpoints.md)).

The workspace's crates and the rules between them are described in [ADR-0005](docs/adr/0005-repository-layout-and-crate-boundaries.md).

### The web app

The web app is in `web/` ([ADR-0009](docs/adr/0009-frontend-toolchain.md)):

```powershell
cd web
npm ci
npm run dev          # http://localhost:5173/, with /api forwarded to the server on port 5000
```

These are the checks CI runs, plus the end-to-end tests:

```powershell
npm run format:check
npm run lint
npm run typecheck
npm run test:unit
npm run build
npx playwright install chromium   # once per machine
npm run test:e2e                  # builds the app and a server that serves it, on ports 5098 and 5099
```

The end-to-end tests' server keeps its settings, data and logs in a temporary folder, emptied for each run, not in `.microscope`. They include whole workflows, such as a histo scan, and Restart through the launcher, which they build.

The long tests run each night in CI ([`nightly.yml`](.github/workflows/nightly.yml)): a 30-minute soak, streaming and scanning, and the performance budgets. To run them, preferably against a release build:

```powershell
$env:MICROSCOPE_LONG = '1'; $env:MICROSCOPE_SOAK_MINUTES = '30'
npx playwright test --project=long   # soak.json and budgets.json are written under test-results
```

`npm run test:contract` runs the web app's WoT client against a running server, whose API root is `MICROSCOPE_API_URL` (by default `http://127.0.0.1:5000/api/v1/`). CI runs it against a fresh server.

`MICROSCOPE_API_TARGET` points the development proxy at another server, such as `http://lab-pc:5000`.

Two pages exist only in development, each showing light and dark side by side:

- <http://localhost:5173/#/dev/tokens> shows the design tokens ([ADR-0011](docs/adr/0011-design-tokens-and-theming.md));
- <http://localhost:5173/#/dev/components> shows every UI component, with their states and overlays ([ADR-0013](docs/adr/0013-own-components-on-reka-ui-primitives.md)).

The rules that keep the app ready for a Tauri desktop app are in [ADR-0010](docs/adr/0010-rules-for-a-tauri-ready-web-app.md).

### Contributing

Work proceeds in small phases, each delivered as one pull request that follows the [Definition of Done](docs/milestone-1/implementation-plan.md#7-how-phases-work). [CLAUDE.md](CLAUDE.md) sums up the working rules, for people and for Claude Code sessions.

## Licence

[MIT](LICENSE). Third-party material and its licences are listed in [NOTICE](NOTICE).
