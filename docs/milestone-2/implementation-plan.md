# Milestone 2 implementation plan

| | |
|---|---|
| Status | Approved 2026-10-16; a living document, updated as phases land (change log) |
| Date | 2026-10-16 |
| Covers | Milestone 2: real cameras and stages, Linux and the Raspberry Pi 5, correlation stitching, authentication, and the desktop app with discovery |
| Inputs | [`goals.md`](goals.md), the [readiness checklist](readiness.md), the planning Q&A (recorded in [§3](#3-decisions-made-while-planning)), and [milestone 1](../milestone-1/implementation-plan.md) |
| Companion | [phases.md](phases.md): the specification of every phase |

## Contents

1. [Milestone 2 in one page](#1-milestone-2-in-one-page)
2. [In and out of scope](#2-in-and-out-of-scope)
3. [Decisions made while planning](#3-decisions-made-while-planning)
4. [Ground rules](#4-ground-rules)
5. [Architecture](#5-architecture)
6. [How phases work](#6-how-phases-work)
7. [Phase overview](#7-phase-overview)
8. [Planned ADRs](#8-planned-adrs)
9. [Risks](#9-risks)
10. [Points for your review](#10-points-for-your-review)
11. [Appendix: change log](#appendix-change-log)

---

## 1. Milestone 2 in one page

Milestone 1 built the whole application against a simulated microscope. Milestone 2 runs it on real ones, on more platforms, and makes it a desktop app.

At the end of milestone 2:

- **Real hardware** replaces the simulator through configuration alone (ADR-0018):
  - **cameras:** Basler, through the pylon SDK; Allied Vision, through Vimba X; the Raspberry Pi camera, through libcamera;
  - **stages:** Zaber XYZ stages, through Zaber's ASCII protocol; the OpenFlexure Sangaboard, through its serial protocol.
- **Three platforms:** Windows x86_64, Linux x86_64, and Linux ARM64 on a Raspberry Pi 5. The Pi runs the server headless, with either hardware set.
- **Stitching refines** where each image goes by image correlation, so a real stage's drift and lost steps don't show as seams.
- **Authentication:** every microscope asks for a token, which the desktop app and the launcher handle.
- **The desktop app** (Tauri) runs on Windows and Linux PCs:
  - it starts a local microscope, as the launcher does now;
  - it finds microscopes on the network, which advertise themselves over mDNS;
  - it connects to them with their tokens.
- **Signed releases:** the Windows app and binaries are signed, so Smart App Control and SmartScreen let them run.
- **The simulator stays:** CI and development still run against it, beside the real drivers.

The work is split into **21 phases (P00–P20)** in **9 stages**, with three demo checkpoints ([§7](#7-phase-overview)). Hardware comes first, then stitching, then security, then the desktop app.

## 2. In and out of scope

**In scope**

- **Cameras:**
  - Basler, through pylon;
  - Allied Vision, through Vimba X;
  - the Raspberry Pi camera, through libcamera, on the Pi 5.
- **Stages:**
  - Zaber XYZ stages, through their ASCII protocol in Rust;
  - the Sangaboard, through its serial protocol.
- **Platforms:**
  - Linux x86_64 and Linux ARM64 (Raspberry Pi 5, 64-bit Raspberry Pi OS), beside Windows x86_64;
  - CI on all three;
  - a headless service for the Pi.
- **On real hardware:** the calibration wizard and the tuning of the thresholds milestone 1 set on the simulator (autofocus's, smart stacks', background detection's).
- **Correlation stitching:** registering overlapping images, then placing every image at once, by least squares.
- **Authentication by bearer token,** on every route, including images and deep-zoom tiles.
- **mDNS:** every server advertises itself on the network.
- **The Tauri desktop app** for Windows and Linux PCs:
  - the local microscope as a sidecar;
  - discovery of microscopes on the network;
  - installers.
- **Code signing** of the Windows release.

**Out of scope, and where it goes**

| Item | Where |
|---|---|
| Finding plugged-in cameras and stages automatically | Later. A configuration names each device (a camera by its serial number); drivers list what they find, to help write one |
| The desktop app on the Pi, or on macOS | Later. The Pi runs the server headless; any PC's desktop app or browser drives it |
| Automatic updates of the desktop app | Later |
| HTTPS in the server | Decided in P14's ADR: tokens over the trusted network in milestone 2, TLS or a reverse proxy later |
| Users and roles | Later. One token per microscope |
| Z-stack and multi-position sequences, pyramidal TIFF | As in milestone 1: after milestone 2 |
| Other cameras and stages (other SDKs, other controllers) | Later. The driver framework (P03) is meant to make them small additions |

## 3. Decisions made while planning

| # | Topic | Decision | Recorded in |
|---|---|---|---|
| D1 | AutoDiscovery | Microscopes on the network: each server advertises itself over mDNS, and the desktop app lists those it finds. A browser can't use mDNS, so the browser app keeps connecting by address | ADR-0046 (P16), P18 |
| D2 | Raspberry Pi 5 | The server runs on a Pi 5 with either hardware set: the Pi camera and a Sangaboard (OpenFlexure hardware), or the USB 3 / GigE cameras and a Zaber stage | ADR-0038 (P00) |
| D3 | Zaber stages | Zaber's ASCII protocol, in Rust (the MIT `zproto` crate, or our own client), rather than the C++ Zaber Motion Library: nothing to install, and the same on every platform | ADR-0041 (P04) |
| D4 | Camera SDKs | Basler through pylon, Allied Vision through Vimba X. The user installs the SDK; the project doesn't redistribute it | ADR-0040 (P03) |
| D5 | Platforms | Windows x86_64, Linux x86_64 and Linux ARM64. The desktop app is for Windows and Linux PCs; the Pi runs headless | ADR-0038 (P00) |
| D6 | Signing | The release workflow signs the Windows installer and binaries, with a certificate (or Azure Trusted Signing) the owner sets up | ADR-0048 (P19) |
| D7 | Authentication | A bearer token per microscope, which the desktop app and the launcher handle; the readiness checklist's gaps (our own routes, images) are closed | ADR-0045 (P14) |
| D8 | Order | Hardware first: platforms, then drivers, checked on real hardware; then stitching; then security; then the desktop app and discovery | [§7](#7-phase-overview) |
| D9 | Test hardware | The owner has a Basler camera, an Allied Vision camera, a Zaber XYZ stage and a Raspberry Pi 5. Their models are recorded in P00 | P00 |
| D10 | Delivery | As milestone 1: one branch and pull request per phase, merged after the owner approves it, with size budgets | [§6](#6-how-phases-work) |

## 4. Ground rules

Milestone 1's rules stand ([milestone 1 §4](../milestone-1/implementation-plan.md#4-ground-rules), [CLAUDE.md](../../CLAUDE.md)): independent implementation (ADR-0002), the Tauri-ready web app's rules (ADR-0010, ADR-0011, ADR-0013), dependencies' licences and age, LF line endings. Milestone 2 adds:

- **Hardware is checked on hardware.** CI can't reach a camera or a stage. Every driver has:
  - a fake at its protocol or SDK boundary, for CI;
  - hardware tests that run when an environment variable names the device, such as `MICROSCOPE_ZABER_PORT=COM3`;
  - a "How to try it" the owner runs on the real device before approving.
- **SDKs are the user's to install.** The project never redistributes pylon or Vimba X. Their licences allow our use, but their runtimes come from Basler's and Allied Vision's installers.
- **Protocols from their documentation.** The Sangaboard's protocol is implemented from its published description, in our own words, as ADR-0002 asks of anything OpenFlexure's GPL code also does.
- **The simulator stays first-class.** Every feature still works and is tested against it, so CI and development need no hardware.
- **Three platforms in CI.** Every pull request builds and tests on Windows, Linux x86_64 and Linux ARM64. The visual baselines stay Windows images (ADR-0034).

## 5. Architecture

### 5.1 Platforms

- **The server** builds for Windows x86_64, Linux x86_64 and Linux ARM64. Milestone 1 kept it portable (ADR-0003), apart from the launcher, and P01 finds what `teta-wot` needs on Linux.
- **On the Pi,** a systemd service runs the server, with its data in the service's folders. The launcher's job there is the service manager's (P02).
- **On a desktop,** the Tauri app takes over the launcher's job (P17). The launcher stays, for running without the app.

### 5.2 Drivers

- **One Thing type per device kind,** as ADR-0018 laid out:
  - `ZaberStage`, `SangaboardStage`;
  - `BaslerCamera`, `AlliedVisionCamera`, `PiCamera`.

  Each implements `StageApi` or `CameraApi`, so autofocus, mapping, scanning and stitching use them unchanged.
- **Device actors:** a device's handle (a serial port, an SDK camera) lives in a `teta-wot` `Device<D>` actor, which owns its thread, as ADR-0018 planned.
- **Shared structs,** as in milestone 1:
  - an SDK camera's frame pipeline, from the SDK's callback to `PreviewFrames`, with conversion to 8-bit RGB (P06);
  - backlash compensation, for the Sangaboard;
  - the capture buffer.
- **SDKs load at run time** (to be confirmed in P03, [§10](#10-points-for-your-review)): one server binary runs with or without pylon and Vimba X. A driver whose SDK isn't installed says so when it starts. CI builds every driver without the SDKs.
- **Settings panels come from the devices:** a camera's exposure, gain and so on are described by the server (ADR-0024), built from the features the SDK reports. The web app needs no change per camera.
- **A configuration per hardware set** in `configs/`, beside `simulation.json`, choosing a camera by serial number and a stage by port.

### 5.3 Correlation stitching

- **Stage positions place the images first,** as now (ADR-0030).
- **Then each pair of overlapping images is registered,** on their overlaps (P22's phase correlation), with its confidence.
- **A least-squares solve places every image at once,** weighted by confidence. Images with no confident neighbour keep their stage placement.
- **`tiles.json` records both placements,** so an overlay layer (P53) can show the corrections.
- **The live preview keeps stage positions,** for speed. The final stitch refines.
- **The simulator gains a drifting stage,** so CI can test the refinement.

### 5.4 Authentication

- **A token per microscope,** made at its first start and kept in its settings folder.
- **The launcher, the service and the desktop app** give the token to the server, through `teta-wot`'s `security` configuration, from an environment variable.
- **Every route asks for it,** including the routes this app adds (logs, data, the web app's API calls), except health and the web app's own files.
- **The web app sends it on every request.** Images and deep-zoom tiles are fetched through the client, so they can carry it too.

### 5.5 mDNS and discovery

- **The server advertises** `_microscope._tcp` with a pure-Rust mDNS crate. The advert's TXT record holds:
  - the microscope's name;
  - its version;
  - its API prefix;
  - whether it needs a token.
- **The desktop app browses for adverts,** on its Rust side, and the connection screen lists what it finds.
- **The browser app** keeps connecting by address, since browsers can't use mDNS.

### 5.6 The desktop app

- **Tauri 2,** around the same web app: a Tauri host adapter (ADR-0010) for files, links, full screen and storage.
- **The local microscope** is the server, as a sidecar, through ADR-0032's contract, as ADR-0033 planned.
- **Installers** for Windows (signed) and Linux (AppImage, .deb), built and published by the release workflow.

## 6. How phases work

As in milestone 1 ([its §7](../milestone-1/implementation-plan.md#7-how-phases-work)), with these differences:

- **Branches** are `m2/pNN-short-name`, and pull requests `PNN: <title>`.
- **Notes** go in `docs/milestone-2/notes/`, from milestone 1's template, with a **Hardware** section: the devices and platform it was tried on, and the owner's run.
- **ADRs** keep milestone 1's numbering, from 0038.
- **Size budgets** are the same: S ≤ 300, M ≤ 600, L ≤ 900 changed lines of hand-written code. Generated bindings don't count.
- **A driver phase is Done** only after the owner has run its "How to try it" on the device.

## 7. Phase overview

Sizes follow [§6](#6-how-phases-work). The detailed specs are in [phases.md](phases.md). Status runs Planned → In progress → In review → Done.

### Stage 0: Platforms

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P00 | Milestone 2 groundwork | — | S | 0038 | In review |
| P01 | Linux builds and CI | P00 | M | 0039 | Planned |
| P02 | The Linux service and packages | P01 | M | — | Planned |

### Stage 1: Driver groundwork

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P03 | The driver framework | P01 | M | 0040 | Planned |

### Stage 2: Stages

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P04 | Zaber stages | P03 | L | 0041 | Planned |
| P05 | The Sangaboard | P03 | L | — | Planned |

### Stage 3: Cameras

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P06 | The SDK camera pipeline | P03 | M | 0042 | Planned |
| P07 | Basler cameras | P06 | L | — | Planned |
| P08 | Allied Vision cameras | P06 | L | — | Planned |
| P09 | The Raspberry Pi camera | P06, P02 | L | 0043 | Planned |

### Stage 4: Real microscopes (ends with Checkpoint A)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P10 | Calibration and tuning on hardware | P04, P05, P07, P08, P09 | M | — | Planned |
| P11 | The Raspberry Pi 5: performance and installing | P02, P05, P09 | M | — | Planned |

**Checkpoint A, "Real microscopes":** Basler with Zaber, and Allied Vision with Zaber, on a PC; the Pi camera with a Sangaboard on a Pi 5. Each calibrates, autofocuses, scans and stitches.

### Stage 5: Correlation stitching (ends with Checkpoint B)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P12 | Correlation placement | P03 | L | 0044 | Planned |
| P13 | Correlation in stitching | P12, P10 | M | — | Planned |

**Checkpoint B, "Seamless mosaics":** a scan on the Zaber stage and one on the Sangaboard stitch without visible seams, and the overlay shows how far correlation moved each image.

### Stage 6: Security

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P14 | Authentication on the server | P02 | M | 0045 | Planned |
| P15 | Authentication in the web app | P14 | M | — | Planned |

### Stage 7: The desktop app and discovery (ends with Checkpoint C)

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P16 | mDNS adverts | P14 | S | 0046 | Planned |
| P17 | The Tauri desktop app | P15 | L | 0047 | Planned |
| P18 | Discovery in the desktop app | P16, P17 | M | — | Planned |
| P19 | Desktop installers and signing | P17 | M | 0048 | Planned |

**Checkpoint C, "Desktop app":** on a Windows or Linux PC, the desktop app finds the Pi and a PC microscope on the network, signs in with their tokens, and drives both.

### Stage 8: Close-out

| ID | Phase | Depends on | Size | ADRs | Status |
|---|---|---|---|---|---|
| P20 | Hardware end to end, and close-out | all | M | — | Planned |

**Order.** The table order is the planned sequence, one phase at a time. P12 (pure Rust, on the simulator) and P14–P16 depend only on early phases, so they can move earlier if hardware is late or a device needs a vendor's help. Any reorder is agreed first, and recorded in the [change log](#appendix-change-log).

## 8. Planned ADRs

Numbers are provisional, assigned in the order written.

| ADR | Title | Phase |
|---|---|---|
| 0038 | Milestone-2 platforms: Windows, Linux and the Raspberry Pi 5 (superseding 0003) | P00 |
| 0039 | `teta-wot` on Linux | P01 |
| 0040 | Hardware drivers: SDKs, device actors and hardware tests | P03 |
| 0041 | Zaber stages through the ASCII protocol | P04 |
| 0042 | The SDK camera frame pipeline | P06 |
| 0043 | The Raspberry Pi camera through libcamera | P09 |
| 0044 | Correlation stitching (superseding part of 0030) | P12 |
| 0045 | Authentication by token | P14 |
| 0046 | mDNS adverts and discovery | P16 |
| 0047 | The Tauri desktop app (superseding part of 0033) | P17 |
| 0048 | Code signing and desktop distribution | P19 |

## 9. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| pylon and Vimba X support Linux ARM64 but don't list the Pi 5 as tested | A camera may not work, or work slowly, on the Pi | Try both on the Pi early (P07, P08); fall back to the Pi camera on the Pi and the SDK cameras on PCs if needed, recorded in the notes |
| `teta-wot` is built and tested on Windows only | Linux may need changes in it (ADR-0004 ruled them out for milestone 1) | P01 finds out first; whether to change `teta-wot` is a review point ([§10](#10-points-for-your-review)) |
| Hardware-only bugs, which CI can't catch | Regressions between the owner's runs | Protocol fakes in CI; hardware tests behind environment variables; the owner's run before each driver phase is Done |
| The Pi 5's speed | A slow preview, autofocus or stitching | Budgets measured on the Pi (P11), as P48 did on Windows; lower-resolution previews; work in release builds |
| GigE cameras need network setup (jumbo frames, addressing) | Frames dropped or cameras unseen | Document the setup; try USB 3 first where the camera has both |
| WebView2 and WebKitGTK differ (the desktop app on Windows and Linux) | Streams or layout may misbehave on Linux | The app's streams already use `fetch` (ADR-0014); run the end-to-end suite in the Linux desktop app's webview as well |
| mDNS is blocked on some networks | Discovery finds nothing | Connecting by address stays; the notes say what the network must allow |
| Signing needs a certificate and a secret in CI | Unsigned builds are blocked by Smart App Control | The owner's certificate is a prerequisite of P19; until then, releases stay unsigned, as now |
| Calibrations and thresholds were tuned on the simulator | Autofocus or background detection misjudge real images | P10 measures them per camera, with the autofocus benchmark adapted to real focus |

## 10. Points for your review

These are choices I made while writing the plan. Each is easy to change now and harder later.

1. **SDKs loaded at run time,** through their C APIs (Vimba X's VmbC, and pylon's C API), so that one binary runs with or without them and CI builds every driver. The alternative is building against them (the `pylon-cxx` crate for Basler), behind cargo features, which needs each SDK wherever the server is built. P03 checks that pylon's C API is available on all three platforms before committing.
2. **`teta-wot` on Linux:** if it needs changes, may they go into `teta-wot` itself, as your repository, superseding ADR-0004? Or should this app work around them?
3. **Hardware tests by hand, or a runner:** I've planned for you to run each driver's checks. A self-hosted GitHub runner on a PC with the devices attached could run them on every pull request, at the cost of keeping that PC on.
4. **The Pi camera:** libcamera through Rust bindings, or `rpicam-vid` as a child process giving MJPEG. P09 starts by trying both.
5. **HTTPS:** tokens over plain HTTP on a trusted network in milestone 2, with TLS later. Or self-signed TLS from the start?
6. **The Sangaboard's protocol,** written from its published documentation in our own words, as ADR-0002 asks.
7. **Your hardware's models:** the cameras' interfaces (USB 3 or GigE), the Zaber controller and axes, and the Pi camera module and Sangaboard version, recorded in P00.
8. **Desktop app updates:** left out of milestone 2. Should they be in?

## Appendix: change log

| Date | Change |
|---|---|
| 2026-10-16 | First draft for review |
| 2026-10-16 | Approved. §10's points stand as proposed, except two still open: whether `teta-wot` may change for Linux (asked in P01) and the owner's hardware models (asked in P00, recorded in [hardware.md](hardware.md)) |
