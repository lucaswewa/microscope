# ADR-0017: Simulator architecture

- Status: Accepted
- Date: 2026-10-09
- Phase: P13

## Context

- Milestone 1 has no hardware. A simulated camera and stage stand in for it, and the frames must depend on where the stage is and how well it is focused. Then autofocus, camera–stage mapping, background detection and scanning run on the code paths the real drivers will use (plan §1).
- Frames must be quick enough for a live preview: an 820×616 frame in about 40 ms, so 10 frames a second or more.
- Tests and demos need frames they can reproduce exactly, and the realism must be switchable, so a test can turn one imperfection on at a time. The imperfections are blur, noise, and later tilt, vignetting and backlash.
- The Things that wrap the simulator come later (P16, P17), and ADR-0005 keeps `microscope-sim` free of `teta-wot`.

## Decision

- **`microscope-sim` is a pure Rust library** that depends on `microscope-core` and `image`, and not on `teta-wot`. It never reads the clock: time, frame numbers and seeds are its inputs.
- **Three frames of reference:**
  - **stage steps** per axis, with a per-axis µm scale (ADR-0019);
  - **the sample plane in µm,** x to the right and y downwards as the camera sees it, with the sample's centre at the origin;
  - **image pixels,** each covering the sensor's pixel pitch divided by the magnification.

  The stage's x and y in µm are the sample point at the centre of the field. Its z, against a focus z, gives the defocus.
- **A `Specimen` draws any region at any scale** (`render(origin, µm/px, width, height)`). It's deterministic for its seed, and a region drawn in parts matches it drawn whole. Details smaller than a pixel are drawn as their average, which is the level of detail for low magnification.
- **Specimens are procedural, not stored.** The blob specimen hashes the seed and a grid cell into that cell's blobs: how many (Poisson), where, how big and which colour. So any region of an unbounded or very large sample costs only what is visible. A sample is finite (a rectangle with empty slide around it) or repeating.
- **Bright-field light:** specimens absorb light, and overlapping absorbers multiply what they let through.
- **Optics as a short, physically shaped model:**
  - **Objectives:** 4×, 10×, 20×, 40×, 60× and 100× (oil), each with its numerical aperture.
  - **Depth of field:** λn/NA² + ne/(M·NA).
  - **Blur:** a Gaussian whose width combines the diffraction limit with a defocus cone of radius |dz|·tan θ, where sin θ = NA/n.

  The blur is three box blurs per direction, whose cost doesn't depend on its size. It's applied to a frame drawn with a margin of two blur widths, so edges blur like the middle, and capped at 64 pixels.
- **Sensor noise** comes from a sequence seeded by the frame's number, so a frame can be reproduced.
- **One function draws a frame:** `render_frame(&SimState)`, from the specimen, objective, sensor, stage position, focus, noise and frame number.
- **No random-number or image-processing crates beyond `image`.** SplitMix64 serves as both hash and sequence, and the blur is our own.

## Consequences

- Frames are reproducible from their inputs, so tests can assert exact pixels, and every later imperfection can be a switch in `SimState`.
- **The budget is met:** in a release build, an 820×616 frame takes about 19 ms in focus and at most 33 ms when heavily defocused.
- The model is plausible, not exact. Blur is Gaussian rather than an Airy pattern, absorption is per channel, and noise is additive. Autofocus and mapping only need the right trends: sharpness falls with defocus, and pixels scale with magnification.
- Tissue and image-backed specimens (P25) implement the same trait. The stage model (P14) and the Things (P16, P17) build on these frames.

## Alternatives considered

- **Pre-rendering a large sample image and cropping it.** It has a fixed resolution, so it can't serve 4× and 100× from one source. A slide-sized sample would be huge, and it couldn't be infinite or repeat.
- **A true optical simulation** (point-spread functions, wave optics). It's far slower than the preview budget allows, for no benefit to the workflows being tested.
- **`rand` and `imageproc`.** They'd be two more dependency trees for a hash, a sequence and a blur that take a few dozen lines.
- **Rendering on the GPU.** It's fast, but it adds a graphics stack and platform risk to a library that must run in tests and CI.
