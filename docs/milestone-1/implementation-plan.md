# Milestone 1 implementation plan

| | |
|---|---|
| Status | Approved 2026-10-08; a living document, updated as phases land (Appendix D) |
| Date | 2026-10-08 |
| Covers | Milestone 1: a simulated microscope with OpenFlexure-level features |
| Inputs | [`goals.md`](goals.md), the planning Q&A (recorded in [§3](#3-decisions-made-while-planning)), a study of `../teta-wot` (v0.1.0) and `../openflexure-microscope-server` (v3 branch) |
| Companion | [phases.md](phases.md): the specification of every phase |

## Contents

1. [Milestone 1 in one page](#1-milestone-1-in-one-page)
2. [In and out of scope](#2-in-and-out-of-scope)
3. [Decisions made while planning](#3-decisions-made-while-planning)
4. [Ground rules](#4-ground-rules)
5. [Architecture](#5-architecture)
6. [Feature parity with OpenFlexure](#6-feature-parity-with-openflexure)
7. [How phases work](#7-how-phases-work)
8. [Phase overview](#8-phase-overview)
9. [Documentation conventions](#9-documentation-conventions)
10. [Risks](#10-risks)
11. [Points for your review](#11-points-for-your-review)
12. [Appendices](#12-appendices)

---

## 1. Milestone 1 in one page

Milestone 1 builds a microscope application for Windows. The backend is a Rust service on [`teta-wot`](https://github.com/lucaswewa/teta-wot). The front end is an original Vue 3 and TypeScript app whose workflows follow the OpenFlexure Microscope (v3). The milestone uses no real hardware. Instead, a simulated camera and XYZ stage produce images that depend on where the stage is and how well it is focused. The live preview, autofocus, calibrations, scans and stitched mosaics therefore run the same code paths that the real drivers will use in milestone 2.

At the end of milestone 1:

- **`microscope-launcher`** starts the backend on a free port and opens the browser. It restarts the backend when asked, or when it crashes.
- **The web app** connects either to that local service or to a remote microscope by URL. Its navigation rail offers View, Control, Slide Scan, Sequence and Gallery at the top, with Settings, Logging, About and Power below. It has Dark, Light and Follow System themes, and it remembers the choice.
- **The microscope can** stream live images, jog and focus, click-to-move, autofocus and capture. It can run the first-run calibration wizard (camera calibration, camera–stage mapping, stage direction) and the stage measurement tools. It scans slides with four workflows (Histo smart spiral, Snake, Raster, C-Chip) using smart z-stacks. It produces live and final stitched mosaics, and you can browse everything in a gallery with a deep-zoom viewer. It also runs timelapse sequences, whose model is ready for z-stacks and multiple positions.
- **The simulator** renders procedural tissue, procedural blobs, or a loaded image. You can switch on backlash, travel limits, a tilted or uneven focal plane, and optical effects (vignetting, uneven illumination, noise) while it runs.
- **The web app is ready for Tauri**: it uses hash routing, makes no same-origin assumptions, keeps platform services behind a host adapter, and fetches its streams with `fetch` so that it can carry auth tokens later.
- **Every phase** leaves implementation notes, and ADRs for any decisions it made, under [`docs/`](../README.md).

The work is split into **50 phases (P00–P49)** in **9 stages**. Five demo checkpoints fall along the way ([§8](#8-phase-overview)).

## 2. In and out of scope

**In scope**

- A Rust backend on `teta-wot`. Thing and affordance names mirror OpenFlexure's where that is practical.
- A simulated camera, stage and illumination, with the image simulator described in [§5.7](#57-simulator).
- OpenFlexure-parity features for View, Control, Slide Scan, Sequence (timelapse), Gallery, Settings, the calibration wizard, Logging, About and Power ([§6](#6-feature-parity-with-openflexure)).
- Stitching that places tiles by stage position, with a live preview, a final mosaic and a deep-zoom (DZI) pyramid.
- A local launcher (a supervised "managed" service) and remote connections by URL.
- Light, dark and system themes, with visual review against OpenFlexure reference screenshots.
- Windows x86_64 builds and CI.

**Out of scope, and where it goes**

| Item | Where |
|---|---|
| Real camera and stage drivers: OpenFlexure (Sangaboard and Pi camera), industrial camera SDKs, µm-native motorised stages | Milestone 2. Milestone 1 designs the interfaces for them ([§5.8](#58-hardware-abstraction-and-units)) |
| Refining stitching by image correlation | Milestone 2, when real stages drift. The registration primitive already exists from P22 |
| Pyramidal TIFF output (OpenFlexure's `stitch_tiff`) | Deferred. DZI and an overview JPEG cover viewing and download |
| The Tauri desktop app | A later milestone, reusing this web app and the launcher's sidecar contract |
| Authentication | Later. Milestone 1 assumes a trusted LAN and keeps the client token-ready |
| Linux and Raspberry Pi | Revisit in milestone 2. Code stays portable, but CI targets Windows only |
| mDNS discovery (OpenFlexure Connect-style) | Later. `teta-wot`'s advertiser is internal to its own `serve`, which we don't use ([ADR-0006](#adr-plan)) |
| OpenFlexure's legacy v2 API and server-defined extension tabs | Not planned |
| Z-stack and multi-position sequences | After milestone 1. The sequence model leaves room for them |

## 3. Decisions made while planning

These came from the planning Q&A. Each one is recorded as an ADR in the phase named.

| # | Topic | Decision | Recorded in |
|---|---|---|---|
| D1 | Sequence tab | Timelapse parity now (interval, duration, optional autofocus per capture, results in Gallery), modelled as a step list so z-stacks and positions can be added without a rewrite | ADR-0031 (P41) |
| D2 | Stitching | Place tiles by stage position through the camera–stage mapping: a live preview during scans, then a final mosaic and a DZI pyramid. Correlation refinement waits for milestone 2 | ADR-0030 (P38) |
| D3 | Scan workflows | Histo smart spiral, Snake, Raster and C-Chip | ADR-0029 (P35) |
| D4 | Extras | Calibration wizard, smart z-stacks, deep-zoom gallery viewer and stage measurement tools are all in | P29, P36, P33, P30 |
| D5 | Managed local service | A Rust launcher in milestone 1 that starts and restarts the backend, picks a port and opens the browser. Tauri takes its place later using the same sidecar contract | ADR-0032 (P44), ADR-0033 (P45) |
| D6 | `teta-wot` | No upstream changes. This app owns the server lifecycle around `teta-wot`'s public runtime and router, and pins the `v0.1.0` tag | ADR-0004 (P00), ADR-0006 (P02) |
| D7 | API naming | Mirror OpenFlexure's Thing and affordance names (`camera`, `stage`, `autofocus`, `camera_stage_mapping`, `smart_scan`, `gallery`, …) where they fit, without promising compatibility | ADR-0007 (P02) |
| D8 | Security | None in milestone 1 (trusted LAN). The client reads streams and SSE with `fetch()` so that a bearer token can be added later without rework | ADR-0014 (P10) |
| D9 | Platform | Windows x86_64 for milestone 1. Revisit at milestone 2 | ADR-0003 (P00) |
| D10 | Milestone-2 hardware | OpenFlexure hardware, industrial camera SDKs and µm-native motorised stages. The interfaces must fit all three | ADR-0018 (P15) |
| D11 | Simulated samples | Procedural tissue, procedural blobs and image-backed samples | P13, P25 |
| D12 | Simulated imperfections | Stage backlash, a tilted or uneven focal plane, travel limits and optical effects, each switchable at runtime | P14, P23, P25 |
| D13 | Units | Thing APIs use integer steps per axis. Each stage declares µm per step so that the UI and metadata can show physical units | ADR-0019 (P15) |
| D14 | UI components | Original components on our own theme tokens. Headless Reka UI (MIT) handles the accessibility-heavy widgets. Icons are Material Symbols (Apache-2.0) | ADR-0012 (P07), ADR-0013 (P08) |
| D15 | Reference screenshots | Captured from OpenFlexure's simulation server in both themes, and kept out of git | P05 |
| D16 | Delivery | One branch and pull request per phase. A pull request is merged only after the project owner approves it, and the next phase starts only once it is merged | [§7](#7-how-phases-work) |

## 4. Ground rules

### 4.1 Independent implementation

OpenFlexure is **GPL-3.0**; this repository is **MIT**. The plan uses OpenFlexure as a behavioural reference, not as a source.

- **Do reuse** the ideas, workflows, layout proportions and interface names (names are mirrored on purpose, for familiarity: D7).
- **Don't copy** code, comments, styles (LESS/CSS), icons, images, sample sprites or UI copy, and don't port files line by line.
- **Spec first for algorithm phases.** Autofocus, camera–stage mapping, background detection, stage measurement, planners, smart stacks and sequences each start their notes with a behaviour spec written in our own words. The implementation is written from that spec.
- **Licences of dependencies** must allow MIT distribution: MIT, Apache-2.0, BSD, ISC or Zlib. MPL-2.0 is acceptable only as an unmodified development dependency (for example axe-core). OpenSeadragon (BSD-3-Clause), Reka UI (MIT) and Material Symbols (Apache-2.0) qualify.
- **Reference screenshots** of OpenFlexure stay out of git (P05).
- Every pull request ticks an "independent implementation" box (P00 adds it to the PR template).

This is an engineering policy, not legal advice. ADR-0002 records it.

### 4.2 A Tauri-ready web app

ADR-0010 records these rules:

- **Hash routing** (`/#/control`), which works under Tauri's custom protocol, on any static host, and behind `teta-wot`'s fallback router ([§5.4](#54-serving-and-lifecycle)).
- **Every server URL** comes from the active connection's base URL plus the forms in the Thing Descriptions. `window.location` is used only to propose the default "this server" connection.
- **Platform services** go through a `HostAdapter`: opening links, saving files, app information and service control. Milestone 1 ships the browser implementation; Tauri adds its own later.
- **No CDN assets**: fonts, icons and viewers are bundled, so the app works offline in Tauri.
- **Streams and SSE use `fetch()`**, never `<img src>` or `EventSource`, so that headers (auth) can be added later.

### 4.3 Mirror OpenFlexure where practical

Use OpenFlexure's names, defaults and flows (port 5000, overlap 0.35, autofocus range 2000, 18 gallery cards per page, …) unless there's a reason not to. When a phase deviates, its notes say why.

### 4.4 Small, self-contained phases

Each phase does one job end to end (code, tests, docs) and can be reviewed in one sitting ([§7](#7-how-phases-work)).

## 5. Architecture

### 5.1 Context

```text
   Browser tab (Vue app)                      Browser on another PC, or the Tauri app (later)
          │ same origin                                  │ cross-origin (CORS allowed by teta-wot)
          ▼                                              ▼
 microscope-launcher ──spawns──►  microscope-server   (one process per microscope)
   picks a port, opens the          ├─ outer router  /                      embedded web app
   browser, restarts on request     │                /api/v1/health, log, logfile, data
   or crash (P45)                   │                anything else ──► teta-wot router
                                    └─ teta-wot runtime: Things
                                         system, camera, stage, illumination, autofocus,
                                         camera_stage_mapping, background detectors, stage_measure,
                                         gallery, smart_scan + workflows, sequence
                                           └─ microscope-sim: specimens, optics, stage model
```

### 5.2 Repository layout

```text
microscope/
├─ Cargo.toml, rust-toolchain.toml   workspace; toolchain pinned to teta-wot's (1.98.1)
├─ crates/
│  ├─ microscope-core/       algorithms without I/O: geometry and units, image registration,
│  │                         camera–stage mapping maths, focus metrics, background detection,
│  │                         scan planners, mosaic and DZI writer
│  ├─ microscope-sim/        image simulator and stage model (does not depend on teta-wot)
│  ├─ microscope-things/     teta-wot Things, hardware interfaces, server-described UI model
│  ├─ microscope-server/     binary: CLI, lifecycle, app routes, logging, embedded web app
│  └─ microscope-launcher/   binary: local supervisor
├─ configs/                  server configuration files (simulation.json, …)
├─ web/                      Vue 3 + TypeScript app (see §5.10)
├─ docs/                     plan, ADRs, phase notes, reference-capture guide
└─ .github/                  CI workflows, pull request template
```

Keeping the algorithms (`microscope-core`) and the simulator (`microscope-sim`) free of `teta-wot` means they can be tested quickly without a server, and milestone 2 can reuse them unchanged.

### 5.3 Things

The names mirror OpenFlexure's (D7). `teta-wot` Things can't be generic, so each driver gets its own Thing type that implements a shared interface ([§5.8](#58-hardware-abstraction-and-units)).

| Thing name | Milestone-1 type | Main affordances | Phases |
|---|---|---|---|
| `system` | `MicroscopeSystem` | `hostname`, `version_data`, `os_version`, `managed`; `shutdown`, `restart` | P02, P44 |
| `stage` | `SimulatedStage` (`StageApi`) | `position` (observable), `moving`, `axis_names`, `um_per_step`, `backlash_steps`, `axis_inverted`; `move_relative`, `move_absolute`, `move_to_origin`, `set_zero_position`, `jog`, `invert_axis_direction`, `stop` | P16 |
| `camera` | `SimulatedCamera` (`CameraApi`) | `mjpeg_stream`, `lores_mjpeg_stream`, streaming modes, manual settings, calibration properties; `capture`, `grab_jpeg`, `capture_to_memory`, `save_from_memory`, `settle`, `full_auto_calibrate`, `set_background`, `image_is_sample`, `load_sample`, `remove_sample` | P17, P25–P27 |
| `illumination` | `SimulatedIllumination` | `set_led`, `flash` | P17 |
| `autofocus` | `Autofocus` | `fast_autofocus`, `looping_autofocus`, `z_move_and_measure_sharpness`, `run_smart_stack`, `run_basic_stack` | P21, P36 |
| `camera_stage_mapping` | `CameraStageMapping` | `image_to_stage_displacement_matrix`, `image_resolution`, `last_calibration`; `calibrate_xy`, `move_in_image_coordinates`, `convert_*` | P23 |
| `bg_color_channels_luv`, `bg_channel_deviations_luv` | background detectors | tolerance, minimum sample coverage, `settings_ui` | P27 |
| `stage_measure` | `StageMeasure` | `calibrated_range`; `perform_rom_test`, `perform_recentre` | P30 |
| `gallery` | `Gallery` | `card_types`, `list_data`, `bulk_actions`; `delete_all_data` | P31 |
| `smart_scan` | `SmartScan` | `workflow_name`, `workflow_display_names`, `latest_scan_live_details`, `stitch_automatically`; `sample_scan`, `stitch_scan`, `stitch_all_scans`, `download_zip`, `purge_empty_scans` | P35, P39 |
| `snake_workflow`, `raster_workflow`, `histo_scan_workflow`, `c_chip_workflow` | workflow Things (`ScanWorkflow`) | overlap, autofocus range, counts, stack settings, `ready`, `settings_ui` | P35, P37 |
| `sequence` | `Sequence` | `dt`, `duration`, `autofocus_enabled`, `autofocus_dz`; `run_sequence`, `download_zip` | P41 |

The global lock is enabled. Scans, calibrations and moves exclude one another, while `jog`, downloads and stitching opt out, as in OpenFlexure.

### 5.4 Serving and lifecycle

`teta-wot`'s `ThingServer::serve` serves only its own router, and that router dispatches everything through a single fallback handler. To add its own routes without changing `teta-wot` (D6), the server:

1. builds a `ThingServer` from the config and the registry;
2. starts the Things (`runtime().start()`);
3. builds an **outer axum router** that answers the app routes below and hands every other request to `teta-wot`'s router as its fallback service;
4. serves it with `axum::serve` and graceful shutdown;
5. on shutdown, repeats `teta-wot`'s own sequence: close the message broker (ending SSE and WebSocket streams), cancel invocations, wait for them within the grace period, then stop the Things in reverse order.

If startup fails and `--fallback` is set, it serves `teta-wot`'s public `fallback_router(FallbackPage)` instead. ADR-0006 records this, along with the cost: our serve loop has to track `teta-wot`'s behaviour whenever we upgrade.

| Path | Answered by | Purpose | Phase |
|---|---|---|---|
| `/`, `/index.html`, `/assets/*`, favicon | outer router | the embedded web app | P12 |
| `/api/v1/health` | outer router | readiness, version, uptime | P02 |
| `/api/v1/log/`, `/api/v1/logfile/` | outer router | recent log records as JSON; the log file | P03 |
| `/api/v1/data/{*path}` | outer router | gallery files: images, thumbnails, DZI tiles | P31 |
| `/api/v1/{thing}/…`, `/api/v1/thing_descriptions/`, `/api/v1/action_invocations/…`, OpenAPI and docs pages | `teta-wot` | the Web of Things API | P02 onward |
| `/api/v1/{thing}/{path}` from `#[endpoint]` | `teta-wot` | Thing-specific routes such as `smart_scan/latest_preview_stitch.jpg` and `*/settings_ui` | P24 onward |
| `/.well-known/wot` | `teta-wot` | WoT discovery | (free) |

### 5.5 Launcher and sidecar contract

The **sidecar contract** (ADR-0032, P44) is what any supervisor relies on, whether it's the milestone-1 launcher or Tauri later:

- `microscope-server --managed` accepts `--port 0`. Once it is listening, it prints one line, `MICROSCOPE_READY {"url": …, "pid": …, "version": …}`, to stdout.
- When stdin closes, the server shuts down gracefully. This avoids Windows' awkward console-signal semantics for child processes.
- Exit codes: `0` stopped, `75` restart requested, `1`/`2`/`3` errors (the same codes as `teta-wot`'s CLI).
- The `system` Thing exposes `managed`, `shutdown` and `restart`. The Power tab uses them.

**The launcher** (ADR-0033, P45) resolves the config and the data folders (`%LOCALAPPDATA%\Microscope\`), holds a single-instance lock, and starts the server, preferring port 5000. It waits for the ready line, then opens the browser. On exit code 75 it restarts at once; after a crash it restarts with backoff, up to a crash-loop limit. On Ctrl-C it closes the server's stdin and waits. A Windows Job Object makes sure the server never outlives the launcher.

### 5.6 Connection model

ADR-0015 (P11) defines two kinds of connection profile:

- **This server** is the origin that served the page. When the launcher started that server, `system.managed` is true and the app shows "managed" status and offers Restart.
- **Remote** uses a base URL that the user enters, such as `http://lab-pc:5000`. Requests are cross-origin; `teta-wot` allows CORS from any origin.

The connection store runs a small state machine (connecting → connected → lost → reconnecting) and caches the Thing Descriptions. It also gates tabs on whether the Things they need are present. Later, Tauri adds a third profile, a sidecar it manages itself, behind the same interface.

### 5.7 Simulator

ADR-0017 (P13) covers this. `microscope-sim` models the world in physical units:

- **Stage model** (P14): the true position against the reported position, a zero offset, timed moves at a set speed, interruptible jogs, backlash per axis, and travel limits.
- **Specimens**: `Specimen::render(region_µm, µm_per_px)` is deterministic for a given seed and supports levels of detail for low magnifications. Blobs come in P13; procedural tissue (H&E-like) and image-backed samples come in P25.
- **Optics**: objectives from 4× to 100× set the field of view and depth of field. Blur grows with defocus relative to a focal surface, which is a tilt plane plus low-frequency waviness (P25). The camera can be rotated or flipped relative to the stage (P23), which makes camera–stage mapping non-trivial. Vignetting, uneven illumination, exposure and gain, noise, and LED on/off complete the model (P25).
- **Budget**: a 820×616 preview frame within about 40 ms (release build), giving a preview of 10 fps or more. P13 and P17 measure this.

Every imperfection is a setting that can be switched at runtime, so tests and demos can turn them on and off.

### 5.8 Hardware abstraction and units

ADR-0018 and ADR-0019 (P15) cover this.

- **Interfaces**: what one Thing needs from another is a `#[teta_wot::interface]` trait: `StageApi`, `CameraApi`, `IlluminationApi` and `ScanWorkflow`. Slots name the interface, not the type, so a config file can swap the simulator for a real driver.
- **One Thing type per driver.** Shared behaviour (capture buffers, streams, backlash compensation, jog queue) lives in plain structs that every driver's Thing composes. In milestone 2, vendor SDKs that need a fixed thread run behind `teta-wot` `Device<D>` actors.
- **Units**: positions are integer **steps** per axis, which keeps OpenFlexure parity and lets camera–stage mapping work in pixels to steps. Each stage declares `um_per_step` per axis. A Sangaboard step is a motor step; a µm-native stage defines one step as its native resolution. The UI and image metadata show µm whenever a scale is known.
- **Walk-throughs.** The P15 notes check the interfaces against the three milestone-2 targets: Sangaboard plus Pi camera, an industrial camera SDK, and a µm-native stage.

### 5.9 Data on disk

ADR-0025 (P26) and ADR-0026 (P31) cover this.

```text
<root>                      dev: .\.microscope\    launcher: %LOCALAPPDATA%\Microscope\
├─ settings\<thing>\Settings-<Type>.json     teta-wot settings files
├─ data\
│  ├─ captures\<name>.jpeg + <name>.json       image + metadata sidecar
│  ├─ scans\<scan>\images\…, scan.json, scan.log, stitched\ (DZI, overview)
│  ├─ sequences\<name>\image_00000.jpeg …, sequence.json
│  └─ calibration\csm-<timestamp>.json, …
└─ logs\microscope.<date>.log
```

The file system is the source of truth. At start-up the gallery builds an in-memory index from the metadata sidecars, then updates it on every write. Unlike OpenFlexure there is no SQLite; ADR-0026 explains the trade-off.

### 5.10 Web app

```text
web/src/
├─ app/            App.vue, router (hash), shell, navigation rail, shortcuts
├─ theme/          tokens.css, light/dark themes, theme store
├─ ui/             design-system components (Button, Select, Dialog, Accordion, …)
├─ host/           HostAdapter + browser implementation
├─ api/
│  ├─ wot/         TD model, forms, properties, invocations, SSE reader, errors
│  ├─ streams/     fetch-based MJPEG reader
│  ├─ generated/   TypeScript types generated from the server's OpenAPI document
│  └─ things/      typed facades (stage, camera, autofocus, …)
├─ connection/     profiles, connection store, Connect screen
├─ server-ui/      renderers for server-described UI elements
├─ features/       live-image, control, capture, autofocus, settings, calibration-wizard,
│                  gallery, viewer, slide-scan, sequence, logging, about, power
└─ views/          one route component per tab
```

- **State**: Pinia stores hold preferences (theme, navigation steps and inversion, stream disabled), the connection, and feature state. Preferences persist to `localStorage`, with every read and write guarded.
- **Live data**: properties are observed over SSE through `fetch`. Invocations are polled with backoff, as OpenFlexure does; this is simple and robust. MJPEG is read with `fetch`, parsed, and drawn to a canvas.
- **Server-described UI** (ADR-0024, P24): settings panels whose contents depend on the hardware (camera settings, workflow settings, background detectors, calibration actions) are described by the Things and drawn by generic renderers. That keeps simulator-only settings out of the front end and lets milestone-2 drivers bring their own. The core views are written by hand.
- **Theming** (ADR-0011, P06): CSS custom-property tokens with a magenta accent, light neutral surfaces and dark charcoal surfaces, compact controls, and a system font stack (Segoe UI on Windows). The theme is set as `data-theme` on `<html>` by a small script that runs before the app mounts, so it never flashes the wrong theme.

### 5.11 Testing and CI

| Layer | Tools | Where |
|---|---|---|
| Algorithms and simulator | Rust unit tests, `proptest`, determinism checks with fixed seeds | `microscope-core`, `microscope-sim` |
| Things | `teta_wot::testing::{Harness, TestClient}` | `microscope-things` |
| Server binary | tower `oneshot` on the composed router; integration tests that spawn the binary | `microscope-server` |
| Web units and components | Vitest, Vue Test Utils, happy-dom | `web/tests/unit` |
| End to end | Playwright against the real server with a deterministic simulation config | `web/tests/e2e` |
| Visual regression | Playwright screenshots in both themes; Windows baselines | `web/tests/visual` (P07, P47) |
| Accessibility | axe-core in Playwright; keyboard walkthroughs | P47 |

CI runs on `windows-latest`, with jobs `rust` (fmt, clippy, test, doc), `web` (lint, typecheck, unit, build) and `e2e` (both builds, then Playwright). A nightly workflow runs the soak tests (P48).

## 6. Feature parity with OpenFlexure

| Area | OpenFlexure feature | Milestone 1 | Phase |
|---|---|---|---|
| Shell | Vertical tab rail, light/dark/system theme, Shift+↑/↓ tab switching, narrow-window warning | Yes | P06, P07 |
| View | Full-window live stream, option to disable the stream, flash on capture | Yes | P18, P26 |
| Control | Position fields, Move, Set Home, Move Home with confirmation | Yes | P20 |
| Control | D-pad and focus jog, keyboard jog, scroll to focus | Yes | P20 |
| Control | Double-click to move (camera–stage mapping) | Yes | P23 |
| Control | Autofocus (`a`), capture to gallery or download (`c`), `?` shortcut help | Yes | P21, P26, P09b |
| Slide Scan | Workflow choice with blurb and workflow-specific settings | Yes | P40 |
| Slide Scan | Histo, Snake, Raster and C-Chip workflows | Yes | P35, P37 |
| Slide Scan | Background-detection settings, set and check background | Yes | P27, P37 |
| Slide Scan | Smart z-stacks | Yes | P36 |
| Slide Scan | Live stitching preview, scan log, phase-aware cancel, scan settings dialog, image count, duration | Yes | P39, P40 |
| Slide Scan | Download ZIP, stitch automatically | Yes | P35, P39 |
| Slide Scan | Pyramidal TIFF | Deferred | — |
| Sequence | Timelapse: duration, interval, autofocus before each capture | Yes, with an MDA-ready model | P41, P42 |
| Gallery | Detailed and thumbnail cards, pagination, type filter, bulk actions, Delete All, per-card downloads, actions and messages | Yes | P31, P32 |
| Gallery | Deep-zoom viewer with brightness, contrast and saturation, fullscreen, image-sequence slider | Yes | P33 |
| Settings | Display (theme, fullscreen, disable stream), stage-control preferences | Yes | P20, P28 |
| Settings | Camera calibration actions and manual settings with a mini stream | Yes | P27, P28 |
| Settings | Stage: z inversion, recentre, range-of-motion test | Yes | P16, P30 |
| Settings | Camera–stage mapping: calibrate, details, download data | Yes | P23, P28 |
| Calibration | First-run wizard, relaunchable from Settings | Yes | P29 |
| Logging | Grouped log list, level filter, pagination, download | Yes | P03, P43 |
| About | Hostname, API origin, version, hardware types, flash illumination, intended use, links | Yes (our own text) | P46 |
| Power | Shut down; restart | Yes (restart through the launcher) | P44–P46 |
| Simulation | Simulated camera: objective, density, colour, noise, infinite sample, load/remove sample | Yes, plus tissue and image-backed samples | P13, P17, P25 |
| Simulation | Dummy stage | Yes, plus backlash and travel limits | P14, P16 |
| Server | Config file with Things, settings folder, `application_config`, `--fallback` | Yes | P02 |
| Server | Server-defined extension tabs (`tab_description`) | No | — |
| Server | Legacy v2 API | No | — |
| Connectivity | Discovery on the network (OpenFlexure Connect) | No (remote connections by URL instead) | — |

## 7. How phases work

**Size.** Each phase has a budget in changed lines of hand-written code, not counting lock files, generated types, fixtures or docs: **S** ≤ 300, **M** 300–600, **L** 600–900. A phase that grows past about 900 lines is split before review, and the plan is updated.

**Branch and pull request.** Each phase gets a branch `m1/pNN-short-name` and a pull request titled `PNN: <title>`.

**Approval and merge.** Phases run strictly one at a time:

1. The pull request is opened with CI green and its status set to *In review*.
2. The project owner reviews it. Any requested changes go into the same pull request, and the phase stays *In review* until they're done.
3. The pull request is merged **only after the project owner approves it**. The phase is then *Done*.
4. Work on the next phase starts **only once the previous pull request is merged**. No phase branches off an unmerged one.

**What every pull request contains**

- the code and its tests;
- `docs/milestone-1/notes/PNN-short-name.md`, the implementation notes, from the template in [Appendix B](#appendix-b-implementation-notes-template);
- new ADRs with the status *Accepted* (approving the pull request accepts them), and their rows in `docs/adr/README.md`;
- the phase marked *In review* in [§8](#8-phase-overview). The next phase's pull request marks it *Done*, since a merged pull request can't be edited.

**Definition of Done** (this becomes the PR template in P00)

- [ ] The scope matches the phase spec in [phases.md](phases.md); any deviation is explained in the notes.
- [ ] CI is green.
- [ ] Tests cover the new behaviour.
- [ ] Notes and ADRs are written, and the indexes are updated.
- [ ] The notes' "How to try it" steps work from a clean checkout.
- [ ] Independent implementation: nothing is copied from OpenFlexure's code, styles, assets or text.
- [ ] The phase is within its size budget.

**Spec-first phases.** P21, P23, P27, P30, P34, P36, P37 and P41 open their notes with a behaviour spec, written before the code ([§4.1](#41-independent-implementation)).

**Checkpoints.** At the end of each stage with a checkpoint, the notes include a short demo script. That's a good moment to look at the whole app, not just the diff.

## 8. Phase overview

Sizes follow [§7](#7-how-phases-work). The detailed specs are in [phases.md](phases.md). Status runs Planned → In progress → In review → Done.

### Stage 0: Groundwork

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P00 | Docs scaffolding and foundational ADRs | — | S | 0001–0004 | Done |
| P01 | Rust workspace and CI | P00 | S | 0005 | Done |
| P02 | Server lifecycle and composed router | P01 | M | 0006, 0007 | Done |
| P03 | Server logging and log endpoints | P02 | M | 0008 | Done |

### Stage 1: Web foundations

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P04 | Web app scaffold and tooling | P01 | M | 0009, 0010 | Done |
| P05 | OpenFlexure reference screenshots | P04 | S | — | Done |
| P06 | Design tokens and theme switching | P04, P05 | M | 0011 | Done |
| P07 | App shell and navigation rail | P06 | M | 0012 | Done |
| P08 | UI components I: controls | P06 | L | 0013 | Done |
| P09 | UI components II: overlays and feedback | P08 | L | — | Done |
| P09b | UI components III: menus, lists and shortcuts | P09 | L | — | In review |

### Stage 2: Client–server plumbing

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P10 | WoT client library | P02, P04 | L | 0014 | Planned |
| P11 | Connection management | P07, P09, P10 | M | 0015 | Planned |
| P12 | Serve the web app from the backend | P02, P04 | M | 0016 | Planned |

### Stage 3: Simulator and live microscope (ends with Checkpoint A)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P13 | Simulation world, optics and blob specimen | P01 | L | 0017 | Planned |
| P14 | Simulated stage motion model | P13 | M | — | Planned |
| P15 | Hardware interfaces and units | P02, P13 | M | 0018, 0019 | Planned |
| P16 | Simulated stage Thing | P14, P15 | M | — | Planned |
| P17 | Simulated camera and illumination Things | P13, P16 | L | 0020 | Planned |
| P18 | View tab: live image | P11, P17 | M | — | Planned |
| P19 | Generated API types and typed facades | P10, P17 | S | 0021 | Planned |
| P20 | Control tab: stage navigation | P09b, P18, P19 | L | — | Planned |

**Checkpoint A, "Live microscope":** the app shows the live simulated image, and you can jog and focus with the buttons, the keys and the wheel, or move to typed coordinates.

### Stage 4: Focus, calibration and capture (ends with Checkpoint B)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P21 | Autofocus | P17, P20 | M | 0022 | Planned |
| P22 | Image registration core | P13 | M | 0023 | Planned |
| P23 | Camera–stage mapping and click-to-move | P21, P22 | L | — | Planned |
| P24 | Server-described UI elements | P09b, P10 | L | 0024 | Planned |
| P25 | Specimens II and optical effects | P17, P24 | L | — | Planned |
| P26 | Capture to the data folder | P17, P20 | M | 0025 | Planned |
| P27 | Background detection and camera calibration | P24, P25 | L | — | Planned |
| P28 | Settings tab | P23, P24, P27 | L | — | Planned |
| P29 | Calibration wizard | P28 | L | — | Planned |
| P30 | Stage measurement tools | P22, P23, P28 | M | — | Planned |

**Checkpoint B, "Calibrated microscope":** on a fresh settings folder the wizard runs. After it, autofocus, click-to-move and capture work, and every Settings panel is live.

### Stage 5: Gallery (ends with Checkpoint C)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P31 | Gallery backend and data route | P26 | L | 0026 | Planned |
| P32 | Gallery tab | P09b, P31 | L | — | Planned |
| P33 | Image and deep-zoom viewer | P32 | M | 0027 | Planned |

**Checkpoint C, "Gallery":** captures appear as cards that you can filter, page through, download, delete and open in the viewer.

### Stage 6: Slide scanning (ends with Checkpoint D)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P34 | Scan planners | P13 | M | 0028 | Planned |
| P35 | Scan engine with snake and raster workflows | P21, P23, P31, P34 | L | 0029 | Planned |
| P36 | Smart z-stacks | P35 | M | — | Planned |
| P37 | Histo smart-spiral and C-Chip workflows | P27, P36 | L | — | Planned |
| P38 | Mosaic and deep-zoom pyramid core | P13 | L | 0030 | Planned |
| P39 | Live and final stitching | P35, P38 | M | — | Planned |
| P40 | Slide Scan tab | P24, P33, P37, P39 | L | — | Planned |

**Checkpoint D, "Slide scanning":** all four workflows run on the simulator, show a live stitching preview, and end as a stitched deep-zoom image in the gallery.

### Stage 7: Sequence (ends with Checkpoint E)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P41 | Sequence engine (MDA-ready) | P21, P26, P31 | L | 0031 | Planned |
| P42 | Sequence tab | P24, P41 | M | — | Planned |

**Checkpoint E, "Sequence":** a timelapse with autofocus runs, can be cancelled, and appears in the gallery with an image-sequence viewer.

### Stage 8: System, launcher and delivery (ends with milestone 1)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P43 | Logging tab | P03, P09b, P11 | M | — | Planned |
| P44 | Sidecar process contract | P02 | M | 0032 | Planned |
| P45 | Local launcher | P12, P44 | L | 0033 | Planned |
| P46 | About and Power tabs | P11, P44 | M | — | Planned |
| P47 | Visual fidelity and accessibility pass | P05, all tabs | L | 0034 | Planned |
| P48 | End-to-end, soak and performance | all | L | — | Planned |
| P49 | Packaging and milestone close-out | P45, P48 | M | 0035 | Planned |

**Order.** The table order is the planned sequence, and phases run one at a time ([§7](#7-how-phases-work)). A few phases depend only on early ones, so they could be moved earlier if a reorder ever helps: P13, P14, P22, P34 and P38 (pure Rust), and P44. Any reorder is agreed first and recorded in [Appendix D](#appendix-d-change-log).

<a id="adr-plan"></a>
### Planned ADRs

Numbers are provisional. They are assigned in the order the ADRs are written.

| ADR | Title | Phase |
|---|---|---|
| 0001 | Record architecture decisions | P00 |
| 0002 | Independent implementation of OpenFlexure-inspired behaviour | P00 |
| 0003 | Milestone-1 platform: Windows x86_64 | P00 |
| 0004 | Depend on `teta-wot` v0.1.0 without upstream changes | P00 |
| 0005 | Repository layout and crate boundaries | P01 |
| 0006 | Own the server lifecycle around `teta-wot`'s runtime | P02 |
| 0007 | API prefix `/api/v1` and OpenFlexure-mirrored names | P02 |
| 0008 | Server logging and log endpoints | P03 |
| 0009 | Frontend toolchain | P04 |
| 0010 | Rules for a Tauri-ready web app | P04 |
| 0011 | Design tokens and theming | P06 |
| 0012 | Icon set | P07 |
| 0013 | Own components on Reka UI primitives | P08 |
| 0014 | Client transports: TD forms, fetch streams, polled invocations | P10 |
| 0015 | Connection model | P11 |
| 0016 | Serving and embedding the web app | P12 |
| 0017 | Simulator architecture | P13 |
| 0018 | Hardware abstraction through `teta-wot` interfaces | P15 |
| 0019 | Stage units: integer steps with a µm scale | P15 |
| 0020 | Camera frame pipeline | P17 |
| 0021 | Generated API types | P19 |
| 0022 | Sharpness metric and fast autofocus | P21 |
| 0023 | Image registration by phase correlation | P22 |
| 0024 | Server-described UI elements | P24 |
| 0025 | Data folder layout and capture metadata | P26 |
| 0026 | Gallery index and data route | P31 |
| 0027 | Deep-zoom viewer | P33 |
| 0028 | Scan planning model | P34 |
| 0029 | Scan engine and workflow Things | P35 |
| 0030 | Position-based stitching with DZI output | P38 |
| 0031 | Sequence model | P41 |
| 0032 | Sidecar process contract | P44 |
| 0033 | Local launcher and its Tauri successor | P45 |
| 0034 | Visual regression baselines | P47 |
| 0035 | Release packaging and third-party notices | P49 |

## 9. Documentation conventions

```text
docs/
├─ README.md                         index of all documentation
├─ adr/
│  ├─ README.md                      ADR index (number, title, status, phase) and process
│  ├─ template.md
│  └─ NNNN-short-title.md
├─ milestone-1/
│  ├─ goals.md                       the milestone goals, in the project owner's words
│  ├─ implementation-plan.md         this document
│  ├─ phases.md                      phase specifications
│  └─ notes/
│     ├─ README.md                   index of implementation notes
│     ├─ template.md
│     └─ PNN-short-name.md           one per phase
└─ reference/
   ├─ README.md                      how to capture OpenFlexure references (P05)
   └─ ofm/, microscope/              screenshots (git-ignored)
```

- **ADRs** are numbered across the whole project, never reused, and never edited after acceptance except for their status. A later ADR supersedes an earlier one ("Superseded by ADR-00NN").
- **Implementation notes** are written for a reviewer and for whoever works on the code next. They cover what was built, how to try it, design notes, deviations from the plan, tests, follow-ups and the independent-implementation check.
- **This plan** is a living document. When a phase is split, merged or reordered, the change goes in the same pull request with a line in [Appendix D](#appendix-d-change-log).

## 10. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| The scope is large: full parity plus a richer simulator | The milestone takes many phases | Checkpoints give usable increments. If time runs short, the cut line is C-Chip, the stage measurement tools and image-backed samples, which can move to a milestone 1.5 without affecting the others |
| Owning the serve loop duplicates `teta-wot` internals | It can drift when `teta-wot` is upgraded | Keep it in one small module with tests of the shutdown behaviour; pin the tag; ADR-0006 names the trigger to revisit (a router hook upstream) |
| `teta-wot` is new (v0.1.0) | Bugs or gaps under real load (MJPEG, SSE, invocations) | Checkpoint A exercises the transports early; work around in the app and report upstream |
| Simulator performance on Windows | A sluggish preview, slow autofocus and scans | Budgets set in P13 and P17; level of detail, caching and a fast blur; a lower-resolution preview stream |
| Decoding MJPEG with `fetch` in the browser | CPU use and dropped frames | `createImageBitmap`, a frame cap, stream sharing; measured in P18 |
| Independent-implementation discipline | Licence contamination from the GPL reference | ADR-0002, spec-first notes, the PR checkbox, references kept out of git |
| Visual fidelity is subjective | Rework late in the milestone | References from P05, token review in P06, side-by-side captures from P07, a dedicated pass in P47 |
| Memory use in large mosaics | Out-of-memory on big scans | The DZI writer works tile by tile and never holds the full mosaic (P38) |
| Screenshot tests are platform-dependent | Flaky visual tests | Windows-only baselines generated in CI, with documented tolerances (ADR-0034) |
| Process control on Windows | Orphaned servers, unclean stops | The stdin-EOF stop protocol, Job Objects, integration tests (P44, P45) |
| `teta-wot`'s guide is out of date in places (it says there is no licence and uses old `wot` crate names) | Confusion when depending on it | Follow the code and `Cargo.toml` (MIT, crate `teta-wot`, tag `v0.1.0`); note it in ADR-0004 |

## 11. Points for your review

These are choices I made while writing the plan. Each is easy to change now and harder later.

1. **Granularity**: 50 phases, each within a 900-line budget. Is that the review size you want?
2. **The Thing is named `sequence`, not OpenFlexure's `timelapse`**, to match the tab and the broader model. Its settings (`dt`, `duration`, `autofocus_enabled`, `autofocus_dz`) keep OpenFlexure's names.
3. **API prefix `/api/v1`** rather than OpenFlexure's `/api/v3`, since compatibility isn't promised.
4. **Hash routing** in the web app, for Tauri and for the fallback-router composition.
5. **Server-described UI** for hardware-dependent settings panels (mirroring OpenFlexure's idea, with our own model); everything else is written by hand.
6. **The global lock is enabled**, with `jog`, downloads and stitching opting out.
7. **Gallery storage** is the file system plus JSON sidecars and an in-memory index, with no SQLite.
8. **Generated TypeScript types** from the server's OpenAPI document, checked for drift in CI.
9. **Data locations**: `.\.microscope\` for development runs and `%LOCALAPPDATA%\Microscope\` under the launcher.
10. **The launcher comes late (P45).** Until then, development uses `cargo run` and the Vite dev server. The connection layer is still designed for it from P11.
11. **Tooling**: npm with Node 24 LTS, and Rust pinned to 1.98.1 to match `teta-wot`.
12. **`goals.md`** moves to `docs/milestone-1/goals.md` in P00 (or stays at the root, if you prefer).
13. **The root README** is rewritten in P00 with the project description and links to these docs.

## 12. Appendices

### Appendix A: ADR template

```markdown
# ADR-NNNN: <Title>

- Status: Proposed | Accepted | Superseded by ADR-NNNN
- Date: YYYY-MM-DD
- Phase: PNN

## Context
What forces are at play, and what problem needs deciding.

## Decision
What we will do, stated plainly.

## Consequences
What becomes easier, what becomes harder, what we must now watch.

## Alternatives considered
Each option with the reason it lost.
```

### Appendix B: implementation notes template

```markdown
# PNN: <Title>

- Status: In review | Done
- Pull request: #NN
- ADRs: ADR-NNNN, …
- Spec: [phases.md#pnn](../phases.md#pnn)

## Summary
Two or three sentences: what this phase delivers.

## Behaviour spec (spec-first phases only)
Written before the code, in our own words.

## What was built
Modules, Things and affordances, components, routes.

## How to try it
Commands and clicks that work from a clean checkout.

## Design notes and deviations from the plan

## Tests

## Follow-ups and known gaps

## Independent implementation
- [ ] No code, styles, assets or text copied from OpenFlexure.
```

### Appendix C: references studied

- `teta-wot` (v0.1.0): `docs/guide/src/*` (structure, actions, properties, events, slots, settings, blobs and MJPEG, arrays, concurrency, testing, configuration, wire profiles, security, discovery, Windows service). Code checked: `crates/teta-wot-server/src/lib.rs` (`ThingServer::serve_with`, `router`, `runtime`), `crates/teta-wot-http/src/lib.rs` (fallback-dispatch router, `HttpOptions`, CORS), `crates/teta-wot-http/src/fallback.rs`, `crates/teta-wot-core/src/{runtime,logs,logging}.rs`, and the macros (no generic Things or interfaces). Examples: `simulated-microscope`, `microscope-service`.
- OpenFlexure Microscope Server (v3 branch, `v3.0.0-beta1-150`, GPL-3.0), studied for behaviour only: the developer docs (`architecture`, `simulation_guide`, `webapp`, `config-file`), the UI screenshots in `apidocs/dev/assets`, the web app's tab structure and workflows (`webapp/src/components/**`, `stores/*`), and the Things' affordances (`src/openflexure_microscope_server/things/**`).

### Appendix D: change log

| Date | Change |
|---|---|
| 2026-10-08 | First draft for review |
| 2026-10-08 | Delivery: each pull request is merged only after the project owner approves it; the next phase starts only after the merge |
| 2026-10-08 | P00: `goals.md` moved to `docs/milestone-1/goals.md`; approving a pull request accepts its ADRs; the next phase's pull request marks the previous phase *Done* |
| 2026-10-09 | P09 split in two, at the project owner's choice, since it was estimated at three times its M budget. P09 keeps dialogs, confirmations, toasts, tooltips and error details. The new P09b has menus, pagination, multiple selection and keyboard shortcuts. Both are sized L. P20, P24, P32 and P43 now depend on P09b |
