# Milestone 2 phase specifications

Companion to the [implementation plan](implementation-plan.md). Each phase lists its goal, dependencies, size and ADRs, then its scope and how it's tested. Sizes and the Definition of Done are as in [milestone 1](../milestone-1/implementation-plan.md#7-how-phases-work), with [this plan's differences](implementation-plan.md#6-how-phases-work). Every phase also writes its notes in `docs/milestone-2/notes/`, with a **Hardware** section.

## Contents

- [Stage 0: Platforms](#stage-0-platforms): [P00](#p00) [P01](#p01) [P02](#p02)
- [Stage 1: Driver groundwork](#stage-1-driver-groundwork): [P03a](#p03a), [P03b](#p03b)
- [Stage 2: Stages](#stage-2-stages): [P04](#p04) [P05](#p05)
- [Stage 3: Cameras](#stage-3-cameras): [P06](#p06) [P07](#p07) [P08](#p08) [P09](#p09)
- [Stage 4: Real microscopes](#stage-4-real-microscopes): [P10](#p10) [P11](#p11)
- [Stage 5: Correlation stitching](#stage-5-correlation-stitching): [P12](#p12) [P13](#p13)
- [Stage 6: Security](#stage-6-security): [P14](#p14) [P15](#p15)
- [Stage 7: The desktop app and discovery](#stage-7-the-desktop-app-and-discovery): [P16](#p16) [P17](#p17) [P18](#p18) [P19](#p19)
- [Stage 8: Close-out](#stage-8-close-out): [P20](#p20)

---

## Stage 0: Platforms

<a id="p00"></a>
### P00: Milestone 2 groundwork

**Goal.** Milestone 2's folders, rules and platform decision in place.
**Depends on** — · **Size** S · **ADRs** 0038

**Scope**
- `docs/milestone-2/notes/` with its index, and the template's Hardware section.
- ADR-0038, the platforms (Windows x86_64, Linux x86_64, Linux ARM64 on the Pi 5), superseding ADR-0003.
- `CLAUDE.md` updated: the platforms, the hardware tests' environment variables, and the rule that a driver phase is Done after the owner's run.
- The owner's hardware, recorded: each device's model, interface and firmware.

**Tests**
- None beyond CI: docs only.

---

<a id="p01"></a>
### P01: Linux builds and CI

**Goal.** The workspace builds and passes its tests on Linux x86_64 and ARM64.
**Depends on** P00 · **Size** M · **ADRs** 0039

**Scope**
- `teta-wot` on Linux: what it needs (its shutdown signals, its Windows-only parts), and the decision in ADR-0039.
- CI jobs on Linux x86_64 and ARM64 (GitHub's ARM runners) for the Rust tests and the web app's unit tests.
- The end-to-end suite on Linux x86_64, without the visual baselines, which stay Windows images.
- What's Windows-only in this app (the launcher's Job Object, console signals and folders) behind `cfg`, with Linux equivalents or a clear "not on Linux".

**Tests**
- The whole Rust suite on all three platforms in CI.

---

<a id="p02"></a>
### P02: The Linux service and packages

**Goal.** A Linux machine, the Pi included, runs a microscope on its own.
**Depends on** P01 · **Size** M · **ADRs** —

**Scope**
- The launcher on Linux:
  - folders under XDG's directories;
  - the server ended with the launcher (`PR_SET_PDEATHSIG` or a process group);
  - SIGINT and SIGTERM.
- A systemd unit that runs the server headless, restarts it the way ADR-0033 does, and keeps its data in the service's folders.
- Release archives for Linux x86_64 and ARM64, and a `.deb` with the unit, beside the Windows zip.

**Tests**
- The launcher's integration tests on Linux.
- The release workflow's smoke test on each archive.

---

## Stage 1: Driver groundwork

<a id="p03"></a>
P03, the driver framework, was split in two: written whole, it came to about 1,200 lines.

<a id="p03a"></a>
### P03a: Device actors and serial ports

**Goal.** What every driver shares, ready before the first one: the stages' half.
**Depends on** P01 · **Size** M · **ADRs** 0041

**Scope**
- **Device actors:** the pattern for a driver's handle in a `teta-wot` `Device<D>`, with its errors, reconnection and shutdown.
- **A serial-port abstraction** with a fake, for the stages' protocol tests.
- **Hardware tests:** the convention (an environment variable names the device; the test is skipped without one), and a test helper.
- **Configurations:** how one names a device (a camera by serial number, a stage by port), and `configs/` laid out per hardware set.
- **ADR-0041,** for both halves ([plan §10](implementation-plan.md#10-points-for-your-review)).

**Tests**
- The serial fake's scripted conversations.
- A driver in a device actor, over the fake: its errors, reconnecting, and closing.

---

<a id="p03b"></a>
### P03b: The SDKs at run time, and listing devices

**Goal.** The camera SDKs, loaded when they're installed; and what every driver finds, listed.
**Depends on** P03a · **Size** L · **ADRs** 0041

**Scope**
- **SDKs at run time:**
  - checking that pylon's and Vimba X's C APIs load on all three platforms;
  - the loader, which reports a missing SDK clearly;
  - `microscope-sdk`, the one crate allowed unsafe code (ADR-0041).
- **Listing devices:** each driver can list what it finds, for writing a configuration (`microscope-server --list-devices`).

**Tests**
- The loader with a missing library.
- The SDKs, where they're installed.

---

## Stage 2: Stages

<a id="p04"></a>
### P04: Zaber stages

**Goal.** A Zaber XYZ stage drives the microscope.
**Depends on** P03a · **Size** L · **ADRs** 0042

**Scope**
- `ZaberStage`, providing `StageApi`, over Zaber's ASCII protocol on a serial port: the `zproto` crate, or our own client (ADR-0042).
- **Axes:** three, found by their device and axis numbers, in microsteps.
  - `axis_scale` comes from each axis's microstep size, so the stage is µm-native (ADR-0019).
- **Moves:** relative and absolute, waiting until idle; `stop`; homing; the device's own limits.
- **The position:** read while moving, for autofocus's monitor.
- **Errors and warnings** (stalls, limits, a lost connection), as action errors and log lines.
- **A settings panel** from the device: speed, acceleration, and homing.
- **A configuration** for a Zaber stage with a simulated camera, and with a real one.

**Tests**
- Against the serial fake: each command and reply, warnings, a stall, a timeout.
- **Hardware:** moves and their positions, stop, and homing, on the owner's stage.

---

<a id="p05"></a>
### P05: The Sangaboard

**Goal.** An OpenFlexure Sangaboard drives the microscope, on the Pi.
**Depends on** P03a · **Size** L · **ADRs** —

**Scope**
- `SangaboardStage`, providing `StageApi`, over the Sangaboard's serial protocol, written from its published documentation (ADR-0002).
- **Steps:** moves in motor steps, with the shared backlash compensation (P16 of milestone 1).
  - `axis_scale` comes from the configuration, or is unknown until calibrated.
- **Moves,** `stop`, the position, and the board's version and status.
- **The calibration wizard's gear check:** a hold-to-turn button, a new kind of server-described element (P29's follow-up).

**Tests**
- Against the serial fake: commands, replies, errors.
- **Hardware,** on the Pi: moves, positions and stop, on the owner's board.

---

## Stage 3: Cameras

<a id="p06"></a>
### P06: The SDK camera pipeline

**Goal.** What every SDK camera shares: frames in, previews and captures out.
**Depends on** P03b · **Size** L · **ADRs** 0043

**Scope**
- **A shared struct:**
  - frames from an SDK's callback into `PreviewFrames`, so the MJPEG streams and autofocus work as with the simulator;
  - conversion to 8-bit RGB for other Things: mono, Bayer, and 10–16-bit pixels scaled.
- **Captures:** `capture_to_memory` at full resolution, with the metadata ADR-0025 records (exposure, gain, the stage's position).
- **Features:** a model of a camera's GenICam-style features (exposure, gain, pixel format, white balance, binning), turned into a server-described settings panel (ADR-0024).
- **A fake SDK camera** that delivers frames by callback, for tests.

**Tests**
- Conversion of each pixel format.
- Frames from the fake reach the streams and captures.
- The panel from a feature list.

---

<a id="p07"></a>
### P07: Basler cameras

**Goal.** A Basler camera is the microscope's camera.
**Depends on** P06 · **Size** L · **ADRs** 0044

**Scope**
- `SdkCamera` (ADR-0044), providing `CameraApi`, through pylon (loaded as ADR-0041 decides), in a device actor.
- **Opening** a camera by serial number; listing those found.
- **Grabbing** continuously for the preview, and captures at full resolution.
- **Its features:** exposure, gain, pixel format, white balance, and a region of interest.
- **The camera calibration actions** (ADR-0024's panels) mapped to the camera's own auto-exposure and white balance.
- **Disconnects:** the camera unplugged and back, reported and reopened.

**Tests**
- Against the fake SDK camera (P06).
- **Hardware,** on the owner's camera, on Windows and Linux, and tried on the Pi: the live preview, captures, autofocus, and the settings.

---

<a id="p08"></a>
### P08: Allied Vision cameras

**Goal.** An Allied Vision camera is the microscope's camera.
**Depends on** P06 · **Size** L · **ADRs** —

**Scope**
- `AlliedVisionCamera`, providing `CameraApi`, through Vimba X's C API (VmbC), as P07 does for Basler: opening by serial number, grabbing, captures, features, calibration and disconnects.

**Tests**
- Against the fake SDK camera.
- **Hardware,** on the owner's camera, on Windows and Linux, and tried on the Pi.

---

<a id="p09"></a>
### P09: The Raspberry Pi camera

**Goal.** The Pi camera is the microscope's camera on a Pi 5.
**Depends on** P06, P02 · **Size** L · **ADRs** 0045

**Scope**
- Trying libcamera through Rust bindings, and `rpicam-vid` as a child process giving MJPEG, and recording the choice in ADR-0045.
- `PiCamera`, providing `CameraApi`:
  - the preview at the streams' sizes;
  - captures at full resolution;
  - exposure, gain and white balance;
  - its calibration (lens shading, white balance), as OpenFlexure's Pi camera does, in our own words.
- A configuration for the Pi camera with a Sangaboard.

**Tests**
- What can run off the Pi: parsing, configuration, the pipeline with recorded frames.
- **Hardware,** on the owner's Pi 5: the preview, captures, autofocus.

---

## Stage 4: Real microscopes

<a id="p10"></a>
### P10: Calibration and tuning on hardware

**Goal.** The calibration wizard and the workflows behave on real hardware as on the simulator.
**Depends on** P04, P05, P07, P08, P09 · **Size** M · **ADRs** —

**Scope**
- **The calibration wizard** on each hardware set: the z direction, the camera's calibration, and the camera–stage mapping.
- **The autofocus benchmark** (P54 of milestone 1) on real hardware, with true focus from a reference: a focus found by hand, or a calibration slide.
- **The thresholds tuned on the simulator,** measured per camera and set in its configuration:
  - autofocus's 3% rise;
  - smart stacks' rise;
  - the background detector's limits.
- **Configurations** for each hardware set, in `configs/`.

**Tests**
- The benchmark's numbers on each set, in the notes.
- A scan and a sequence on each set.

---

<a id="p11"></a>
### P11: The Raspberry Pi 5: performance and installing

**Goal.** A Pi 5 microscope is quick enough, and easy to set up.
**Depends on** P02, P05, P09 · **Size** M · **ADRs** —

**Scope**
- **The budgets of milestone 1's P48,** measured on the Pi: the preview's frame rate, a scan's time per tile, stitching's time and memory, and the app's responsiveness.
- **Fixes where they fall short:** smaller previews, faster JPEG encoding, work moved off the frame path.
- **An installation guide:** Raspberry Pi OS (64-bit), the `.deb`, the service, the camera's and the Sangaboard's setup, and connecting from a PC.

**Tests**
- The budgets on the owner's Pi, in the notes.
- The soak test, on the Pi, for an hour.

**Done when (Checkpoint A)**
- Basler with Zaber, and Allied Vision with Zaber, on a PC, and the Pi camera with a Sangaboard on a Pi 5, each calibrate, autofocus, scan and stitch.

---

## Stage 5: Correlation stitching

<a id="p12"></a>
### P12: Correlation placement

**Goal.** Where each image of a mosaic really is, from its overlaps.
**Depends on** P03a · **Size** L · **ADRs** 0046

**Scope**
- In `microscope-core`, pure and tested without hardware:
  - for each pair of overlapping images, register their expected overlaps (P22's phase correlation), with its confidence;
  - solve for every image's position at once, by least squares weighted by confidence;
  - keep an image's stage placement where it has no confident neighbour;
  - reject a pairwise shift too far from the stage's.
- **A drifting stage in the simulator:** steps lost and a slow drift, switchable at runtime, so CI can test the refinement.

**Tests**
- Synthetic mosaics, with known shifts, recovered.
- Featureless overlaps, which keep their stage placement.
- The simulator's drifting stage, whose scan is stitched within a pixel or two of the truth.

---

<a id="p13"></a>
### P13: Correlation in stitching

**Goal.** Final mosaics stitched with their images where correlation puts them.
**Depends on** P12, P10 · **Size** M · **ADRs** —

**Scope**
- **A stitching setting:** "Refine with image correlation", on by default for the final stitch. The live preview keeps stage positions.
- **`tiles.json`** records both placements.
- **A Corrections overlay layer** (milestone 1's P53) shows how far each image moved.
- **The scan's log** says how many images moved, and the largest move.

**Tests**
- A drifting simulated scan, stitched with and without refinement.
- **Hardware:** a scan on the Zaber stage and one on the Sangaboard.

**Done when (Checkpoint B)**
- A scan on the Zaber stage and one on the Sangaboard stitch without visible seams, and the overlay shows the corrections.

---

## Stage 6: Security

<a id="p14"></a>
### P14: Authentication on the server

**Goal.** A microscope only answers those with its token.
**Depends on** P02 · **Size** M · **ADRs** 0047

**Scope**
- **A token** made at the first start, kept in the settings folder, shown in the launcher's or service's log, and changeable.
- **`teta-wot`'s `security` configuration:** a bearer token from an environment variable, which the launcher and the service set.
- **Every route asks for it,** including this app's own (logs, the data route, health's details), except the web app's files and a bare health check.
- **ADR-0047:** tokens, where they're kept, and HTTPS (a trusted network in milestone 2, or TLS).

**Tests**
- Every route, with a token, without one, and with a wrong one.

---

<a id="p15"></a>
### P15: Authentication in the web app

**Goal.** The web app connects to a microscope that asks for a token.
**Depends on** P14 · **Size** M · **ADRs** —

**Scope**
- **A sign-in prompt** when a microscope asks for a token. The token is kept per connection, through the host adapter (ADR-0010).
- **The token on every request:**
  - streams and server-sent events, which already use `fetch` (ADR-0014);
  - images, through the client as blobs;
  - deep-zoom tiles, through OpenSeadragon's headers.
- **The local microscope** passes its token to the app it opens, without a prompt.

**Tests**
- Unit tests of the prompt, and of the token on each kind of request.
- End to end against a server with a token.

---

## Stage 7: The desktop app and discovery

<a id="p16"></a>
### P16: mDNS adverts

**Goal.** Microscopes say they're there.
**Depends on** P14 · **Size** S · **ADRs** 0048

**Scope**
- The server advertises `_microscope._tcp` with a pure-Rust mDNS crate. Its TXT record holds:
  - its name;
  - its version;
  - the API prefix;
  - whether it asks for a token.
- **A setting** turns the advert off, and it's only made when the server listens beyond this computer.

**Tests**
- An advert seen by a browse in the same test.

---

<a id="p17"></a>
### P17: The Tauri desktop app

**Goal.** The web app as a desktop app, with its own local microscope.
**Depends on** P15 · **Size** L · **ADRs** 0049

**Scope**
- **Tauri 2** around the web app's build, in a new `desktop/` folder.
- **A Tauri host adapter:** saving files, opening links, full screen and storage, so nothing else in the web app changes (ADR-0010).
- **The local microscope** as a sidecar, through ADR-0032's contract: started, restarted and stopped as the launcher does (ADR-0033).
- **The end-to-end suite** run against the desktop app's webview, on Windows and Linux.

**Tests**
- The host adapter's unit tests.
- The sidecar's start, restart and stop.
- The end-to-end suite in the app.

---

<a id="p18"></a>
### P18: Discovery in the desktop app

**Goal.** The desktop app finds microscopes on the network.
**Depends on** P16, P17 · **Size** M · **ADRs** —

**Scope**
- **Browsing** for adverts on the app's Rust side, given to the web app through a Tauri command and event.
- **The connection screen** lists those found: name, address, version, and whether it asks for a token. Connecting asks for the token if needed.
- **The browser app** keeps connecting by address.

**Tests**
- Unit tests of the list.
- A test server's advert found by the app.

---

<a id="p19"></a>
### P19: Desktop installers and signing

**Goal.** The desktop app installs cleanly and runs without warnings.
**Depends on** P17 · **Size** M · **ADRs** 0050

**Scope**
- **Installers** in the release workflow:
  - Windows (NSIS or MSI);
  - Linux AppImage and `.deb`;
  - with the third-party licences, as ADR-0035 does.
- **Signing** the Windows installer and binaries, with the owner's certificate or Azure Trusted Signing, from a secret in CI.
- **The user guide** for the desktop app.

**Tests**
- The release workflow's smoke test on each installer.
- A signed build run under Smart App Control.

**Done when (Checkpoint C)**
- On a Windows or Linux PC, the desktop app finds the Pi and a PC microscope on the network, signs in, and drives both.

---

## Stage 8: Close-out

<a id="p20"></a>
### P20: Hardware end to end, and close-out

**Goal.** Evidence that milestone 2 works on each hardware set, and a start for the next.
**Depends on** every earlier phase · **Size** M · **ADRs** —

**Scope**
- **A hardware checklist,** run by the owner on each set: connect, calibrate, autofocus, capture, scan and stitch, sequence, restart.
- **The soak test** on a real set.
- **A release** with every platform's packages.
- **The retrospective,** and a milestone-3 readiness checklist.

**Tests**
- The checklist's results, in the notes.
