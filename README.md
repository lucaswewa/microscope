# microscope

Microscope control software. A Rust backend serves the microscope's camera, stage and the routines built on them (autofocus, calibration, slide scanning, timelapse) as [W3C Web of Things](https://www.w3.org/WoT/) Things over HTTP. A Vue 3 and TypeScript web app is the user interface.

- The backend is built on [`teta-wot`](https://github.com/lucaswewa/teta-wot), a Rust framework for Web of Things servers.
- The workflows and feature set are inspired by the [OpenFlexure Microscope](https://openflexure.org/), with an original interface and an independent implementation ([ADR-0002](docs/adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md)). This project is not affiliated with the OpenFlexure project.

## Status

Early development; there is nothing to run yet.

**Milestone 1** builds the application against a simulated camera and XYZ stage, whose images depend on the stage position and focus. It aims for feature parity with the OpenFlexure Microscope: live view, stage control, autofocus, calibration, slide scanning with stitching, timelapse sequences, a gallery, and the Settings, Logging, About and Power pages. It also ships a local launcher and supports connecting to remote microscopes. Real cameras and stages follow in milestone 2, and the web app will later be reused in a Tauri desktop app.

Milestone 1 targets Windows x86_64 ([ADR-0003](docs/adr/0003-milestone-1-platform-windows-x86-64.md)).

## Documentation

- [Milestone 1 goals](docs/milestone-1/goals.md)
- [Milestone 1 implementation plan](docs/milestone-1/implementation-plan.md) and [phase specifications](docs/milestone-1/phases.md)
- [Architecture decision records](docs/adr/README.md)
- [Implementation notes](docs/milestone-1/notes/README.md), one per phase

## Contributing

Work proceeds in small phases, each delivered as one pull request that follows the [Definition of Done](docs/milestone-1/implementation-plan.md#7-how-phases-work). Build and test instructions arrive with the first code phases (P01 for Rust, P04 for the web app).

## Licence

[MIT](LICENSE)
