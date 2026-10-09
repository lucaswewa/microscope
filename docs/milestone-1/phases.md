# Milestone 1 phase specifications

Companion to the [implementation plan](implementation-plan.md). Each phase lists its goal, dependencies, size and ADRs, then its scope, what it leaves out, when it's done, and how it's tested. Sizes and the Definition of Done are defined in [§7 of the plan](implementation-plan.md#7-how-phases-work). Every phase also writes its implementation notes in `docs/milestone-1/notes/`.

"Mirrors OpenFlexure" means the names and behaviour follow OpenFlexure v3, implemented independently ([§4.1](implementation-plan.md#41-independent-implementation)). **Spec-first** phases open their notes with a behaviour spec written before the code.

## Contents

- [Stage 0: Groundwork](#stage-0-groundwork): [P00](#p00) [P01](#p01) [P02](#p02) [P03](#p03)
- [Stage 1: Web foundations](#stage-1-web-foundations): [P04](#p04) [P05](#p05) [P06](#p06) [P07](#p07) [P08](#p08) [P09](#p09) [P09b](#p09b)
- [Stage 2: Client–server plumbing](#stage-2-clientserver-plumbing): [P10](#p10) [P11](#p11) [P12](#p12)
- [Stage 3: Simulator and live microscope](#stage-3-simulator-and-live-microscope): [P13](#p13) [P14](#p14) [P15](#p15) [P16](#p16) [P17](#p17) [P18](#p18) [P19](#p19) [P20](#p20)
- [Stage 4: Focus, calibration and capture](#stage-4-focus-calibration-and-capture): [P21](#p21) [P22](#p22) [P23](#p23) [P24](#p24) [P25](#p25) [P26](#p26) [P27](#p27) [P28](#p28) [P29](#p29) [P30](#p30)
- [Stage 5: Gallery](#stage-5-gallery): [P31](#p31) [P32](#p32) [P33](#p33)
- [Stage 6: Slide scanning](#stage-6-slide-scanning): [P34](#p34) [P35](#p35) [P36](#p36) [P37](#p37) [P38](#p38) [P39](#p39) [P40](#p40)
- [Stage 7: Sequence](#stage-7-sequence): [P41](#p41) [P42](#p42)
- [Stage 8: System, launcher and delivery](#stage-8-system-launcher-and-delivery): [P43](#p43) [P44](#p44) [P45](#p45) [P46](#p46) [P47](#p47) [P48](#p48) [P49](#p49)

---

## Stage 0: Groundwork

<a id="p00"></a>
### P00: Docs scaffolding and foundational ADRs

**Goal.** Set up the documentation system that every later phase uses, and record the decisions that frame the milestone.
**Depends on** — · **Size** S (docs and config only) · **ADRs** 0001, 0002, 0003, 0004

**Scope**
- Extend `docs/README.md` (the index, created with this plan), add `docs/adr/README.md` (the index table and the process), `docs/adr/template.md`, `docs/milestone-1/notes/README.md` and `template.md` (templates from the plan's appendices).
- `.github/pull_request_template.md` with the Definition-of-Done checklist.
- Rewrite the root `README.md`: what the project is, its status, how it relates to `teta-wot` and OpenFlexure, and links to the docs.
- Move `goals.md` to `docs/milestone-1/goals.md`, or keep it at the root (plan §11, point 12).
- ADR-0001 Record architecture decisions. ADR-0002 Independent implementation of OpenFlexure-inspired behaviour. ADR-0003 Milestone-1 platform: Windows x86_64. ADR-0004 Depend on `teta-wot` v0.1.0 without upstream changes (this one also notes where `teta-wot`'s guide is out of date).

**Done when**
- The docs render on GitHub, the ADR index lists 0001–0004 as Accepted, and new pull requests show the template.

<a id="p01"></a>
### P01: Rust workspace and CI

**Goal.** A buildable, empty Cargo workspace with the crate boundaries the plan uses, and CI that enforces formatting, lints, tests and docs on Windows.
**Depends on** P00 · **Size** S · **ADRs** 0005

**Scope**
- Root `Cargo.toml`: resolver 3, edition 2024, `rust-version = "1.98"`. Workspace lints `missing_docs = "warn"` and `unsafe_code = "forbid"` (the launcher may need a documented exception in P45), plus clippy `all = "warn"`.
- `rust-toolchain.toml` pinned to 1.98.1 with rustfmt and clippy, matching `teta-wot`.
- Crate skeletons with crate-level docs: `microscope-core`, `microscope-sim` and `microscope-things` (libraries), and `microscope-server` and `microscope-launcher` (binaries that print their version).
- `teta-wot` as a workspace dependency: `{ git = "https://github.com/lucaswewa/teta-wot", tag = "v0.1.0" }`.
- `.github/workflows/ci.yml` on `windows-latest`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`.
- `.gitignore`: `node_modules/`, `web/dist/`, `.microscope/`, `docs/reference/ofm/`, `docs/reference/microscope/`.

**Done when**
- CI is green on the pull request, and `cargo run -p microscope-server -- --version` prints the version.

<a id="p02"></a>
### P02: Server lifecycle and composed router

**Goal.** `microscope-server` serves Things from a config file under a lifecycle the app owns, so the app can add its own routes without changing `teta-wot`.
**Depends on** P01 · **Size** M · **ADRs** 0006, 0007

**Scope**
- A CLI with LabThings- and `teta-wot`-compatible flags: `-c/--config`, `-j/--json`, `--host` (default 127.0.0.1), `--port` (default 5000), `--debug` and `--fallback`. Exit codes match: 0 stopped, 1 general error, 2 invalid command line, 3 invalid config or a Thing failed to start.
- Config: `teta-wot`'s `ServerConfig` with a typed `application_config` (`data_folder`, `log_folder`), `api_prefix` set to `/api/v1`, and `enable_global_lock: true`. `configs/simulation.json` lists only `system` for now.
- A registry with the `system` Thing (`MicroscopeSystem`): `hostname`, `version_data` (crate version plus the build's `git describe`) and `os_version`.
- A lifecycle module that builds the `ThingServer`, starts the runtime, composes the outer router (app routes, then `teta-wot`'s router as the fallback service), and serves with graceful shutdown. Shutdown repeats `ThingServer::serve_with`'s steps: close the broker, cancel invocations, wait for idle within the grace period, then stop the runtime.
- `GET /api/v1/health` returns status, version and uptime.
- `--fallback` serves `teta-wot`'s `fallback_router(FallbackPage)` when startup fails.

**Out of scope**: logging beyond the console (P03), the SPA (P12), managed mode (P44).

**Done when**
- `cargo run -p microscope-server -- -c configs/simulation.json` serves `/api/v1/system/`, the API docs and `/api/v1/health`.
- Ctrl-C during a long, test-only action cancels it and exits 0 within the grace period.

**Tests**
- The composed router, through tower `oneshot`: app routes win, everything else reaches `teta-wot`, and 404s come from `teta-wot`.
- The shutdown sequence, with a test Thing.
- CLI exit codes, by spawning the binary.

<a id="p03"></a>
### P03: Server logging and log endpoints

**Goal.** Server-wide logs that the Logging tab and the action dialogs can show, and a log file that can be downloaded.
**Depends on** P02 · **Size** M · **ADRs** 0008

**Scope**
- A `tracing` setup with four outputs: the console (filtered by `RUST_LOG`), `teta-wot`'s invocation-log layer, an in-memory ring buffer of the last 1000 records, and a daily rolling file in `log_folder`.
- Records shaped for the UI and aligned with OpenFlexure's where practical: `created`, `levelname`, `levelno`, `name` (the target), `message`, `invocation_id`/`thing`/`action` when inside an invocation, and `exception`.
- `GET /api/v1/log/` returns JSON records, oldest first, with an optional minimum `?level=`. `GET /api/v1/logfile/` downloads the current file as text.

**Done when**
- An action's log lines appear in both its invocation log and `/api/v1/log/`, and `/api/v1/logfile/` downloads.

**Tests**
- Ring-buffer capacity and ordering, the level filter, the record shape (snapshot), and the route answers.

---

## Stage 1: Web foundations

<a id="p04"></a>
### P04: Web app scaffold and tooling

**Goal.** A typed Vue app skeleton with linting, tests and CI for later phases to fill in.
**Depends on** P01 · **Size** M · **ADRs** 0009, 0010

**Scope**
- `web/`: Vite, Vue 3, TypeScript (strict), Pinia and Vue Router (hash history).
- npm with Node 24 LTS (`.nvmrc`, `engines`) and a committed `package-lock.json`.
- ESLint (flat config, typescript-eslint, eslint-plugin-vue), Prettier, `vue-tsc --noEmit`, Vitest (happy-dom, @vue/test-utils), and Playwright (Chromium) with one smoke test.
- `src/host/`: the `HostAdapter` interface (open a link, save a file, app info, service control) with a browser implementation.
- A Vite dev-server proxy from `/api` to `http://127.0.0.1:5000`.
- A `web` CI job on Windows: `npm ci`, lint, typecheck, unit tests and build.
- ADR-0010 lists the Tauri-readiness rules from the plan's §4.2.

**Done when**
- `npm run dev` shows a placeholder app and the `web` CI job is green.

<a id="p05"></a>
### P05: OpenFlexure reference screenshots

**Goal.** A repeatable way to capture OpenFlexure's UI in both themes for visual comparison, without committing any of the captures.
**Depends on** P04 · **Size** S · **ADRs** —

**Scope**
- `docs/reference/README.md` explains the setup: Python 3.11 or later through uv, installing `../openflexure-microscope-server` in a virtual environment, building its web app, and running `--fallback -c ofm_config_simulation.json` on a spare port.
- `web/tests/reference/capture.ts`, a Playwright script that isn't run in CI. It captures every tab and key state (Control; Slide Scan idle and running; Gallery with items; each Settings section; Logging; About; Power; the calibration wizard). It captures each in light and dark, by setting OpenFlexure's persisted theme preference, at 1280×800 and 1920×1080. The output goes to `docs/reference/ofm/` (git-ignored).
- The same script with `--target microscope` captures our app with identical file names into `docs/reference/microscope/` (git-ignored), for side-by-side review.
- Measurements taken from the captures go in the notes: rail width, icon and label sizes, control-pane widths, control heights, spacing, and the colour roles. P06 builds on them.

**Done when**
- Running the script produces the full set, and the notes record the measurements.

<a id="p06"></a>
### P06: Design tokens and theme switching

**Goal.** Original theme tokens for light and dark, and a persisted Light, Dark or Follow System preference.
**Depends on** P04, P05 · **Size** M · **ADRs** 0011

**Scope**
- `src/theme/tokens.css` defines CSS custom properties:
  - colour: a magenta accent scale, light neutral surfaces, dark charcoal surfaces, text, borders, the focus ring and status colours;
  - typography: the system UI stack (Segoe UI on Windows), a size scale and light-weight headings;
  - layout: a 4 px spacing scale, radii, compact control heights, layout constants (rail width, pane widths) and elevation;
  - motion that respects `prefers-reduced-motion`.
- A theme store holding `light`, `dark` or `system`, persisted in `localStorage` with guarded access. It listens to `matchMedia` and sets `data-theme` on `<html>`. An inline bootstrap script in `index.html` prevents a flash of the wrong theme.
- A development-only route, `/#/dev/tokens`, shows every token in both themes.

**Done when**
- Switching the theme updates the page at once and survives a reload, and the tokens page has been reviewed against the references.

**Tests**
- The theme store: a change in the system theme, persistence failing (it falls back to `system`), and the bootstrap script agreeing with the store.

<a id="p07"></a>
### P07: App shell and navigation rail

**Goal.** The OpenFlexure-like frame: a vertical rail with the nine destinations, placeholder views behind routes, and keyboard tab switching.
**Depends on** P06 · **Size** M · **ADRs** 0012

**Scope**
- The rail: View, Control, Slide Scan, Sequence and Gallery at the top; Settings, Logging, About and Power pinned to the bottom. Each item shows an icon above its label. The active item is filled with the accent, and hover, focus and tooltips are handled.
- Material Symbols (Apache-2.0), imported as individual SVGs, with an entry in NOTICE.
- Routes `/#/view`, `/#/control`, `/#/slide-scan`, `/#/sequence`, `/#/gallery`, `/#/settings/:section?`, `/#/logging`, `/#/about` and `/#/power`, each with a placeholder view. Layout primitives: a fixed-width `ControlPane` and a `MainView`.
- Shift+↑ and Shift+↓ cycle through the tabs. A notice appears when the window is narrow or in portrait.
- A tab-availability hook: tabs whose required Things are missing are hidden. P11 wires it to real Thing Descriptions.
- The first captures of our own app, with a side-by-side review against the P05 references. Playwright visual baselines for the shell in both themes.

**Done when**
- The shell's proportions match the references within the tolerances the P05 notes set. Baselines are committed.

<a id="p08"></a>
### P08: UI components I: controls

**Goal.** The compact, original controls that every tab is built from.
**Depends on** P06 · **Size** L · **ADRs** 0013

**Scope**
- Button (primary, default, danger and ghost; small and regular sizes) and IconButton.
- TextField and NumberField (no spinners; Enter submits; min, max and step; an invalid state).
- Select and Accordion, built on Reka UI.
- Checkbox, Toggle, and Field (label, help and error).
- ProgressBar (determinate and indeterminate), Spinner, Card and SectionHeading.
- A development-only `/#/dev/components` gallery in both themes.

**Tests**
- Keyboard and ARIA behaviour, and `v-model` contracts.

<a id="p09"></a>
### P09: UI components II: overlays and feedback

**Goal.** Dialogs, confirmations, notifications and tooltips with consistent behaviour.
**Depends on** P08 · **Size** L · **ADRs** —

**Scope**
- Dialog (focus trap; Escape and outside-click policies) and `useConfirm()`, a promise-based confirmation with optional rich content.
- Toasts (success, info and error, with expandable details) and Tooltip.
- ErrorDetails, which renders `teta-wot` validation lists (422) and `{detail}` errors.

**Tests**
- Focus handling, Escape behaviour, confirm resolve and reject, toast timing, and each error shape.

<a id="p09b"></a>
### P09b: UI components III: menus, lists and shortcuts

**Goal.** Menus, paging, multiple choice and keyboard shortcuts with consistent behaviour. Split from P09 (Appendix D of the plan).
**Depends on** P09 · **Size** L · **ADRs** —

**Scope**
- Menu and ButtonMenu, Pagination, and MultiSelect.
- A keyboard-shortcut registry and the `?` help dialog that lists registered shortcuts. P07's Shift+↑/↓ moves onto it.

**Tests**
- Menu keyboard and ARIA behaviour, Pagination, MultiSelect's `v-model`, and the shortcut registry (conflicts, scoping to inputs).

---

## Stage 2: Client–server plumbing

<a id="p10"></a>
### P10: WoT client library

**Goal.** A small, typed client for `teta-wot` servers, driven by Thing Descriptions.
**Depends on** P02, P04 · **Size** L · **ADRs** 0014

**Scope**
- TD types for the subset of TD 1.1 this app uses, fetching `/thing_descriptions/`, and finding forms by `op` with `base` resolved.
- `readProperty`; `writeProperty` (JSON, encoding `false` and `0` correctly); `resetProperty`.
- `invokeAction`, which returns an `Invocation`: status polled with backoff, `cancel()` through `DELETE`, `output` and `log`. Also `ongoingInvocations(thing, action)`.
- Observation (`observeProperty`, `observeProperties`, `subscribeEvent`) through `fetch` and a `text/event-stream` parser, never `EventSource`, so headers can be added later. It reconnects with backoff.
- A typed `ApiError` covering the HTTP status, the 422 validation list, global lock busy, network or offline, and cancelled.
- `AbortSignal` support throughout, and an auth-header hook (unused in milestone 1).

**Tests**
- Unit tests with a fake `fetch` and scripted SSE streams, including chunks that split events.
- A contract test against the running server (the `system` Thing) in CI.

<a id="p11"></a>
### P11: Connection management

**Goal.** Connect to the local service or to a remote microscope, and keep the rest of the app unaware of where the server is.
**Depends on** P07, P09, P10 · **Size** M · **ADRs** 0015

**Scope**
- Two profiles: "This server" (the page's origin, shown as managed once `system.managed` exists in P44) and "Remote" (a base URL the user enters). Recent connections are remembered.
- A connection store with the states idle → connecting → connected → lost → reconnecting. It caches the Thing Descriptions, checks for the required Things (`system` for now; `camera` from P17), and puts the hostname in the window title.
- A `/#/connect` screen: the same-origin default, URL entry with validation, and the recent list. A `?microscope=` URL parameter connects directly.
- A connection indicator, an offline overlay with Retry, and tabs gated on Thing availability.

**Done when**
- The app served on one port connects to a server on another port or host (through CORS), and recovers after that server restarts.

**Tests**
- State-machine transitions, and an end-to-end connect and reconnect against the server.

<a id="p12"></a>
### P12: Serve the web app from the backend

**Goal.** One binary serves both the API and the app, while development can still use Vite.
**Depends on** P02, P04 · **Size** M · **ADRs** 0016

**Scope**
- Release builds embed `web/dist` (for example with `rust-embed`). `--webapp-dir <path>` serves from disk instead. A placeholder page appears when no build is present.
- The outer router answers `/`, `/index.html`, `/assets/{*path}` and the favicon. Hashed assets are cached as immutable; `index.html` is never cached.
- The build order is documented: build the web app, then `cargo build --release`. CI does the same and runs the e2e smoke test against the served app.

**Done when**
- `microscope-server` on its own serves the app at `/`, and the app connects to its own origin.

---

## Stage 3: Simulator and live microscope

<a id="p13"></a>
### P13: Simulation world, optics and blob specimen

**Goal.** The pure-Rust core of the image simulator: what the camera sees for any stage position and focus.
**Depends on** P01 · **Size** L · **ADRs** 0017

**Scope** (`microscope-sim`)
- Frames and units: stage steps to µm through a per-axis scale; the sample plane in µm; image pixels from the objective's magnification and the sensor's pixel pitch.
- A `Specimen` trait that renders an RGB region at a requested µm-per-pixel, deterministic for a given seed, with levels of detail for low magnification.
- `BlobSpecimen`: procedural coloured blobs drawn by our own generator, with configurable density, colours and size range. The sample is finite with empty margins, or repeating.
- `Optics`: objectives at 4×, 10×, 20×, 40×, 60× and 100×; field of view; depth of field; defocus blur from |z − z_focus| using a fast separable approximation; sensor noise.
- `render_frame(&SimState) -> RgbImage`, and `examples/render.rs`, which writes PNGs for review.
- A performance budget of about 40 ms for an 820×616 frame in a release build, measured and recorded in the notes but not a CI gate.

**Tests**
- Determinism, scale relationships, blur increasing with |dz|, and benchmarks as ignored tests.

<a id="p14"></a>
### P14: Simulated stage motion model

**Goal.** A realistic, testable model of an XYZ stage whose imperfections can be switched on and off.
**Depends on** P13 · **Size** M · **ADRs** —

**Scope** (`microscope-sim`)
- True and reported position, with a zero offset.
- Timed moves at a set speed, with an injectable clock; interrupt and stop; jogs where the latest command wins.
- Backlash per axis (a dead band), switchable.
- Travel limits: a finite range in which hitting a limit stops the axis and reports an error, switchable.
- `examples/trajectory.rs`.

**Tests**
- Property tests of backlash hysteresis and limits, and timing with a fake clock.

<a id="p15"></a>
### P15: Hardware interfaces and units

**Goal.** The seams that let milestone-2 drivers replace the simulator through configuration alone.
**Depends on** P02, P13 · **Size** M · **ADRs** 0018, 0019

**Scope** (`microscope-things`)
- `#[teta_wot::interface]` traits for what other Things need: `StageApi` (position, relative and absolute moves, stop, axis scale), `CameraApi` (grab a frame or JPEG, the next frame's size, capture to memory, stream information) and `IlluminationApi`.
- Shared types: `Position` (integer steps per axis), `AxisScale` (µm per step, optional) and `CaptureMetadata`.
- Conventions: one Thing type per driver (Things can't be generic), with shared logic in plain structs that each driver's Thing composes; SDK drivers behind `Device<D>` actors (milestone 2); OpenFlexure-mirrored names.
- A registry module, and `configs/simulation.json` naming the Things the plan adds as they land.
- Test fakes implementing each interface.

**Done when**
- The ADRs walk through three milestone-2 examples (Sangaboard stage in steps, a µm-native stage, an industrial camera SDK) and show that each fits.

<a id="p16"></a>
### P16: Simulated stage Thing

**Goal.** The `stage` Thing, mirroring OpenFlexure's, backed by the P14 model.
**Depends on** P14, P15 · **Size** M · **ADRs** —

**Scope**
- `stage` (`SimulatedStage`, implementing `StageApi`):
  - properties `position` (observable), `axis_names`, `moving` and `um_per_step`;
  - settings `backlash_steps` and `axis_inverted`;
  - actions `move_relative`, `move_absolute`, `move_to_origin`, `set_zero_position`, `jog` (no global lock, the latest command wins, `stop: true` halts it), `invert_axis_direction` and `stop`;
  - `calibration_required` and `can_calibrate`, for the z-direction check;
  - the event `arrived`.
- Simulation settings: speed, backlash on or off, and travel limits on or off with their range.

**Tests**
- Harness tests of every action, cancelling mid-move, interrupting a jog, settings persistence, and limit errors.

<a id="p17"></a>
### P17: Simulated camera and illumination Things

**Goal.** A live, stage-dependent camera, and the illumination it depends on.
**Depends on** P13, P16 · **Size** L · **ADRs** 0020

**Scope**
- `camera` (`SimulatedCamera`, implementing `CameraApi`), with the stage in a slot:
  - a preview thread, started in `#[on_start]` and stopped in `#[on_stop]`, renders from the stage's true position into `mjpeg_stream` and `lores_mjpeg_stream`;
  - `streaming_modes`, `streaming_mode`, `change_streaming_mode` and `stream_active`;
  - actions `grab_jpeg` (a Blob) and `grab_jpeg_size`, the buffer pair `capture_to_memory` and `save_from_memory`, `settle` (with a `settling_time` setting) and `discard_frames`;
  - settings for exposure, gain, objective, sample density and colour, noise, and a repeating sample, plus `load_sample` and `remove_sample`.
- `illumination` (`SimulatedIllumination`): `set_led` and `flash`.
- The client starts requiring `camera`.

**Tests**
- Frames change with stage x and y; sharpness falls as |dz| grows; LED off gives black frames; streams stop on shutdown.

<a id="p18"></a>
### P18: View tab: live image

**Goal.** The full-window live image, read with `fetch`.
**Depends on** P11, P17 · **Size** M · **ADRs** —

**Scope**
- A fetch-based MJPEG reader: it parses the multipart stream, decodes frames with `createImageBitmap` and draws them to a canvas. Viewers of the same URL share one stream through reference counting. The stream pauses when it is hidden (IntersectionObserver and page visibility), and its frame rate is capped.
- `LiveImage`: contain-fit with letterboxing; states for connecting, disabled, no connection and error; a flash on capture.
- The View tab, full window, and a "Disable stream" preference.

**Done when**
- The live image runs at 10 fps or more on the development machine, CPU use is recorded in the notes, and the stream stops when you leave the tab.

**Tests**
- The multipart parser (boundaries split across chunks) and reference counting; end to end, frames arrive.

<a id="p19"></a>
### P19: Generated API types and typed facades

**Goal.** TypeScript types that can't drift from the server.
**Depends on** P10, P17 · **Size** S · **ADRs** 0021

**Scope**
- `microscope-server --print-openapi` writes the OpenAPI document.
- `npm run api:types` (openapi-typescript) generates `web/src/api/generated/`. CI fails when the committed types are stale.
- Typed facades for `system`, `stage`, `camera` and `illumination`, built on the P10 client.

<a id="p20"></a>
### P20: Control tab: stage navigation

**Goal.** Moving around the sample the way OpenFlexure's Control tab does.
**Depends on** P09b, P18, P19 · **Size** L · **ADRs** —

**Scope**
- The Control layout: a narrow control pane beside the live image.
- A Position section: x, y and z fields; Refresh; Move (cancellable); Set Home; Move Home, with a "remove your sample" confirmation.
- A d-pad and focus ±: pointer capture, repeating while held, stopping on release.
- Keyboard jog (the arrow keys and PgUp/PgDn), and the mouse wheel over the image to focus.
- The live position, through property observation.
- Navigation preferences (step sizes and inversion), persisted, and the shortcuts listed in the `?` dialog.

**Done when (Checkpoint A)**
- The app shows the live simulated image, and the buttons, keys, wheel and typed coordinates all move the stage, with the image following.

**Tests**
- The jog controller (timers and key repeat); end to end, the d-pad and keys move the stage and the image changes.

---

## Stage 4: Focus, calibration and capture

<a id="p21"></a>
### P21: Autofocus

**Goal.** OpenFlexure-style fast autofocus driven by the live stream. **Spec-first.**
**Depends on** P17, P20 · **Size** M · **ADRs** 0022

**Scope**
- A sharpness monitor that records the MJPEG frames' JPEG sizes with timestamps, samples the stage position, and interpolates between them.
- `autofocus` with `fast_autofocus(dz, start)`, `looping_autofocus` and `z_move_and_measure_sharpness`. Each returns the focus curve.
- An Autofocus button in the control pane, and the `a` shortcut.

**Tests**
- It converges from random offsets within a set tolerance, with backlash on and off, and can be cancelled.

<a id="p22"></a>
### P22: Image registration core

**Goal.** The image-alignment primitive used by camera–stage mapping, stage measurement and (later) correlation stitching.
**Depends on** P13 · **Size** M · **ADRs** 0023

**Scope** (`microscope-core`)
- Phase correlation through an FFT: a window, the normalised cross-power spectrum, a sub-pixel peak, a confidence score, and downsampling for speed.
- An example that measures the shift between two rendered frames.

**Tests**
- Synthetic shifts, including sub-pixel accuracy; robustness to noise; the ambiguity of periodic patterns, documented.

<a id="p23"></a>
### P23: Camera–stage mapping and click-to-move

**Goal.** Calibrate the camera-to-stage relationship and double-click to move. **Spec-first.**
**Depends on** P21, P22 · **Size** L · **ADRs** —

**Scope**
- `camera_stage_mapping`:
  - `calibrate_xy`: moves each axis by about a tenth of the field of view, registers the images, fits a linear model, estimates backlash, and saves the calibration data as JSON in the data folder;
  - settings `image_to_stage_displacement_matrix`, `image_resolution` and `last_cal_timestamp`;
  - properties `last_calibration`, `calibration_required` and `can_calibrate`;
  - actions `move_in_image_coordinates`, `convert_image_to_stage_coordinates` and `convert_stage_to_image_coordinates`.
- In the simulator, a camera rotation and flip relative to the stage (a setting), so the matrix is not trivial.
- In the web app, double-clicking the live image moves there, accounting for contain-fit scaling, with a move lock.

**Tests**
- The recovered matrix matches the simulator's ground truth for several rotations and flips; click-to-move centres a feature within tolerance.

<a id="p24"></a>
### P24: Server-described UI elements

**Goal.** Let Things describe the hardware-dependent parts of the UI, and draw them generically.
**Depends on** P09b, P10 · **Size** L · **ADRs** 0024

**Scope**
- A Rust model of our own design, informed by OpenFlexure's idea:
  - header, text and bullet blocks;
  - `PropertyControl` (label, options, step, read-back, and an input style that includes duration as H:M:S);
  - `ActionButton` (label, confirmation, progress inline or in a dialog, the stream inside the dialog, cancel, a success message, refresh-on-response);
  - `DownloadButton`, `Accordion` and `Container`;
  - helpers that check affordance names when the description is built, and sanitised text (an allow-list).
- Web renderers: `ServerInterface`; `PropertyControl` (inputs that follow the schema, with a changed highlight); `ActionButton` (the invocation lifecycle, its log, cancel, and a dialog with a mini stream); `DownloadButton`. Each shows a "broken" state when an affordance is missing.

**Tests**
- Rust serialisation snapshots, and renderer component tests against fixtures.

<a id="p25"></a>
### P25: Specimens II and optical effects

**Goal.** A realistic sample and realistic optics.
**Depends on** P17, P24 · **Size** L · **ADRs** —

**Scope**
- A procedural tissue specimen: H&E-like stroma, nuclei and gaps, with control over coverage.
- An image-backed specimen: loads PNG, JPEG or TIFF through the `image` crate, with a µm-per-pixel scale, mip levels and bounds.
- A focal surface (a tilt plane plus low-frequency waviness), vignetting, uneven illumination, and the response to exposure and gain.
- The sample type can be switched at runtime. These settings are exposed through `manual_camera_settings` (server-described).

**Tests**
- Determinism; tissue has background regions; sampling of the loaded image; the focal surface changes sharpness across x and y.

<a id="p26"></a>
### P26: Capture to the data folder

**Goal.** Captures with metadata, saved for the gallery or downloaded.
**Depends on** P17, P20 · **Size** M · **ADRs** 0025

**Scope**
- The data folder layout (`captures/`, `scans/`, `sequences/`, `calibration/`) and the capture naming scheme.
- A JPEG plus a JSON sidecar recording the time, the stage position in steps and µm, the objective, exposure and gain, the camera–stage mapping if there is one, and the simulator state.
- `camera.capture(capture_mode, retain_image)` returns either a saved file or a Blob.
- Capture in the control pane: Save to gallery or Download, the `c` shortcut, and a flash.

**Tests**
- Metadata contents, Blob downloads, and path safety.

<a id="p27"></a>
### P27: Background detection and camera calibration

**Goal.** Telling sample from empty background, and calibrating the camera. **Spec-first.**
**Depends on** P24, P25 · **Size** L · **ADRs** —

**Scope**
- LUV colour conversion and sample-coverage computation (`microscope-core`).
- The Things `bg_color_channels_luv` and `bg_channel_deviations_luv`: tolerance, minimum sample coverage, stored background statistics and `settings_ui`.
- On `camera`:
  - the `background_detector_name` setting, `set_background` and `image_is_sample`;
  - `full_auto_calibrate` (in the simulator: remove the sample, set the background, load the sample again);
  - `primary_calibration_actions`, `secondary_calibration_actions`, `manual_camera_settings` and `calibration_required`.

**Tests**
- Background and tissue are classified correctly with vignetting and uneven illumination switched on.

<a id="p28"></a>
### P28: Settings tab

**Goal.** Every Settings section from OpenFlexure, plus Connection.
**Depends on** P23, P24, P27 · **Size** L · **ADRs** —

**Scope**
- Navigation between sections.
- Application:
  - Display: theme, fullscreen and disable stream;
  - Stage control preferences;
  - Connection: current, recent and switch.
- Microscope:
  - Launch calibration wizard (a button that P29 wires up);
  - Camera: calibration actions and manual settings, with a mini stream;
  - Stage: z direction and invert z, with a slot for the P30 tools;
  - Camera-to-stage mapping: calibrate (with a confirmation and a progress dialog showing the stream), details (with a matrix display), and downloading the calibration data.
- A reusable mini-stream component.

<a id="p29"></a>
### P29: Calibration wizard

**Goal.** First-run guidance through every calibration the microscope needs.
**Depends on** P28 · **Size** L · **ADRs** —

**Scope**
- A modal wizard made of tasks and steps.
- Tasks are discovered from Things that expose `calibration_required` and `can_calibrate`: stage z direction; camera calibration, including illumination and focus steps; and camera–stage mapping, including a focus step.
- Welcome and final steps.
- It opens on connect when anything needs calibrating. Escape asks before leaving. Settings can relaunch it with every task included.
- The explanatory copy is our own.

**Tests**
- How the task list is built; end to end, the first-run flow on a fresh settings folder.

<a id="p30"></a>
### P30: Stage measurement tools

**Goal.** Range-of-motion test and recentring. **Spec-first.**
**Depends on** P22, P23, P28 · **Size** M · **ADRs** —

**Scope**
- `stage_measure`:
  - `perform_rom_test` drives each axis toward its limits while tracking image motion, and reports the range and any parasitic z motion;
  - `perform_recentre` moves to the centre of the measured range and zeroes it;
  - the `calibrated_range` setting.
- Buttons in Settings → Stage, with a confirmation and a progress dialog. These use the simulator's travel limits (P14).

**Done when (Checkpoint B)**
- On a fresh settings folder the wizard runs. After it, autofocus, click-to-move and capture work, and every Settings panel is live.

---

## Stage 5: Gallery

<a id="p31"></a>
### P31: Gallery backend and data route

**Goal.** One index of everything the microscope has saved, and safe access to the files.
**Depends on** P26 · **Size** L · **ADRs** 0026

**Scope**
- A gallery index built from the data folder at start-up and updated on every write. Each card type has a provider: captures now, scans and sequences when they land.
- `gallery`:
  - `card_types`;
  - `list_data`: entries with a name, type, created time, thumbnail, viewer data, download buttons, card actions, messages and info;
  - `bulk_actions` and `delete_all_data(card_types)`.
- Delete endpoints per provider, and thumbnails made when items are saved.
- `GET /api/v1/data/{*path}`, with protection against path traversal and correct content types.
- A ZIP helper for downloads.

**Tests**
- Rebuilding the index, deletion, and rejecting traversal attempts.

<a id="p32"></a>
### P32: Gallery tab

**Goal.** Browse and manage saved items, as OpenFlexure's Gallery does.
**Depends on** P09b, P31 · **Size** L · **ADRs** —

**Scope**
- A card grid, detailed or thumbnail-only, at 18 or 25 cards per page. "Showing a–b of n", pagination and Refresh.
- A collapsible sidebar: a type filter, bulk actions, and Delete All, whose confirmation lists the types it will delete.
- Each card: a thumbnail (with a fallback), a title, a download button or menu, Delete with confirmation, action buttons or a menu, View, info lines, and warning or alert messages.
- The gallery refreshes when the tab becomes visible.

<a id="p33"></a>
### P33: Image and deep-zoom viewer

**Goal.** View images, image sequences and large mosaics.
**Depends on** P32 · **Size** M · **ADRs** 0027

**Scope**
- A viewer dialog using OpenSeadragon (BSD-3-Clause), for single images, DZI sources and directories (an image-sequence slider).
- Brightness, contrast and saturation filters with Reset; fullscreen; chrome that follows the theme.
- A small DZI fixture for tests, until P38 generates real ones.

**Done when (Checkpoint C)**
- Captures appear as cards that you can filter, page through, download, delete and open in the viewer.

---

## Stage 6: Slide scanning

<a id="p34"></a>
### P34: Scan planners

**Goal.** Paths for every workflow, as pure, well-tested code. **Spec-first.**
**Depends on** P13 · **Size** M · **ADRs** 0028

**Scope** (`microscope-core`)
- Grid geometry: the step is the field of view × (1 − overlap), converted to stage steps through the camera–stage mapping.
- `RegularGrid` (snake and raster, with x and y counts).
- `SmartSpiral`: spirals out from the start, grows around the sample, skips background, respects a maximum range and an equal-distances option, and reports why it finished.
- The C-Chip pattern.
- A z estimate from nearby sites that are already in focus.
- `examples/plan.rs` draws a planned path as SVG for review.

**Tests**
- Property tests (sites are unique, within range and adjacent), and scripted background maps for the spiral.

<a id="p35"></a>
### P35: Scan engine with snake and raster workflows

**Goal.** Run a scan end to end on the simulator.
**Depends on** P21, P23, P31, P34 · **Size** L · **ADRs** 0029

**Scope**
- The `ScanWorkflow` interface, and the Things `snake_workflow` and `raster_workflow` (overlap, autofocus range, x and y counts, capture mode, `settings_ui` and `ready`).
- `smart_scan`:
  - the `workflow_name` setting, `all_workflow_names` and `workflow_display_names`;
  - `sample_scan(scan_name)`, under a scan lock with unique names. For each tile it moves, autofocuses (using the z estimate) and captures;
  - a scan folder holding the images, per-image metadata, a snapshot of the settings and the scan log;
  - a return to the start position on error or cancel;
  - `latest_scan_live_details`, `purge_empty_scans`, `download_zip` and a delete endpoint.
- A scan gallery provider, using the directory viewer until the scan is stitched.

**Tests**
- An in-process scan on the simulator; cancelling mid-scan; an error mid-scan returns the stage to the start.

<a id="p36"></a>
### P36: Smart z-stacks

**Goal.** Capture several focal planes per tile and keep the best. **Spec-first.**
**Depends on** P35 · **Size** M · **ADRs** —

**Scope**
- `autofocus.run_smart_stack` and `run_basic_stack`: the number of images to test and to save, and dz. They pick the sharpest images, retry when the peak is at the edge, and can save on failure.
- Stack settings shared by the workflows, and a buffered capture-and-save pipeline.

**Tests**
- With a tilted focal surface, stacks keep the sharpest planes.

<a id="p37"></a>
### P37: Histo smart-spiral and C-Chip workflows

**Goal.** The two specialised workflows. **Spec-first.**
**Depends on** P27, P36 · **Size** L · **ADRs** —

**Scope**
- `histo_scan_workflow`:
  - a background detector in a slot, plus skip background, maximum range, equal distances and the stack settings;
  - `set_background` and `check_background`;
  - `settings_ui` with Background Detect and Scan Settings sections;
  - `ready`, which requires a camera–stage mapping and a background.
- `c_chip_workflow`: a counting-chamber grid with dense stacks.
- `histo_scan_workflow` becomes the default workflow.

**Tests**
- A spiral over a tissue island stops at its edge; C-Chip visits every site.

<a id="p38"></a>
### P38: Mosaic and deep-zoom pyramid core

**Goal.** Stitch tiles by position and write a deep-zoom pyramid, within bounded memory.
**Depends on** P13 · **Size** L · **ADRs** 0030

**Scope** (`microscope-core`)
- Tile placement from the stage positions through the camera–stage mapping, and feathered blending.
- A memory-bounded DZI writer (the tiles and the `.dzi` file), an overview JPEG and a thumbnail, and a preview mosaic at reduced scale.
- `examples/mosaic.rs`.

**Tests**
- Synthetic tiles cut from a known image reassemble within tolerance; pyramid level counts and tile sizes are correct.

<a id="p39"></a>
### P39: Live and final stitching

**Goal.** Show the mosaic grow during a scan, and finish it afterwards.
**Depends on** P35, P38 · **Size** M · **ADRs** —

**Scope**
- A preview stitcher updated after each tile. It serves the `smart_scan/latest_preview_stitch.jpg` endpoint, and its timestamp appears in the live details.
- A final stitch after the scan when `stitch_automatically` is set.
- `stitch_scan` and `stitch_all_scans`, which run without the global lock and off the async threads.
- Scan cards switch to the DZI viewer once a scan is stitched, and offer downloads of the tiles (ZIP) and the overview JPEG.

<a id="p40"></a>
### P40: Slide Scan tab

**Goal.** OpenFlexure's Slide Scan workflow in our UI.
**Depends on** P24, P33, P37, P39 · **Size** L · **ADRs** —

**Scope**
- The control pane: a workflow select with its blurb, the workflow's server-described settings, Stitching settings with a changed highlight, Sample ID, and Start Smart Scan.
- While running: the live stitching preview or the live image, a mini stream, the invocation log, a Cancel that names the current phase, and Close when it's done.
- Scan info: the Scan ID with a settings dialog, the number of images captured, and the duration. Download ZIP when complete.
- It picks up scans started elsewhere, for example in another browser window.

**Done when (Checkpoint D)**
- All four workflows run on the simulator, show the live stitching preview, and end as a stitched deep-zoom image in the gallery.

---

## Stage 7: Sequence

<a id="p41"></a>
### P41: Sequence engine (MDA-ready)

**Goal.** Timelapse parity, built on a model that can grow into multi-dimensional acquisition. **Spec-first.**
**Depends on** P21, P26, P31 · **Size** L · **ADRs** 0031

**Scope**
- `SequencePlan`: a time axis (interval and duration), and the steps run at each time point (an optional autofocus, then a capture). The step enum reserves z-stacks and multiple positions for later.
- A scheduler that skips late captures with a warning, and handles cancellation.
- Capture to memory, with a save queue.
- `sequence`: the settings `dt`, `duration`, `autofocus_enabled` and `autofocus_dz` (mirroring OpenFlexure's timelapse), `run_sequence(name)` and `download_zip`.
- A gallery provider: sequence cards with the directory viewer and its slider, the image count and the duration.

**Tests**
- Timing with a fake clock; skipping; cancelling keeps the images already captured.

<a id="p42"></a>
### P42: Sequence tab

**Goal.** Set up, run and follow a sequence.
**Depends on** P24, P41 · **Size** M · **ADRs** —

**Scope**
- A form: total duration as H:M:S, the interval, and an autofocus toggle with its range.
- While running: images captured and expected, time elapsed and remaining, a mini stream, the log, and Cancel.
- On completion, a link to the gallery entry.

**Done when (Checkpoint E)**
- A timelapse with autofocus runs, can be cancelled, and appears in the gallery with the image-sequence viewer.

---

## Stage 8: System, launcher and delivery

<a id="p43"></a>
### P43: Logging tab

**Goal.** Read the server's log in the app.
**Depends on** P03, P09b, P11 · **Size** M · **ADRs** —

**Scope**
- Groups of related records (by invocation, or consecutive records from the same source), each showing its highest level.
- A level filter from DEBUG to CRITICAL (default INFO), 10 groups per page, and expand and collapse.
- Refresh, Download log file, and states for an empty log and for everything filtered out.

**Tests**
- Grouping and filtering against a fixture.

<a id="p44"></a>
### P44: Sidecar process contract

**Goal.** A stable contract for any supervisor: the launcher now, Tauri later.
**Depends on** P02 · **Size** M · **ADRs** 0032

**Scope**
- `--managed` allows `--port 0`. Once listening, the server prints one line, `MICROSCOPE_READY {"url": …, "pid": …, "version": …}`.
- When stdin closes, the server shuts down gracefully.
- Exit codes: 0 stopped, 75 restart requested, and 1, 2 or 3 for errors (as in P02).
- `system` gains the `managed` property and the `shutdown` and `restart` actions (restart only when managed).

**Tests**
- Integration tests that spawn the binary: the ready line; restart gives 75; closing stdin gives 0 within the grace period.

<a id="p45"></a>
### P45: Local launcher

**Goal.** The managed local service: one command starts the microscope and opens the app.
**Depends on** P12, P44 · **Size** L · **ADRs** 0033

**Scope**
- `microscope-launcher` resolves the config (a bundled simulation default) and the data folders under `%LOCALAPPDATA%\Microscope\`, writing absolute paths into a generated config. It holds a single-instance lock.
- It starts the server with `--managed`, preferring port 5000 and falling back to a free port. It waits for the ready line, with a timeout, then opens the browser unless `--no-browser` is given.
- It restarts at once on exit code 75. After a crash it restarts with exponential backoff, up to a crash-loop limit.
- On Ctrl-C or when closed, it closes the server's stdin, waits, and then terminates it if needed.
- A Windows Job Object ensures the server never outlives the launcher.
- Server output goes to the console and to a launcher log.

**Tests**
- The supervisor logic with a fake process, and an integration test with the real server.

<a id="p46"></a>
### P46: About and Power tabs

**Goal.** Status and lifecycle control from the app.
**Depends on** P11, P44 · **Size** M · **ADRs** —

**Scope**
- About:
  - the hostname, the API origin, the server version and its source, and the OS;
  - the camera, stage and illumination types (from the TD titles), and Flash illumination;
  - a "managed by launcher" indicator;
  - our own intended-use statement (research and education; not a medical device), and links to the repository and its issues.
- Power:
  - Shut down the server, with confirmation;
  - Restart, only when managed;
  - Disconnect, for remote connections;
  - a "server stopped" screen with Reconnect.

<a id="p47"></a>
### P47: Visual fidelity and accessibility pass

**Goal.** Close the visual gap with the references, and make the app accessible.
**Depends on** P05 and every tab · **Size** L · **ADRs** 0034

**Scope**
- A side-by-side review of every tab and key state, in both themes and at both viewport sizes, followed by fixes to tokens and proportions.
- Playwright visual baselines for our app, committed (Windows CI only, with the tolerance documented).
- axe checks, and a keyboard-only walkthrough.
- The notes record the differences we keep on purpose.

<a id="p48"></a>
### P48: End-to-end, soak and performance

**Goal.** Evidence that the milestone works as a whole and keeps working.
**Depends on** every earlier phase · **Size** L · **ADRs** —

**Scope**
- A Playwright workflow suite against the real server:
  - connect, jog, autofocus, capture and see it in the gallery;
  - the calibration wizard and camera–stage mapping;
  - a snake scan ending in a stitched DZI, and a histo scan;
  - a sequence, the Logging tab, and Restart through the launcher.
- A soak test: streaming plus repeated scans for 30 minutes, with memory use and handle counts staying stable.
- Budgets recorded: preview frame rate, time per scan tile, and UI responsiveness.
- A nightly CI workflow for the long tests.

<a id="p49"></a>
### P49: Packaging and milestone close-out

**Goal.** Something you can hand to someone, and a clean start for milestone 2.
**Depends on** P45, P48 · **Size** M · **ADRs** 0035

**Scope**
- A release workflow that produces a Windows zip containing:
  - the launcher, and the server with the web app embedded;
  - the default configs, the README and the LICENSE;
  - third-party licences for the Rust crates (for example through cargo-about) and the npm packages;
  - notices for Material Symbols and OpenSeadragon.
- A user guide in the README: running it, connecting remotely, where data lives, and resetting settings.
- A milestone retrospective in the notes.
- A milestone-2 readiness checklist: driver interfaces, correlation stitching, the Linux and Pi question, and authentication.
