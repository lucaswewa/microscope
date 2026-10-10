# P15: Hardware interfaces and units

- Status: Done
- Pull request: [#20](https://github.com/lucaswewa/microscope/pull/20)
- ADRs: [ADR-0018](../../adr/0018-hardware-abstraction-through-teta-wot-interfaces.md), [ADR-0019](../../adr/0019-stage-units-integer-steps-with-a-um-scale.md)
- Spec: [phases.md#p15](../phases.md#p15)

## Summary

The seams that let milestone-2 drivers replace the simulator through configuration alone:

- **Interfaces:** `StageApi`, `CameraApi` and `IlluminationApi`, as `teta-wot` interfaces that other Things reach through slots;
- **Shared types:** `Position` in whole steps, `AxisScale` in µm per step where known, and `CaptureMetadata`;
- **A registry module** listing the plan's Things;
- **Test fakes** for each interface.

A test proves the point: a Thing that uses hardware only through the interfaces is pointed at a different camera by its configuration's `thing_slots`, and uses it.

## What was built

All in `crates/microscope-things/`.

| Where | What |
|---|---|
| `src/hardware/mod.rs` | The conventions every driver follows |
| `src/hardware/stage.rs` | `StageApi`, `Position` (with `+`, `-` and negation) and `AxisScale` (converting to µm per axis, and back when every axis is known) |
| `src/hardware/camera.rs` | `CameraApi`, `CaptureMetadata`, `Capture` and `StreamInfo` |
| `src/hardware/illumination.rs` | `IlluminationApi` |
| `src/fakes.rs` | `FakeStage`, `FakeCamera` (its frame size configurable) and `FakeIllumination`, and `register()` to add them to a test's registry |
| `src/registry.rs` | `registry()`, moved from `lib.rs`, with a table of the plan's Things and the phases that add them |
| `tests/hardware.rs` | The tests below |

## How to try it

```powershell
cargo test -p microscope-things --test hardware
cargo doc -p microscope-things --no-deps --open   # the hardware module's conventions
```

## Design notes and deviations from the plan

ADR-0018 records the abstraction, with walk-throughs of a Sangaboard stage, a µm-native stage and an industrial camera SDK. ADR-0019 records the units. The details:

- **The interfaces hold what other Things need,** not everything a device can do:
  - `StageApi`: `position`, `move_relative`, `move_absolute`, `stop` and `axis_scale`;
  - `CameraApi`: `grab_frame`, `grab_jpeg`, `frame_size`, `capture_to_memory` and `stream_info`;
  - `IlluminationApi`: `set_led` and `led_on`.

  Async methods return boxed futures, as `teta-wot`'s macro requires of an interface. Answers known at once (`axis_scale`, `frame_size`, `stream_info` and `led_on`) are plain methods.
- **Frames are `image::RgbImage`**, 8-bit RGB, which is what analysis needs. ADR-0018 notes the limit for full-bit-depth cameras.
- **`CaptureMetadata`:**
  - `acquired`, as RFC 3339 text;
  - the image size;
  - the stage position and scale, if known;
  - µm per pixel, if known;
  - the camera's settings, by name.

  It's serialisable, for saving with images later (P26, P31).
- **Optional hardware connects by interface, with no default name.** A slot such as `#[slot(default = "illumination")] OptSlot<…>` fails to build when there is no Thing of that name: `teta-wot` requires a named default to exist even for an optional slot. The test's first version found this. A plain `#[slot]` connects to the one Thing providing the interface, if there is one.
- **The fakes are never in the server's registry.** Tests add them with `fakes::register`. They're plain enough to reason about:
  - **`FakeStage`** moves at once, at 0.1 µm per step, and remembers its targets.
  - **`FakeCamera`** returns grey frames of its configured size (64 × 48 by default) and captures twice that size, and counts them. Its "JPEG" is just the start and end markers.
  - **`FakeIllumination`** remembers whether it's on.
- **`configs/simulation.json` still names only `system`.** The registry's documentation lists the plan's Things and the phases that add them, and each phase adds its Thing to both.
- **A side effect:** `registry` is now both a module and a function. Two documentation links became ambiguous, and now point at the function, `registry()`.
- **Size:** 695 changed lines of hand-written code (205 of them tests) against M's budget of 600, above my estimate of about 600.

## Tests

5 tests (`tests/hardware.rs`):

- **Through the interfaces:** a test Thing (`Bench`) reaches the stage, camera and light only through their interfaces, in a server built from a configuration file. It switches the light on, moves there and back, stops, grabs a frame and takes a capture. The fakes record the moves and the capture.
- **Configuration alone chooses the hardware:** with two cameras and `thing_slots` pointing the bench at the second, a 320 × 240 one, the bench gets 320 × 240 frames, and only that camera counts a capture. Without a light, the optional slot is empty.
- **The fakes** aren't in the server's registry.
- **`Position`** adds, subtracts and negates, and goes on the wire as `{"x", "y", "z"}`.
- **`AxisScale`** converts each known axis to µm, converts back only when every axis is known, rounding to whole steps, and writes an unknown axis as `null`.

The server's tests still pass with the registry moved.

## Follow-ups and known gaps

- P16 writes `SimulatedStage`, which provides `StageApi` over the P14 model, and the shared backlash-compensation struct. P17 writes the simulated camera and illumination.
- Later Things name these interfaces in their slots: autofocus (P21), mapping (P23), scanning (P35).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The interfaces mirror OpenFlexure's names (ADR-0007) but are this project's own design.
