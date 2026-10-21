# ADR-0043: The SDK camera frame pipeline

- Status: Accepted
- Date: 2026-10-21
- Phase: M2 P06

## Context

- **SDK cameras deliver frames by callback,** on the SDK's own thread, at the sensor's resolution, often several megapixels, and often faster than 10 a second.
- **Their pixels vary:** mono or colour; 8, 10, 12 or 16 bits; Bayer mosaics in four arrangements; RGB or BGR.
- **The rest of the microscope expects 8-bit RGB** (ADR-0018): `PreviewFrames` feeds the main and low-resolution MJPEG streams, which autofocus measures (ADR-0022), and `CameraApi` hands frames and captures to the other Things.
- **A capture's metadata** records how it was taken (ADR-0025).
- **Each camera model has its own features,** GenICam-style: exposure, gain, pixel format, white balance, binning, and many more, with its own ranges and lists. A `teta-wot` Thing's properties are fixed at compile time, and a panel's property control can offer choices, but not limits of its own (ADR-0024).

## Decision

- **Pixels** convert in `microscope_core::pixels`, as pure computation (ADR-0005):
  - mono is grey;
  - deeper pixels are shifted down to 8 bits, by the bits they use;
  - RGB is kept as it is, and BGR reordered;
  - a Bayer mosaic is demosaicked bilinearly: each pixel keeps its own channel, and takes each other channel's average from its neighbours with that channel.

  `PixelFormat::from_genicam` reads GenICam's names. Packed formats, such as `Mono12p`, aren't read: a driver asks for unpacked ones.
- **`FramePipeline`,** in `microscope-things::hardware`, takes each frame from the SDK's callback:
  - **the preview:** at most one every 100 ms is converted, shrunk to fit 1024 × 1024, and published through `PreviewFrames`, so the streams, `grab_frame` and autofocus work as with the simulator;
  - **captures:** `capture` waits for the next frame, up to 5 s, and keeps it whole, at the sensor's resolution, with the metadata the camera gives it. The frame's time and size fill in theirs;
  - **frames that are skipped aren't converted,** so a fast camera costs only the frames used;
  - **a frame that can't be converted** is an error, for the captures waiting too.
- **Settings:** every SDK camera's Thing has the same properties for its features: `exposure_time` (µs), `gain` (dB), `pixel_format`, `white_balance` and `binning`.
  - **`Feature`** says which of those features a camera has, and how: a number with its step, or a choice from the camera's own list;
  - **`settings_panel`** makes the panel from them, a control for each, in a fixed order. Each control reads its value back after writing, since a camera rounds or clamps what it's given;
  - **other features** aren't shown.
- **`FakeSdkCamera`,** in `microscope_things::fakes`, delivers frames by callback from a thread of its own, as an SDK does, for tests.

## Consequences

- **The camera drivers** (P07, P08) are mostly their SDK's calls: opening a camera, its features, and its frames into `FramePipeline::deliver`.
- **A capture is the next frame of the stream,** not a separate exposure: the same exposure and gain as the preview. A driver can add a triggered capture later.
- **A Bayer frame's conversion** runs in `microscope-core`, which isn't optimised in debug builds: slow for a full frame there, fine in a release build. Previews, which are shrunk, convert every 100 ms at most.
- **Limits come from the camera:** a value out of its range is clamped or refused there, and the panel shows what it took.
- **A camera's other features,** such as its frame rate, need a property added for each, in `STANDARD_FEATURES` and in every camera's Thing.

## Alternatives considered

- **The SDKs' own converters** (pylon's image format converter, Vimba X's image transform): more native code, called through `unsafe`, for each SDK, where one converter in safe Rust serves both.
- **Converting every frame:** at full resolution and full rate, too much work for frames that are mostly dropped.
- **A property per feature, made at run time:** `teta-wot` Things have fixed affordances.
- **A capture by a software trigger:** more of each SDK's API, and a pause in the preview. Left for the drivers, where they need it.
