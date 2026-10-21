# ADR-0044: One camera Thing for every SDK, and Basler cameras through pylon

- Status: Accepted
- Date: 2026-10-21
- Phase: M2 P07
- Supersedes: [ADR-0018](0018-hardware-abstraction-through-teta-wot-interfaces.md), in part: a Thing type per camera driver

## Context

- **ADR-0018 planned a Thing type per driver:** `BaslerCamera`, `AlliedVisionCamera`.
- **A camera Thing has about 20 affordances** the app and the other Things use:
  - its two streams;
  - its captures;
  - its settings;
  - its background detection;
  - its calibration panels;
  - `CameraApi`.
- **For the SDK cameras, only the SDK's calls differ.** `teta-wot` Things can't be generic, so two Thing types would write those affordances twice.
- **pylon's C API** (P03b, ADR-0041):
  - reads and writes every feature as text, by its GenICam name;
  - grabs through a stream grabber, with buffers queued and a wait object to wait on;
  - lists its cameras with their serial numbers and models.
- **Its camera emulator** (`PYLON_CAMEMU=1`) gives a camera on a computer without one. It starts in `SingleFrame` mode, and its automatic exposure never finishes.

## Decision

- **One Thing, `SdkCamera`** (`microscope.camera:SdkCamera`), configured with its SDK (`"sdk": "pylon"`) and the camera's `serial_number`, or else the first camera found.
- **The SDK is behind a trait, `CameraSdk`:**
  - `has`, `features`, `get` and `set`, for features by name, their values as text;
  - `start` and `stop`, for grabbing, with frames delivered to a callback.

  `SdkDriver` holds it in a `teta-wot` `Device`. Vimba X (P08) adds its own implementation, and a variant of `Sdk`.
- **`SdkCamera`'s affordances** are what the app and the other Things use from the simulated camera:
  - **its streams:** `mjpeg_stream` and `lores_mjpeg_stream`, through `FramePipeline` (ADR-0043);
  - **its captures:** `capture`, `capture_to_memory`, `save_from_memory` and `clear_buffers`;
  - **its settings:** ADR-0043's standard properties, with `manual_camera_settings` made from the camera's features;
  - **background detection:** `background_detector_name`, `calibration_required`, `set_background` and `image_is_sample`;
  - **calibration:** `full_auto_calibrate`, and its two panels;
  - **`reconnect`**, and `CameraApi`.
- **Calibration** sets the camera's `ExposureAuto`, then its `BalanceWhiteAuto`, to `Once`, where it has them, and waits for each to return to `Off`. Then it sets the background. A setting still at `Once` after 10 s is turned off and logged, so the calibration goes on.
- **Pixel formats:**
  - a camera sending a format that can't be shown, such as a packed one, is switched at start to the first one it offers that can be;
  - the panel offers only formats that can be shown;
  - changing the format, or the binning, stops the grab, changes it, and starts grabbing again.
- **Disconnects:** a watchdog checks every second. If no frame has come for 5 s, it says so, and reopens the camera (`reconnect`), again every 5 s until it's back.
- **pylon:**
  - the device opens to control it and to stream from it;
  - a thread of its own grabs into four buffers, requeuing each once its frame is delivered;
  - before grabbing, `AcquisitionMode` is set to `Continuous` and `TriggerMode` to `Off`, where the camera has them;
  - padding at the ends of rows is taken out;
  - pylon's own error message comes with each failed call;
  - pylon stays initialised while a camera is open.
- **A feature's other name:** an older camera's exposure is `ExposureTimeAbs`, used where `ExposureTime` isn't there.
- **`--list-devices`** lists pylon's cameras by serial number and model.

## Consequences

- **P08 is Vimba X's calls,** with nothing more to write in the Thing.
- **A configuration names the SDK,** where it named a Thing type before. Two cameras of different makes are two `SdkCamera`s with different `sdk`.
- **The region of interest** isn't a setting yet. A property control can't edit a rectangle, and the stitching and mapping assume whole frames. It's left for P10's tuning.
- **Captures are the stream's next frame** (ADR-0043): the preview's exposure.
- **The unsafe code grows** by pylon's grabbing calls, each with its `SAFETY:` comment (ADR-0041). The grab result's struct is mirrored field for field from `PylonC.h`.

## Alternatives considered

- **A Thing type per SDK** (ADR-0018): each SDK's name in the configuration's class, at the cost of every affordance written twice.
- **A macro writing both Things:** one source, but two Thing types to test and an unusual macro to read.
- **`PylonDeviceGrabSingleFrame` for each frame:** simpler, but it starts and stops acquisition each time, which is slow for a live preview.
- **pylon's removal callback for disconnects:** a C callback from pylon's own thread. A watchdog on the frames covers every SDK, and a camera that stops sending for any reason.
