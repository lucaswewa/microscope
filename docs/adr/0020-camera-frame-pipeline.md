# ADR-0020: Camera frame pipeline

- Status: Accepted
- Date: 2026-10-09
- Phase: P17

## Context

- The camera feeds three kinds of consumer:
  - **people,** through the live preview in a browser;
  - **analysis** in other Things (autofocus, camera–stage mapping, background detection, scanning), which needs pixels;
  - **captures,** which need more detail than the preview and a record of how they were taken.
- OpenFlexure's camera serves two MJPEG streams, `mjpeg_stream` and a 320 × 240 `lores_mjpeg_stream`. It grabs frames from them without pausing, and captures separately (ADR-0007 mirrors its names).
- `teta-wot` provides `MjpegStream`. Frames are added from any thread; the stream keeps a ring of recent frames, and slow clients skip frames rather than holding the camera up.
- The simulator draws a frame from the stage's true position (ADR-0017). That takes tens of milliseconds, so it must not run on the async threads.
- The preview budget is about 10 frames a second (plan §5.7).
- Milestone-2 cameras deliver frames by SDK callback (ADR-0018), and should publish them the same way.

## Decision

- **One preview thread per camera,** started in `on_start` and stopped in `on_stop`. It's an OS thread, not an async task, because drawing a frame is CPU-bound.
  - **Pacing:** it draws a frame every 100 ms, or as often as drawing allows, then sleeps for the rest of the interval.
  - **Stopping:** a flag and an unpark make it stop within one frame. `on_stop` then ends both streams, so every client's response finishes.
- **Publishing goes through a shared struct,** `hardware::PreviewFrames`, which every camera driver composes. Each frame is:
  - encoded as a JPEG (quality 85) for `mjpeg_stream`;
  - shrunk to 320 × 240 for `lores_mjpeg_stream`;
  - kept as pixels, as the latest frame.

  The frame is stored before it's added to the stream. So whoever is woken by the new frame finds it.
- **Grabbing vs capturing:**
  - **Grabbing** returns the next frame, after the call, without disturbing the preview: `grab_jpeg` (a Blob), `grab_jpeg_size`, and `CameraApi::grab_frame`, which gives pixels. Calling it twice guarantees a frame drawn wholly after the call.
  - **Capturing** draws a separate frame at the capture size: 1640 × 1232, binned 2 × 2, the preview's field of view in twice the detail. It's drawn on a blocking thread and comes with `CaptureMetadata`.
- **Streaming modes:**
  - **`default`** streams 820 × 616 frames.
  - **`full_resolution`** streams at the capture size, more slowly.

  Changing the mode applies from the next frame. The stream isn't restarted, since MJPEG viewers accept a change of size.
- **The simulated frame:**
  - it's drawn from the stage's true position (`SimulatedStage::true_position_um`), the objective, the sample settings and the noise;
  - it's scaled by exposure (relative to 10 ms) times gain, saturating at 255;
  - it's black while an `illumination` Thing's LED is off.

  The camera reaches the stage through a slot of the concrete `SimulatedStage` type, not `StageApi`: only the simulator knows where the sample really is.
- **Debug builds optimise the hot crates:** `microscope-sim`, `image` and `zune-jpeg` (Cargo's `profile.dev.package`). The shrink goes through `DynamicImage`, whose non-generic methods are compiled, and optimised, inside `image`. A development server then previews at nearly the full rate, and the code under development still builds for debugging.

## Consequences

- **Measured on the development machine** (an 820 × 616 frame at 40×):

  | | Draw | Encode and shrink | Rate |
  |---|---|---|---|
  | Release, in focus | 21 ms | 11 ms | ~10 fps (the cap) |
  | Release, 20 µm defocused | 36 ms | 11 ms | ~9 fps |
  | Release, `full_resolution` | 86 ms | 34 ms | ~4 fps |
  | Debug, in focus | 30 ms | 61 ms | ~10 fps |
  | Debug, defocused | 57 ms | 62 ms | ~8 fps |

  Before the profile change, debug builds took 680–1,300 ms a frame.
- **What analysis Things can rely on:** `CameraApi::grab_frame` gives pixels without decoding a JPEG, and a frame drawn after a move is only one wait away. P17b adds `settle` and `discard_frames` for that.
- **A real camera** (milestone 2) replaces only the drawing. Its SDK callback hands frames to the same `PreviewFrames`, and its captures fill the same metadata.
- **Every frame is drawn, encoded twice and kept,** whether or not anyone watches. That's simple and keeps grabs fast, at the cost of steady CPU use. A later phase could pause drawing when nobody watches or grabs.

## Alternatives considered

- **Drawing on demand, per client.** Each viewer would cost a frame's drawing, and grabs would race with viewers. One shared producer is how real cameras work too.
- **An async task for the preview loop.** Drawing would block a runtime worker for tens of milliseconds at a time. `spawn_blocking` per frame works, but a long-lived thread is simpler, and it's what an SDK's frame callback looks like.
- **Shrinking the lores stream by drawing it separately.** It's cheap, but it isn't the same frame, and its noise would differ from the main stream's.
- **Keeping only JPEGs.** Analysis would decode every frame, costing time and precision.
- **Optimising the whole workspace in debug builds.** The Things and the server would then be harder to step through in a debugger, for little gain: their own code isn't the hot path.
