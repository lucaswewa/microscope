# ADR-0022: Sharpness metric and fast autofocus

- Status: Accepted
- Date: 2026-10-10
- Phase: P21

## Context

- Autofocus must work on the simulator now and on milestone-2 hardware later, through `CameraApi` and `StageApi` (ADR-0018).
- OpenFlexure focuses quickly by sweeping the stage through focus while the preview streams, without stopping to grab frames. It measures each preview frame's sharpness by its JPEG size, since a sharper image has more detail and compresses less. Then it moves to where the sharpest frame was taken. Its users know this behaviour.
- The preview runs at about 10 frames a second (ADR-0020). The stage moves at a known speed but reports its position only when asked.
- A frame is published 50–100 ms after it was drawn, once encoded.
- Measured in the simulator at 40×, the small preview's JPEG size peaks sharply at focus. It falls by about 10% a micrometre out, and by 40% at 20 µm, the same both sides. The main stream's size peaks far less clearly, because its noise compresses badly whatever the focus.

## Decision

- **Sharpness is the JPEG size of the camera's low-resolution preview frames.** `CameraApi` gains `lores_stream()`, an additive change: the stream itself, whose frames carry their timestamps.
- **Frames are timestamped when they're taken:** when the camera reads the stage for the frame, before drawing and encoding. A real camera driver uses its sensor's timestamp.
- **A monitor watches the stream and the stage during a move:**
  - it records each frame's time and size;
  - it samples the stage's z every 20 ms;
  - a frame's z is interpolated between the samples either side of it;
  - after a move it waits for a frame taken after the move's end, so frames still being encoded aren't lost.
- **`fast_autofocus(dz, start)`**, as OpenFlexure's:
  1. move down dz/2 (with `start` "centre");
  2. sweep up dz while monitoring;
  3. take the z of the sharpest frame;
  4. move down dz;
  5. move up to that z.

  Steps 4 and 5 approach the focus from below, as the sweep did, so the stage's backlash lies the same way, and the carriage stops where the sharpest frame was seen.
- **`looping_autofocus`** repeats fast autofocus, centred on the last focus, until the focus falls in the middle three fifths of a sweep, up to 10 sweeps.
- **`z_move_and_measure_sharpness`** measures sharpness along a series of moves, for calibrating and diagnosing.
- **All three return a focus curve:** each frame's z and sharpness, the focus chosen, and the number of sweeps. They work through `CameraApi` and `StageApi` alone.

## Consequences

- **Precision:** in the simulator at 40×, with the default 2,000-step sweep, from ten offsets within ±40 µm, the stage ends a mean of 0.65 µm from focus, at worst 1.25 µm, with backlash on or off. That's within the objective's depth of field (about 1.5 µm).
  - **The limit** is the sampling: frames come about 50 steps (2.5 µm) apart at 500 steps a second and 10 frames a second.
  - **Better precision** needs a slower sweep or a faster preview.
- **Speed:** a 2,000-step sweep takes about 4 s at the simulator's z speed, and fast autofocus about 10 s in all. That's slow, but it never stops the preview or captures.
- **Thick or empty samples:** the metric needs detail. On an empty slide, or a sample thicker than the sweep, the sharpest frame is meaningless, so looping autofocus gives up after 10 sweeps, and fast autofocus moves to whatever was sharpest.
- **Timing matters:** stamping frames when they're drawn rather than when they're ready, and waiting for late frames, each remove a bias of up to about a frame's worth of z. The second was found by a flaky test.

## Alternatives considered

- **Stopping at each z to grab and score a frame,** for example by gradient energy. It's accurate, but each point costs a move, a settle and a frame, several times slower, and unlike OpenFlexure's.
- **Fitting a curve through the sweep's sharpness** to place the peak between frames. The curve is a sharp spike on broad shoulders, and noisy, so a fit can be pulled off the peak. OpenFlexure takes the sharpest frame, and the precision above is enough.
- **Recording the stage only at the start and end of each move,** as OpenFlexure does. That assumes a constant speed; sampling every 20 ms also suits stages that accelerate.
- **The main stream's JPEG size.** Its peak is much flatter, as measured above.
