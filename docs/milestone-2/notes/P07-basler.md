# P07: Basler cameras

- Status: In review, until the owner has tried it on the camera
- Pull request: [#80](https://github.com/lucaswewa/microscope/pull/80)
- ADRs: [ADR-0044](../../adr/0044-one-camera-thing-for-every-sdk.md)
- Spec: [phases.md#p07](../phases.md#p07)

## Summary

A Basler camera is the microscope's camera. `SdkCamera` provides `CameraApi` and what the app uses from a camera, over an SDK behind a trait. P07 adds pylon's half of that trait; P08 adds Vimba X's.

- **Grabbing:** pylon grabs continuously on a thread of its own, into P06's pipeline, for the streams, autofocus and full-resolution captures.
- **Settings:** the camera's exposure, gain, pixel format, white balance and binning are its own, read and written on it.
- **Calibration:** it runs the camera's automatic exposure and white balance once, then sets the background.
- **Disconnects:** the camera is opened again once it's back.

On this PC it runs against pylon's camera emulator; the owner's camera is next.

## What was built

| Where | What |
|---|---|
| `crates/microscope-sdk/src/pylon.rs` | pylon's camera API: `cameras` (serial number and model), `open`, `PylonCamera` (`has`, `get`, `set`, `execute`, `choices`, `grab`), `Grabbing`; pylon's own error messages |
| `crates/microscope-things/src/sdk_camera.rs` | `CameraSdk`, `Sdk`, `SdkCameraConfig`, `SdkDriver`, pylon's implementation, and the `SdkCamera` Thing |
| `crates/microscope-things/src/fakes.rs` | `FakeCameraSdk`: a mono camera's features, frames from `FakeSdkCamera` (now with a texture), and `unplug` |
| `crates/microscope-things/src/drivers/mod.rs` | `--list-devices` lists pylon's cameras by serial number and model |
| `crates/microscope-things/src/hardware/frames.rs` | `FramePipeline::last_delivered`, for the watchdog |
| `configs/basler-zaber.json` | A Basler camera with a Zaber stage |
| `crates/microscope-things/tests/sdk_camera_thing.rs` | `SdkCamera` against the fake, and on a Basler camera |

### `SdkCamera`

| Affordance | What |
|---|---|
| `mjpeg_stream`, `lores_mjpeg_stream` | The preview, through `FramePipeline` |
| `exposure_time`, `gain`, `pixel_format`, `white_balance`, `binning` | The camera's own, by GenICam name |
| `manual_camera_settings` | The panel, for the features the camera has, with the pixel formats that can be shown |
| `capture`, `capture_to_memory`, `save_from_memory`, `clear_buffers` | Captures, as with the simulated camera |
| `background_detector_name`, `calibration_required`, `set_background`, `image_is_sample` | Background detection, as with the simulated camera |
| `full_auto_calibrate`, `primary_calibration_actions`, `secondary_calibration_actions` | The camera's automatic exposure and white balance, then the background |
| `reconnect` | Closes the camera, opens it again, and grabs |

## How to try it

On a PC with pylon, here the owner's, with PowerShell's commands.

1. **Without a camera,** on pylon's emulator:

   ```powershell
   $env:PYLON_CAMEMU = '1'
   cargo run -p microscope-server -- --list-devices       # 0815-0000: Emulation
   $env:MICROSCOPE_BASLER_SERIAL = '0815-0000'
   cargo test -p microscope-sdk the_camera_emulator -- --nocapture
   cargo test -p microscope-things --test sdk_camera_thing on_a_basler -- --nocapture
   ```

2. **On the Basler camera,** plugged in, with `PYLON_CAMEMU` unset:

   ```powershell
   Remove-Item Env:PYLON_CAMEMU
   cargo run -p microscope-server -- --list-devices       # its serial number
   $env:MICROSCOPE_BASLER_SERIAL = '<its serial number>'
   cargo test -p microscope-things --test sdk_camera_thing on_a_basler -- --nocapture
   ```

3. **In the app:**
   - Copy `configs/basler-zaber.json`, set the camera's `serial_number` and the stage's `port`.
   - Run it in a folder of its own: `target\debug\microscope-launcher.exe -c my-basler.json --root $env:TEMP\microscope-basler`.
   - **The live image** is the camera's.
   - **Settings → Camera** has its exposure, gain, pixel format and binning, and white balance if it's a colour camera.
   - **Calibrate it,** looking at the empty slide.
   - **Capture**, then **autofocus**, on a slide.
   - **Unplug the camera** and plug it back in: the log says so, and the image comes back within a few seconds.
4. **Record** the camera's model, interface (USB 3 or GigE) and firmware in [hardware.md](../hardware.md).

## Hardware

- **pylon's camera emulator,** on the owner's Windows PC (pylon 26.09.2):
  - it lists as `0815-0000: Emulation`, opens by its serial number, and its features read and write;
  - it grabs 1024 × 1040 frames in `Mono8`, about six a second;
  - a capture is the whole frame;
  - through the server, its stream, a capture saved with its sidecar, and the settings panel all worked;
  - its automatic exposure never finishes, which is how calibration's 10-second limit was found.
- **Not yet tried:** the owner's Basler camera, Linux and the Pi.

## Design notes and deviations from the plan

- **One Thing for every SDK** (ADR-0044), as the owner chose, so P08 needs only Vimba X's calls. The configuration names the SDK: `{"class": "microscope.camera:SdkCamera", "kwargs": {"sdk": "pylon"}}`.
- **Features as text,** as pylon reads and writes them: one way for numbers and choices alike.
- **Continuous acquisition:** pylon's emulator starts in `SingleFrame`, and gave one frame, then none. `AcquisitionMode` is set to `Continuous` and `TriggerMode` to `Off`, where the camera has them.
- **A format that can't be shown** is replaced at start by the first the camera offers that can be. The panel offers only those.
- **Calibration doesn't wait for ever:** an automatic setting still at `Once` after 10 s is turned off, and the calibration goes on, logged.
- **Disconnects,** by a watchdog on the frames (5 s), not by pylon's removal callback: it serves every SDK, and any camera that stops sending.
- **The region of interest isn't a setting yet** (ADR-0044): a property control can't edit a rectangle, and mapping and stitching assume whole frames. Left for P10.
- **The fake's frames have a texture now:** the background detector rightly refuses an image of one level.
- **Size:** 1,917 changed lines (1,863 added, 54 removed), well over L's 900. I estimated about 1,300, and the owner chose one pull request; measured at 1,917, I asked again, and the owner kept one. pylon's bindings are 712 lines, each unsafe call with its `SAFETY:` comment, and `SdkCamera` is 732.

## Tests

- **pylon** (`microscope-sdk`):
  - where it's installed, it lists its cameras;
  - with `PYLON_CAMEMU=1`, the emulator opens by its serial number, its features read and write, a bad value is refused with pylon's message, its pixel formats list, and it grabs frames of its size;
  - a pixel's bytes by format.
- **`SdkCamera`, against the fake:**
  - it streams, and captures whole frames to memory and to the data folder, with a sidecar;
  - its settings are the camera's, and its panel shows only the formats that can be shown;
  - calibrating runs the auto exposure, then sets the background;
  - a camera in a format it can't show is switched to one it can;
  - a camera unplugged is opened again once its frames stop.
- **On a Basler camera,** with `MICROSCOPE_BASLER_SERIAL`: frames stream, the exposure written reads back, and a capture is whole. Passed on the emulator; not yet on the camera.

## Follow-ups and known gaps

- **The owner's run** on the Basler camera, on Windows, then Linux and the Pi, which makes the phase Done, and its model in hardware.md.
- **P08:** Vimba X's half of `CameraSdk`.
- **The region of interest,** in P10's tuning.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
