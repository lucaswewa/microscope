# P17: Simulated camera and illumination Things

- Status: In review
- Pull request: (added once opened)
- ADRs: [ADR-0020](../../adr/0020-camera-frame-pipeline.md)
- Spec: [phases.md#p17](../phases.md#p17). P17 was split, at the owner's choice: the capture buffer and settling are now P17b (plan Appendix D).

## Summary

A live camera that shows the sample where the simulated stage really is, and the LED that lights it:

- **`camera` (`SimulatedCamera`):**
  - a preview thread drawing about 10 frames a second into two MJPEG streams;
  - two streaming modes;
  - JPEG grabs;
  - settings for the objective, the sample, the noise and the exposure;
  - removing and loading the sample.

  It provides `CameraApi`, including captures with metadata.
- **`illumination` (`SimulatedIllumination`):** `set_led` and `flash`. While the LED is off, frames are black.
- **The web app** now requires the `camera` Thing to connect.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/camera.rs` | `SimulatedCamera`, its streaming modes and settings types, and its `CameraApi` implementation |
| `crates/microscope-things/src/illumination.rs` | `SimulatedIllumination` and its `IlluminationApi` implementation |
| `crates/microscope-things/src/hardware/preview.rs` | `PreviewFrames`, the shared struct that publishes a frame to both streams and keeps it as pixels, and `encode_jpeg` |
| `crates/microscope-sim/src/optics.rs` | `Sensor::CAPTURE`: 1640 × 1232, the preview's field of view in twice the detail |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | Register and run `camera` and `illumination` |
| `Cargo.toml` | The `image` crate's `jpeg` feature, and debug builds that optimise `microscope-sim`, `image` and `zune-jpeg` |
| `web/src/connection/store.ts` | `REQUIRED_THINGS` is `system` and `camera` |
| `crates/microscope-things/tests/camera.rs`, `web/tests/unit/connection/` | The tests below |

The camera's interface:

| Kind | Names |
|---|---|
| Streams | `mjpeg_stream` (820 × 616, or 1640 × 1232), `lores_mjpeg_stream` (320 × 240) |
| Properties | `streaming_modes`, `streaming_mode`, `stream_active`, `sample_loaded` |
| Settings | `objective` (4, 10, 20, 40, 60 or 100), `blob_density` (per mm²), `colour` (hex colours separated by semicolons), `repeating`, `noise_level`, `exposure_time` (µs), `analogue_gain` |
| Actions | `change_streaming_mode`, `grab_jpeg`, `grab_jpeg_size`, `remove_sample`, `load_sample` |

The illumination's: the property `led_on`, and the actions `set_led` and `flash`.

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

Then open these in a browser:

- `http://127.0.0.1:5090/api/v1/camera/mjpeg_stream/viewer`, the live stream;
- `http://127.0.0.1:5090/api/v1/docs`, to move the stage, change the settings, or switch the LED off.

Or from PowerShell:

```powershell
$api = 'http://127.0.0.1:5090/api/v1'
Invoke-RestMethod -Method Post "$api/stage/move_relative" -ContentType application/json -Body '{"x": 500}'
Invoke-RestMethod -Method Post "$api/stage/move_relative" -ContentType application/json -Body '{"z": 200}'    # 10 µm out of focus
Invoke-RestMethod -Method Put "$api/camera/objective" -ContentType application/json -Body '10'
Invoke-RestMethod -Method Post "$api/illumination/flash" -ContentType application/json -Body '{"number_of_flashes": 3}'
```

## Design notes and deviations from the plan

ADR-0020 records the frame pipeline and its measurements. In short, release builds draw and publish a preview frame in about 32 ms in focus, and 47 ms at 20 µm defocus. That's within the 100 ms budget, so the stream runs at about 10 fps. Debug builds now manage about 10 fps, and 8 fps defocused.

- **The camera's stage slot names `SimulatedStage`, not `StageApi`.** Only the simulator knows where the sample really is (`true_position_um`, which includes backlash). A real camera needs no stage to draw. Captures take the reported position and scale through `StageApi`.
- **The illumination slot is optional** (by interface, with no default name, as ADR-0018 says). Without an `illumination` Thing, the light is always on.
- **`full_resolution` streams at the capture size,** 1640 × 1232, at about 4 fps in release. OpenFlexure's simulator has the mode but streams the same frames in both.
- **The settings' names are OpenFlexure's simulated camera's**: `objective`, `blob_density`, `colour`, `repeating` and `noise_level`, and the Pi camera's `exposure_time` and `analogue_gain`. Our own model:
  - **`blob_density`** is per mm², not per million pixels.
  - **`colour`'s default** is the P13 palette of four colours.
  - **`repeating`** repeats the sample every 2 mm (2,016 µm, a whole number of the specimen's cells). Otherwise the sample is 20 × 20 mm.
  - **Brightness** is exposure relative to 10 ms, times gain. P25 brings the fuller optical model (vignetting, uneven illumination).
  - **Invalid values are rejected:** an objective of 7, `"red"` as a colour, or negative noise. OpenFlexure logs a warning and ignores them.
- **`remove_sample` and `load_sample`** fail when the sample is already out or in, as in OpenFlexure. The state is the read-only property `sample_loaded`. It isn't saved, so a restart puts the sample back.
- **`flash`** turns the LED off for `dt` seconds, then on for `dt` seconds, `number_of_flashes` times. It always ends on, even when cancelled. `dt` must be from 0 to 60 seconds.
- **`change_streaming_mode`** takes `default` or `full_resolution`, and rejects anything else with a 422. OpenFlexure falls back to the default.
- **Frame timing:** a frame grabbed just after a change may have been started before it. Tests grab twice, and P17b adds `discard_frames` and `settle` for this.
- **Dependencies:** `image`'s `jpeg` feature adds `zune-jpeg` 0.5.15 and `zune-core` 0.5.3, both MIT/Apache-2.0/Zlib and older than two weeks. `chrono`, already in the lock file through `teta-wot`, timestamps captures.
- **Size:** 1,071 changed lines of hand-written code (425 of them tests) against L's budget of 900, above my estimate after the split of about 750.

## Tests

9 camera tests (`crates/microscope-things/tests/camera.rs`), with the noise off so the same view draws the same frame:

- **Frames follow the stage:** a 28 µm move in x or y shifts the image by exactly 250 pixels at 40×. Moving back gives the identical frame.
- **Sharpness falls with defocus:** the frames at 0, 2, 5, 10 and 20 µm from focus get steadily less sharp, and 5 µm below focus is as blurred as 5 µm above.
- **The LED:** with it off, frames are all black; on, they're lit. Without an illumination Thing, they're lit.
- **`flash`** turns the LED off and ends with it on, even when cancelled while off. A negative `dt` is an error.
- **Streams:** `grab_jpeg` serves an 820 × 616 JPEG from the main stream and a 320 × 240 one from the lores stream, and `grab_jpeg_size` gives their sizes. When the server stops, the stream ends and its clients' streams finish.
- **Streaming modes:** both are listed, `full_resolution` gives 1640 × 1232 frames, and an unknown mode is rejected.
- **Settings:**
  - half the exposure halves the brightness, and twice the gain restores the identical frame;
  - the objective, colour and density each change the frame, and restoring them restores it;
  - invalid values are rejected.
- **The sample:** removing it leaves an even, empty slide, and removing it twice is an error. Loading it restores the frame. 15 mm out, past the sample's edge, the slide is empty unless the sample repeats.
- **`CameraApi`:**
  - the frame size and stream information;
  - a decodable JPEG;
  - a 1640 × 1232 capture whose metadata has the stage position, the scale, 0.056 µm per pixel at 40×, the objective and a UTC timestamp.

The web app's connection tests now also check that a server without `camera` is reported as missing it.

**Mutation checks:** I broke each of the following, and a test failed every time:

- following the stage;
- the LED;
- exposure;
- removal;
- repeating;
- the colour;
- the objective;
- the streaming mode;
- stopping the stream on shutdown;
- shrinking the lores frames;
- the capture's scale;
- the flash ending on;
- refusing a second removal.

The camera tests take about 2 s, and twelve runs in a row passed.

## Follow-ups and known gaps

- **P17b:** `capture_to_memory`, `save_from_memory` and `clear_buffers` over a shared capture buffer, and `settle` and `discard_frames`.
- **P18** shows `mjpeg_stream` in the View tab. **P25** adds tissue and image-backed samples, a tilted focal plane, vignetting and uneven illumination. **P26** saves captures with their metadata.
- **The preview draws frames even when nobody watches.** Pausing it could save CPU later (ADR-0020).
- **The plan's camera row** also lists `capture`, `full_auto_calibrate`, `set_background`, `image_is_sample` and the calibration properties. Those belong to P26 and P27.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The names mirror OpenFlexure's camera and illumination (ADR-0007), and I read its source for behaviour only. The drawing is P13's own simulator, and the pipeline design is this project's own (ADR-0020).
