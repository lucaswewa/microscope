# P06: The SDK camera pipeline

- Status: In review
- Pull request: [#79](https://github.com/lucaswewa/microscope/pull/79)
- ADRs: [ADR-0043](../../adr/0043-the-sdk-camera-frame-pipeline.md)
- Spec: [phases.md#p06](../phases.md#p06)

## Summary

What every SDK camera shares, ready for the Basler and Allied Vision drivers. Frames from an SDK's callback become the preview the streams and autofocus use, and captures at full resolution. Pixels in every common format become 8-bit RGB. A camera's features make its settings panel. A fake SDK camera delivers frames by callback, for tests.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/pixels.rs` | `PixelFormat` (mono 8–16 bits, RGB, BGR, Bayer 8–16 bits in four arrangements), from GenICam's names; `RawFrame`; `to_rgb8` |
| `crates/microscope-things/src/hardware/frames.rs` | `FramePipeline`: `deliver` from the SDK's callback, `grab_frame`, `preview_size`, `capture` |
| `crates/microscope-things/src/hardware/features.rs` | `Feature`, `FeatureKind`, `STANDARD_FEATURES`, `settings_panel` |
| `crates/microscope-things/src/fakes.rs` | `FakeSdkCamera`: a thread that delivers frames to a callback |
| `crates/microscope-things/tests/sdk_camera.rs` | A camera Thing put together as the drivers' will be |

### The pipeline

| | |
|---|---|
| Preview | One frame every 100 ms at most, converted and shrunk to fit 1024 × 1024, to the main and low-resolution streams |
| Captures | The next frame whole, within 5 s, with the metadata given, and the frame's time and size |
| Frames skipped | Not converted |
| A frame that can't be converted | An error, for the captures waiting too |

### Every SDK camera's settings

| Feature | Property | Label |
|---|---|---|
| `ExposureTime` | `exposure_time` | Exposure (µs) |
| `Gain` | `gain` | Gain (dB) |
| `PixelFormat` | `pixel_format` | Pixel format |
| `BalanceWhiteAuto` | `white_balance` | White balance |
| `BinningHorizontal` | `binning` | Binning |

## How to try it

There's no camera driver yet: P07 and P08 are the first.

```sh
cargo test -p microscope-core pixels
cargo test -p microscope-things hardware::frames
cargo test -p microscope-things --test sdk_camera
```

## Hardware

None: the phase has no hardware. P07 and P08 bring it to the owner's Basler and Allied Vision cameras.

## Design notes and deviations from the plan

- **The order:** with no Sangaboard or Pi camera yet, the owner chose the cameras first. P06 to P08 come before P05 and P09, as the plan's overview and change log now say.
- **Conversion in `microscope-core`:** it's pure computation (ADR-0005). It isn't optimised in debug builds, so a full Bayer frame converts slowly there; a release build is fine, and the preview's frames are converted at most every 100 ms.
- **Bilinear demosaicking,** each channel averaged over the neighbours that have it: simple, and good enough to focus and to look at. Captures for measuring colour may want better, from the camera's own processing.
- **Fixed properties for the features:** a Thing's properties can't be made at run time. The five standard ones cover what the plan asks; a camera's others aren't shown.
- **No limits in the panel:** a property control can't carry a camera's own range, so the camera clamps or refuses a value, and the control reads back what it took.
- **A capture is the stream's next frame,** not a triggered exposure: simpler, and the same settings as the preview. A driver can add triggering if it needs it.
- **The size:** 782 changed lines (779 added, 3 removed), within L's 900. The owner chose one L pull request, over M's 600, when I estimated 850.

## Tests

- **Pixels:**
  - GenICam's names, and those that aren't read (packed, unknown, out of range);
  - mono, and 12 bits scaled;
  - RGB and BGR;
  - each Bayer arrangement of one colour coming out that colour, in 8 bits and in 10;
  - a frame too short for its size.
- **The pipeline:**
  - the preview takes a frame at most every 100 ms, shrunk to fit;
  - a capture takes the next frame whole, with its metadata, time and size;
  - a camera with no frames says so;
  - a frame that can't be converted fails the capture too.
- **Put together** (`tests/sdk_camera.rs`): frames from the fake SDK, 2048 × 1536 in 12 bits, reach the main stream at 1024 × 768, the low-resolution one at 320 × 240, and a capture at 2048 × 1536. The settings panel has exposure with its step, gain marked missing where the Thing lacks it, and the pixel formats the camera lists.

## Follow-ups and known gaps

- **P07, P08:** the Basler and Allied Vision drivers on this pipeline, with their SDK's features.
- **Autofocus and frame rates:** a sweep that slows to the camera's frame rate, so a slow camera, or a busy computer, still gives it enough frames. Raised in the flaky-tests pull request (#78), for the owner to decide.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
