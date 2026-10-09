# microscope

Microscope control software. A Rust backend serves the microscope's camera, stage and the routines built on them (autofocus, calibration, slide scanning, timelapse) as [W3C Web of Things](https://www.w3.org/WoT/) Things over HTTP. A Vue 3 and TypeScript web app is the user interface.

- The backend is built on [`teta-wot`](https://github.com/lucaswewa/teta-wot), a Rust framework for Web of Things servers.
- The workflows and feature set are inspired by the [OpenFlexure Microscope](https://openflexure.org/), with an original interface and an independent implementation ([ADR-0002](docs/adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md)). This project is not affiliated with the OpenFlexure project.

## Status

Early development. The server runs with a placeholder System Thing, logging and its API docs. The web app has its shell: the navigation rail, light, dark and system themes, and the UI components the pages will be built from. The pages themselves come in later phases. The [plan's phase overview](docs/milestone-1/implementation-plan.md#8-phase-overview) shows where each phase stands.

**Milestone 1** builds the application against a simulated camera and XYZ stage, whose images depend on the stage position and focus. It aims for feature parity with the OpenFlexure Microscope: live view, stage control, autofocus, calibration, slide scanning with stitching, timelapse sequences, a gallery, and the Settings, Logging, About and Power pages. It also ships a local launcher and supports connecting to remote microscopes. Real cameras and stages follow in milestone 2, and the web app will later be reused in a Tauri desktop app.

Milestone 1 targets Windows x86_64 ([ADR-0003](docs/adr/0003-milestone-1-platform-windows-x86-64.md)).

## Documentation

- [Milestone 1 goals](docs/milestone-1/goals.md)
- [Milestone 1 implementation plan](docs/milestone-1/implementation-plan.md) and [phase specifications](docs/milestone-1/phases.md)
- [Architecture decision records](docs/adr/README.md)
- [Implementation notes](docs/milestone-1/notes/README.md), one per phase

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

It listens on <http://127.0.0.1:5000/> (`--host` and `--port` change that). The API is under `/api/v1` (for example <http://127.0.0.1:5000/api/v1/health> and <http://127.0.0.1:5000/api/v1/system/>), and the interactive API docs are at <http://127.0.0.1:5000/docs>. Ctrl-C stops it gracefully. `--help` lists the options; they are `teta-wot`'s.

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
npm run test:e2e
```

`MICROSCOPE_API_TARGET` points the development proxy at another server, such as `http://lab-pc:5000`.

Two pages exist only in development, each showing light and dark side by side:

- <http://localhost:5173/#/dev/tokens> shows the design tokens ([ADR-0011](docs/adr/0011-design-tokens-and-theming.md));
- <http://localhost:5173/#/dev/components> shows every UI component, with their states and overlays ([ADR-0013](docs/adr/0013-own-components-on-reka-ui-primitives.md)).

The rules that keep the app ready for a Tauri desktop app are in [ADR-0010](docs/adr/0010-rules-for-a-tauri-ready-web-app.md).

### Contributing

Work proceeds in small phases, each delivered as one pull request that follows the [Definition of Done](docs/milestone-1/implementation-plan.md#7-how-phases-work). [CLAUDE.md](CLAUDE.md) sums up the working rules, for people and for Claude Code sessions.

## Licence

[MIT](LICENSE). Third-party material and its licences are listed in [NOTICE](NOTICE).
