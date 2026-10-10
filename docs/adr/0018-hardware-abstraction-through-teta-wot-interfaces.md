# ADR-0018: Hardware abstraction through `teta-wot` interfaces

- Status: Accepted
- Date: 2026-10-09
- Phase: P15

## Context

- Milestone 1 runs on a simulated stage, camera and light. Milestone 2 brings real ones: OpenFlexure hardware (a Sangaboard, which drives stepper motors), industrial camera SDKs, and stages whose native unit is the micrometre (D10). Swapping one for the other must be a configuration change, with no change to the Things that use the hardware: autofocus, camera–stage mapping, scanning, the gallery.
- `teta-wot` connects Things with slots, filled when the server is built. A slot can name a `#[teta_wot::interface]` trait instead of a type, and any Thing that provides the trait can fill it. The configuration's `thing_slots` chooses which.
- `teta-wot` Things can't be generic: a Thing is one concrete type.
- Vendor SDKs often insist on being called from the thread that opened them. `teta-wot`'s `Device<D>` actor runs such a driver on a thread of its own and queues calls to it.

## Decision

- **Three interfaces** in `microscope-things::hardware`, holding what other Things need, not everything a device can do:
  - **`StageApi`:** `position`, `move_relative`, `move_absolute`, `stop` and `axis_scale`.
  - **`CameraApi`:**
    - `grab_frame`, the latest preview as 8-bit RGB, for analysis;
    - `grab_jpeg`;
    - `frame_size`, the size of the next frame;
    - `capture_to_memory`, a full-resolution image with its `CaptureMetadata`;
    - `stream_info`.
  - **`IlluminationApi`:** `set_led` and `led_on`.

  Async methods return boxed futures, as the macro requires. Answers known at once, such as the axis scale or the frame size, are plain methods.
- **Things that use hardware name the interface in their slots** (`Slot<dyn StageApi>`), never a driver's type.
  - **Required hardware** has a default name, such as `stage`.
  - **Optional hardware** connects by interface, with no default name (`OptSlot<dyn IlluminationApi>`). A named default must exist even for an optional slot.
- **One Thing type per driver,** such as `SimulatedStage`, `SangaboardStage` or `BaslerCamera`. Each lists its interfaces in `#[thing(interfaces(…))]` and implements them for `ThingRef<Self>`. It can call its own actions there, keeping their validation and the global lock.
- **Shared behaviour lives in plain structs** that each driver's Thing composes: capture buffers, the MJPEG stream, backlash compensation, the jog queue. That is how drivers share code without generic Things.
- **SDK drivers run behind `Device<D>` actors** (milestone 2).
- **Names mirror OpenFlexure's** (ADR-0007). The Things are `stage`, `camera` and `illumination`, and the actions are `move_relative`, `capture_to_memory` and so on.
- **Test fakes** (`microscope_things::fakes`) implement each interface, for testing the Things that use hardware. They are added to a test's registry, never to the server's.

## Walk-throughs: does each milestone-2 target fit?

**A Sangaboard stage** (OpenFlexure hardware: three stepper motors on a USB serial board)
- **The Thing:** `SangaboardStage` provides `StageApi`. Its serial port lives in a `Device<SangaboardSerial>`, so commands are queued on one thread.
- **The calls:** `move_relative` sends the board a relative move in steps and waits until it finishes. `position` asks the board, and `stop` interrupts the move.
- **Units:** positions are motor steps, the board's own unit (ADR-0019). `axis_scale` is unknown until a calibration or a configured lead screw gives µm per step; until then the UI shows steps.
- **Backlash:** compensation comes from the shared backlash struct that P16 writes for the simulator.
- **It fits.**

**A µm-native stage** (a commercial motorised stage, driven in µm with a fixed resolution such as 0.1 µm)
- **Units:** `MicronStage` defines one step as its resolution, so `axis_scale` is always known (`AxisScale::uniform(0.1)`), and positions are whole multiples of it.
- **The calls:** the controller moves in absolute coordinates, so `move_relative` is `move_absolute(position + displacement)`. `stop` uses its own halt command. The controller's library, if it needs a thread, goes in a `Device<D>`.
- **Mapping:** camera–stage mapping works in pixels and steps, so it doesn't care that a step is 0.1 µm.
- **It fits.**

**An industrial camera SDK** (Basler pylon, Teledyne FLIR Spinnaker or IDS peak, which open the camera on one thread and deliver frames by callback)
- **The Thing:** `BaslerCamera` provides `CameraApi`. Its SDK handle lives in a `Device<PylonCamera>`.
- **Frames:** the callback writes each frame into the shared capture-buffer struct. `grab_frame` takes the latest one and converts it to 8-bit RGB: a monochrome camera repeats its channel, and 12- or 16-bit data is scaled. `grab_jpeg` and the MJPEG stream come from the shared stream struct.
- **Captures:** `capture_to_memory` triggers a full-resolution frame. It records exposure and gain in `camera_settings`, and the stage's position if the camera has a stage slot.
- **It fits,** with one known limit: `CameraApi` hands other Things 8-bit RGB. Workflows that need the full bit depth would need a method added to the interface, which is an additive change.

## Consequences

- A configuration file chooses the drivers, and a test proves it: the same Thing, with `thing_slots` changed, uses a different camera.
- Things that use hardware are tested against fakes, quickly and without the simulator.
- Each new driver is a new Thing type plus the shared structs it composes. That is more types than a generic design would have, but the only kind `teta-wot` allows.
- **What other Things can rely on** is the interfaces, and changing them touches every driver. They're kept small, and grow only when a workflow needs more.

## Alternatives considered

- **A Thing generic over its driver** (`Stage<D: Driver>`). `teta-wot` Things can't be generic.
- **Slots by concrete type** (`Slot<SimulatedStage>`). Swapping in real hardware would mean changing the code of every Thing that uses it.
- **Talking to hardware over HTTP between Things,** through the WoT API. It's slower, it loses the in-process global lock and cancellation, and frames would have to be serialised.
- **One interface with everything a device can do.** Every driver would have to implement it all, and other Things would come to depend on driver-specific details.
