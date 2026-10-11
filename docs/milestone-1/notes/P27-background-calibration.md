# P27: Background detection and camera calibration

- Status: Done
- Pull request: [#37](https://github.com/lucaswewa/microscope/pull/37)
- ADRs: none new
- Spec: [phases.md#p27](../phases.md#p27)

## Behaviour spec

Written before the code, in this project's own words, from what OpenFlexure's background detectors and its simulated camera's calibration do.

**Colour.** Images are compared in CIE 1976 L\*u\*v\*, computed from sRGB with the D65 white point.
- **L\*** is the lightness, from 0 to 100.
- **u\* and v\*** place the colour: 0 for greys, so the empty slide's near-white is close to 0.
- **Why u\* and v\*:** vignetting, uneven lighting and focus change the lightness much more than u\* and v\*.

**Two background detectors,** each a Thing. The camera uses one, by name.

- **`bg_color_channels_luv` compares each pixel's colour with the background's.**
  - **Setting the background:** from an image of the empty slide, take each channel's mean and standard deviation over the whole image.
  - **A pixel is background** if both its u\* and its v\* are within `channel_tolerance` deviations of the background's mean. L\* is ignored.
  - **Sample coverage** is the percentage of pixels that aren't background.
- **`bg_channel_deviations_luv` compares texture.**
  - **The grid:** the image is divided into an 8 × 8 grid of cells, and each cell's standard deviation is taken in each channel.
  - **Setting the background:** take the median, over the cells, of each channel's deviation.
  - **A cell holds sample** if any channel's deviation is more than `channel_tolerance` times the background's.
  - **Sample coverage** is the percentage of cells that hold sample.
- **Both detectors:**
  - **Settings:**
    - `channel_tolerance`, at least 0, by default 7;
    - `min_sample_coverage`, a percentage, by default 25.
  - **Deviations:**
    - **A blank image is refused:** setting the background from an image in which a channel doesn't vary at all fails ("the image is blank…"). It usually means the camera isn't ready.
    - **A floor:** deviations below 0.5 are raised to 0.5, so that a very clean background doesn't make faint noise look like sample.
  - **The background's statistics** are kept as read-only settings, so they last across restarts. `ready` is true once they're set.
  - **Sample or not:** an image holds sample if its coverage is more than `min_sample_coverage`. The message says "x% sample", or "only x% sample" when it isn't.
  - **Unset background:** without a background, asking fails ("the background isn't set: calibrate the camera first").
  - **A panel:** `settings_ui` describes the two settings, served as ADR-0024 says.

**On the camera:**
- **`background_detector_name`,** a setting, names the detector in use.
  - **The default:** `bg_channel_deviations_luv` if the server has it, or else the first detector it has.
  - **An unknown name** is refused.
- **`set_background`** grabs a frame, shrinks it to the low-resolution stream's 320 × 240, and sets the detector's background from it.
- **`image_is_sample`** does the same, and returns whether the image holds sample, its coverage and the message.
- **`full_auto_calibrate`,** in the simulator:
  1. take the sample out (unless it's already out);
  2. wait for a frame drawn after that;
  3. set the background;
  4. put the sample back, even if setting the background failed, and report the failure after.
- **`calibration_required`** is true without a detector, or while the detector in use isn't ready.
- **`primary_calibration_actions`** is a panel with "Full auto-calibrate". **`secondary_calibration_actions`** has "Set background", "Remove sample" and "Load sample". Both are served as panels (ADR-0024), as `manual_camera_settings` is (P25).

## Summary

The microscope can now tell sample from the empty slide, and the simulated camera can be calibrated for it.

- **`microscope_core::background`:**
  - sRGB → CIE L\*u\*v\*;
  - two ways of comparing an image with a background, by each pixel's colour (`ColourStatistics`) and by each region's texture (`TextureStatistics`).
- **Two background detector Things,** `bg_color_channels_luv` and `bg_channel_deviations_luv`, behind a `BackgroundDetector` interface. Each has its tolerance and coverage settings, its stored statistics and a `settings_ui` panel.
- **The camera:**
  - **The detector:** it uses one detector, named by `background_detector_name`.
  - **Actions:** `set_background`, `image_is_sample` and `full_auto_calibrate`.
  - **`calibration_required`** says whether it still needs calibrating.
  - **Panels:** `primary_calibration_actions` and `secondary_calibration_actions`, for the Settings tab (P28) and the calibration wizard (P29).
- **With vignetting and uneven lighting on,** both detectors tell tissue from the empty slide:

  | Detector | Tissue | Empty slide |
  |---|---|---|
  | Colour | 100% | 0% |
  | Texture | 44% | 0% |

  Both are against the default 25% threshold.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/background.rs` | `Luv`, `ColourStatistics`, `TextureStatistics` |
| `crates/microscope-things/src/hardware/background.rs` | The `BackgroundDetector` interface |
| `crates/microscope-things/src/background.rs` | `ColourChannelDetector` and `ChannelDeviationDetector` |
| `crates/microscope-things/src/camera.rs` | `background_detectors`, a `SlotMap` of the server's detectors; `background_detector_name`; the actions; `calibration_required`; the two panels; `SampleCheck` |
| `crates/microscope-things/src/ui.rs` | `OwnEntry`: a Thing's own registry entry, kept weakly, which builds its panels; used by the camera and the detectors |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | The two detectors |
| `crates/microscope-things/tests/background.rs` | The tests below |
| `web/src/views/dev/ComponentsView.vue` | The camera's calibration panels in the components gallery |
| `web/src/api/generated/` | Regenerated |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5000
npm --prefix web run dev
```

Open `http://localhost:5173/#/dev/components` and scroll to the camera's calibration actions under "Server-described UI".

- **Calibrate:** press **Full auto-calibrate**. A dialog shows the stream as the sample goes out and comes back, then "The camera is calibrated."
- **Check:** at `http://127.0.0.1:5000/docs`, run `camera/image_is_sample`. It says whether the view holds sample, and how much: about 50% on the default blobs, by the texture detector.
- **The empty slide:** run `camera/remove_sample`, then `image_is_sample` again: "only 0.0% sample". Run `camera/load_sample` to put the sample back.

## Design notes and deviations from the plan

- **Our own colour conversion:** the conversion is CIE 1976 L\*u\*v\* written from its definition (sRGB decoding, then XYZ, then L\*u\*v\* against D65), in true units.
  - **OpenFlexure** uses OpenCV's 8-bit LUV, scaled to 0–255.
  - **So the floor differs:** the minimum deviation is ours, 0.5 in each channel's own units, rather than OpenFlexure's 0.5, 0.3 and 0.5 in OpenCV's.
  - **Checked against the definition:** the tests compare sRGB's primaries with their published L\*u\*v\* values, and mid-grey with L\* 50.
- **A blank image** is any whose channel's deviation is below 10⁻⁹, not exactly zero, since rounding leaves a trace on a uniform image.
- **The texture detector reads smooth tissue as background:** it found 44% of the tissue's cells to hold sample at 40×. The rest is smooth stroma. That still classifies tissue as sample, but less decisively than the colour detector. It's the default, as in OpenFlexure, because colour alone mistakes a coloured background for sample.
- **Analysis frames:** they're drawn wholly after the request (the second of two frames), then shrunk to 320 × 240, like the low-resolution stream.
- **The detector setting** is a functional setting:
  - **Reading it** gives the detector actually in use, so it reads `bg_channel_deviations_luv` before it's ever set.
  - **An unknown name** is refused with HTTP 422.
- **`full_auto_calibrate` always puts the sample back,** as the spec says, so a sample that was out before ends up in. OpenFlexure does the same.
- **The calibration actions are panels** (endpoints), not properties as in OpenFlexure, since our panel model has no WoT schema (ADR-0024).
- **`OwnEntry`** replaces the camera's own weak registry entry from P25, so every Thing with a panel keeps its entry the same way.
- **Checked in a browser,** against my own server: Full auto-calibrate from the gallery's panel, after which `calibration_required` was false and `image_is_sample` gave "53.1% sample" on the blobs.
- **Size:** 879 changed lines of hand-written code (about 280 of them tests) against L's budget of 900, a little above my estimate of 750–850.

## Tests

- **`background` unit tests in `microscope-core`** (4):
  - **conversion:** sRGB's primaries, white, black and mid-grey come out as their published values;
  - **colour coverage:** a quarter stained pink is 25%, and a darker slide, changed only in L\*, is still 0%;
  - **texture coverage:**
    - stripes in half the cells are 50%;
    - a smooth shading is 0%;
    - texture in v\* alone counts;
  - **blank images** can't be a background.
- **`tests/background.rs`** (3 tests), against the camera with both detectors:
  - **with vignetting and uneven lighting on,** for each detector:
    - calibration is required, `full_auto_calibrate` succeeds and leaves the sample in, and then it isn't required;
    - tissue is sample;
    - the empty slide is "only …% sample";
  - **the camera's defaults and errors:**
    - the detector defaults to `bg_channel_deviations_luv`, and an unknown name gets HTTP 422;
    - `image_is_sample` before a background asks for calibration;
    - with the light off, the blank frame can't be the background;
  - **the panels:** the primary and secondary calibration actions, and each detector's settings, name only what the Things have.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in the core:**
  - sRGB decoding;
  - the colour test ignoring L\*;
  - texture in any channel;
  - the deviation floor;
  - blank images;
- **in the camera and detectors:**
  - the default detector;
  - refusing unknown names;
  - `calibration_required`;
  - taking the sample out to calibrate, and putting it back;
  - the sample threshold;
  - the colour detector's tolerance;
  - the blank image's message;
  - the panel's actions.

Two survived at first: sRGB decoding, and texture counting in any one channel. The tests now also check mid-grey's L\*, and texture in v\* alone.

## Follow-ups and known gaps

- **P28's Settings tab** shows `manual_camera_settings`, the calibration panels and the active detector's `settings_ui`, through `ServerPanel`.
- **P29's calibration wizard** asks `calibration_required` and runs the primary actions.
- **Scanning** (P34 onwards) uses `image_is_sample` to skip the empty slide.
- **No lens-shading correction yet:** the background only classifies. Correcting vignetting and uneven lighting in the image is a later addition, if scans need flat fields.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **The spec:** it was written in this project's own words from what OpenFlexure's detectors and simulated calibration do.
  - **Our own:** the colour conversion, the statistics, the Things and the panels, written from scratch.
