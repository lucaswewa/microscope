# Milestone 2 readiness

- Written: 2026-10-15, at the end of milestone 1 (P49)
- For: planning milestone 2, whose goal is real hardware ([plan §2](../milestone-1/implementation-plan.md#2-in-and-out-of-scope), D10)

Each section says what milestone 1 leaves in place, then what milestone 2 must decide or build. The boxes are for milestone 2's plan to tick or turn into phases.

## Driver interfaces

**In place** ([ADR-0018](../adr/0018-hardware-abstraction-through-teta-wot-interfaces.md), [ADR-0019](../adr/0019-stage-units-integer-steps-with-a-um-scale.md), [ADR-0020](../adr/0020-camera-frame-pipeline.md)):

- **Three interfaces** in `microscope-things::hardware`, which every Thing that uses hardware goes through:
  - `StageApi`: `position`, `move_relative`, `move_absolute`, `stop`, `set_zero` and `axis_scale`;
  - `CameraApi`: `grab_frame`, `grab_jpeg`, `frame_size`, `capture_to_memory`, `stream_info` and `lores_stream`;
  - `IlluminationApi`: `set_led` and `led_on`.
- **Slots name the interface** (`Slot<dyn StageApi>`), so a configuration file chooses the driver.
- **Shared structs** for each driver's Thing to compose: `BacklashCompensation`, `CaptureBuffer`, `PreviewFrames` and `BackgroundDetector`.
- **Fakes** of each interface, for testing the Things that use hardware.
- **Drivers bring their own settings panels,** described in Rust, with no change to the web app ([ADR-0024](../adr/0024-server-described-ui-elements.md)).
- **Walk-throughs** of a Sangaboard stage, a µm-native stage and an industrial camera SDK, each of which fits (ADR-0018).

**To decide or build:**

- [ ] **The first targets,** and hardware on the bench: a Sangaboard with a Pi camera, a camera SDK (Basler pylon, Spinnaker or IDS peak), a µm-native stage, or several.
- [ ] **A `Device<D>` actor** for a serial port or an SDK handle. `teta-wot` provides it, but this app hasn't used one yet.
- [ ] **Testing without hardware:** CI has none. Each driver needs a fake at its protocol, such as a serial-port fake for the Sangaboard or a recorded SDK session, and a manual check on the hardware before its pull request is merged.
- [ ] **Bit depth:** `CameraApi` hands other Things 8-bit RGB. A workflow that needs 12 or 16 bits needs a method added, which is an additive change.
- [ ] **A stage's scale:** a Sangaboard's µm per step is unknown until a calibration or a configured lead screw gives it. Until then, the UI shows steps.
- [ ] **The calibration wizard's gear check:** a real stage's panel needs a hold-to-turn button, a new element kind (P29's follow-ups).
- [ ] **Thresholds tuned on the simulator:** smart z-stacks' 3% sharpness rise (P36), autofocus's ranges and the background detector's limits should be measured on real images.

## Correlation stitching

**In place:**

- **Registration** by phase correlation, `microscope_core::registration::register`, which gives a shift and a confidence between 0 and 1 ([ADR-0023](../adr/0023-image-registration-by-phase-correlation.md), P22).
- **Placement by stage position:** `mosaic::Layout::new` turns each tile's stage position into its origin in the mosaic, through the camera–stage mapping. Blending and the deep-zoom writer then work from those origins, tile by tile ([ADR-0030](../adr/0030-position-based-stitching-with-dzi-output.md)).

**To decide or build:**

- [ ] **Something to correct:** the simulator's stage doesn't drift or lose steps, so correlation has nothing to fix there. A real stage's error, measured, shows how much it's needed. A simulated drift would let CI test it.
- [ ] **Registering neighbours:** crop each pair of overlapping tiles to their expected overlap, and register the strips. `register` needs same-size images, and finds shifts under half the compared square.
- [ ] **Solving globally:** pairwise shifts disagree, and chaining them adds up errors. Adjust all the origins at once, by least squares weighted by confidence, between `Layout::new` and the blending.
- [ ] **Empty overlaps:** a slide's background has nothing to register. Shifts with low confidence keep the stage's placement.
- [ ] **When it runs:** on the final stitch only, or on the live preview too, which has a time budget.

## The Linux and Raspberry Pi question

**In place** ([ADR-0003](../adr/0003-milestone-1-platform-windows-x86-64.md)):

- **Windows x86_64 only,** in CI and in the release. The visual baselines are Windows images.
- **Portable code:** the only platform-specific code is in the launcher, behind `cfg(windows)`: the Job Object, the console's signals, opening the browser, and `%LOCALAPPDATA%`. The other crates use only the standard library's paths and APIs.
- **`teta-wot`** is built and tested on Windows only.

**To decide or build:**

- [ ] **Where the server runs with OpenFlexure hardware:** on the Pi, as OpenFlexure does, with the app connecting by URL; or on a Windows computer, with the Sangaboard and camera on USB. The Pi camera's interface (libcamera) points to the Pi.
- [ ] **`teta-wot` on Linux,** x86_64 then ARM64: build it and run its tests. Problems there may need upstream changes, which [ADR-0004](../adr/0004-depend-on-teta-wot-v0-1-0-without-upstream-changes.md) ruled out for milestone 1.
- [ ] **A Linux CI job** for the Rust tests and the web app's unit tests. The visual baselines stay on Windows.
- [ ] **The launcher on Linux,** or a systemd service in its place: a data folder under XDG's directories, and a way to end the server with the launcher instead of the Job Object.
- [ ] **Performance on a Pi:** the preview's JPEG encoding, autofocus's sharpness and the FFT in registration, against the budgets P48 measures.
- [ ] **A release for ARM64:** cross-compilation, and its own licence lists ([ADR-0035](../adr/0035-release-packaging-and-third-party-notices.md); `release/about.toml` names its targets).

## Authentication

**In place:**

- **None, by design:** milestone 1 assumes a trusted network (plan D8). The README warns that `--host 0.0.0.0` has no password.
- **A client ready for a token:** `WotClient` takes headers for every request, and it reads streams and server-sent events with `fetch()`, which can carry them ([ADR-0014](../adr/0014-client-transports-td-forms-fetch-streams-polled-invocations.md)).
- **`teta-wot` can check credentials:** its configuration's `security` takes HTTP Basic or a bearer token, read from an environment variable, and `ThingServer::from_config` applies it.

**To decide or build:**

- [ ] **The scheme:** a bearer token suits the client and the launcher, which could create one and pass it to the server in an environment variable. Basic suits people typing a password.
- [ ] **The routes this app adds:** logs, the data route, the web app and health are merged beside `teta-wot`'s routes in `lifecycle::serve`. Check whether its check covers them, and add one where not. Health may stay open.
- [ ] **Images:** the gallery's cards and the scan and sequence pages load images with `<img>`, and the deep-zoom viewer fetches its tiles itself. Neither sends an `Authorization` header. They need a cookie, a token in the URL, or fetching through the client.
- [ ] **Where the web app gets the token:** a sign-in page, the launcher passing it in the URL's fragment, or one stored per connection through `src/host/` ([ADR-0010](../adr/0010-rules-for-a-tauri-ready-web-app.md)).
- [ ] **HTTPS:** a token sent over plain HTTP can be read on the network. Decide between TLS in the server, a reverse proxy, or a trusted network only.
- [ ] **Which actions need it:** all of them, or more for `shutdown`, `restart` and deleting from the gallery.
