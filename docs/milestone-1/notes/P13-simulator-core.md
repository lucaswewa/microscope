# P13: Simulation world, optics and blob specimen

- Status: In review
- Pull request: #NN
- ADRs: [ADR-0017](../../adr/0017-simulator-architecture.md)
- Spec: [phases.md#p13](../phases.md#p13)

## Summary

The pure-Rust core of the image simulator, in `microscope-sim`. It draws what the camera sees for any stage position and focus:

- a procedural specimen of coloured blobs;
- objectives from 4× to 100×, which set the scale, the field of view and the depth of field;
- blur that grows with defocus;
- sensor noise.

Frames depend only on their inputs, so tests can reproduce them exactly. In a release build an 820×616 preview frame takes about 19 ms in focus and at most 33 ms defocused, within the 40 ms budget. An example writes PNGs for review.

## What was built

All in `crates/microscope-sim/`.

| Where | What |
|---|---|
| `src/units.rs` | `Point` in the sample plane (µm), `StageScale` (µm per step per axis, with conversions both ways), and `um_per_px` |
| `src/specimen.rs` | The `Specimen` trait (`render(origin, µm/px, width, height)`) and `Extent` (finite, or repeating) |
| `src/blobs.rs` | `BlobSpecimen` and `BlobConfig`: seed, density, radius range, colours, background and extent |
| `src/optics.rs` | `Objective` and `OBJECTIVES`, depth of field, defocus blur, and `Sensor` with `Sensor::PREVIEW` and the field of view |
| `src/filters.rs` | The fast Gaussian blur (three box blurs per direction) and sensor noise |
| `src/random.rs` | SplitMix64, as a hash and as a sequence |
| `src/render.rs` | `SimState` and `render_frame` |
| `examples/render.rs` | Writes review PNGs, and how long each frame took |
| `tests/sim.rs` | The tests below, and the benchmark |
| `Cargo.toml` (workspace) | `image` 0.25.10, PNG only |

## How to try it

```powershell
cargo run --release -p microscope-sim --example render -- target/sim-review
cargo test --release -p microscope-sim -- --ignored --nocapture   # the benchmark
```

`target/sim-review/` then holds, as 820×616 PNGs:

- each objective in focus (`objective-4x.png` … `objective-100x.png`);
- 40× from 0 to 40 µm out of focus;
- the right-hand edge of the 20 mm sample at 10×;
- a repeating sample;
- a frame with sensor noise.

## Design notes and deviations from the plan

ADR-0017 records the architecture. The details:

- **The scale:** `Sensor::PREVIEW` is 820 × 616 pixels, a 1.12 µm sensor binned 4 × 4 into 4.48 µm pixels.

  | Objective | NA | µm per pixel | Field of view (µm) | Depth of field (µm) |
  |---|---|---|---|---|
  | 4× | 0.10 | 1.12 | 918 × 690 | 66 |
  | 10× | 0.25 | 0.448 | 367 × 276 | 10.6 |
  | 20× | 0.40 | 0.224 | 184 × 138 | 4.0 |
  | 40× | 0.65 | 0.112 | 92 × 69 | 1.5 |
  | 60× | 0.80 | 0.075 | 61 × 46 | 0.95 |
  | 100× (oil) | 1.25 | 0.045 | 37 × 28 | 0.59 |

- **Blur:** the standard deviation combines the diffraction limit (0.21 λ/NA) with half the defocus cone's radius, |dz|·tan(asin(NA/n))/2. It's capped at 64 pixels, so beyond that the frame is a wash however far from focus it is. At 40×, 10 µm from focus gives about 19 pixels of blur.
- **The blobs:**
  - **Cells:** blobs live in cells four of the largest radius wide (32 µm by default), about 1.5 blobs per cell on average.
  - **Defaults:** 1,500 blobs per mm², 4–16 µm across, purple, pink and blue, on a 20 × 20 mm sample.
  - **Repeating samples:** the period is rounded to a whole number of cells; a requested 100 µm repeats every 96 µm, and `period_um()` reports it.
  - **Finite samples:** only blobs centred inside the rectangle are drawn, so the margin starts at the edge.
- **Level of detail:** a blob smaller than half a pixel darkens the pixel its centre falls in, by the share of the pixel it covers. A 4,000 µm overview at 20 µm per pixel draws each blob as a single darkened pixel.
- **Performance:**
  - **Measured** in a release build, 820×616 at 40×, averaging 20 frames: 18.9 ms in focus, 23.1 ms at 5 µm, 31.9 ms at 20 µm, and 33.2 ms at 40 µm. Every objective in focus takes about 20–22 ms.
  - **The margin:** the first version drew a margin of three blur widths, and heavily defocused frames took up to 47 ms. Two widths are enough, since edges repeat beyond them, which brought the worst case to 33 ms.
  - The budget isn't a CI gate.
- **The specimen trait returns 8-bit RGB,** and `render_frame` blurs and adds noise in floating point before rounding back. A specimen that wants more precision can come later.
- **The stage model** (P14) and the optical imperfections (tilt, vignetting, illumination: P25) aren't here. `SimState` takes the stage position in µm directly.
- **Dependencies:** `image` 0.25.10 (March 2026), with only its PNG codec. Its new dependencies are all at least ten days older than the two-week cut-off, and are MIT, Apache-2.0, BSD-3-Clause, Zlib or Unlicense.
- **Size:** 1,099 changed lines of hand-written code (318 of them tests, 100 the example) against L's budget of 900, above my estimate of about 900. Rustfmt writes struct literals one field per line, which accounts for part of it.

## Tests

11 tests, and a benchmark run on request (`tests/sim.rs`):

- **Determinism:** the same state draws the same frame, a different seed a different one, and a region drawn in two halves matches it drawn whole.
- **Scale and geometry:**
  - stripes 20 µm apart give the expected number of edges at 10× and 40×;
  - the preview's field at 40× is 91.84 × 68.99 µm;
  - a dot at the stage position is at the frame's centre, and moves by the right number of pixels in the right directions.
- **Blur:** sharpness falls steadily from 0 to 12 µm of defocus, is the same either side of focus, and moving the focus matches moving the sample. The depth of field shrinks with each objective.
- **Extent:** a finite sample is empty far away and right beyond its edge, but not inside it; a repeating sample repeats at its period.
- **Level of detail:** sub-pixel blobs still darken pixels.
- **Noise:** it's the same for the same frame and different for another, with a mean near 0 and a standard deviation within 10% of the setting.
- **Units:** steps and µm convert both ways, with rounding.

Mutation checks: each of these, removed, fails a test:

- the blur;
- the repeat;
- the sub-pixel dots;
- the check that keeps blobs inside a finite sample. The first version of the margin test missed this one: it looked only far away, where no cells are visited. It now also looks just beyond the edge.

## Follow-ups and known gaps

- P14 models the stage's motion, P16 and P17 wrap the simulator in Things, and P17 adds JPEG output.
- P25 adds tissue and image-backed specimens, a tilted and wavy focal surface, vignetting, uneven illumination, exposure and gain.
- If tissue specimens prove slower, the blur could run on a downsampled frame when it's large.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The simulator is written from the plan's description and textbook optics. OpenFlexure's simulation server was used only to see its behaviour. Nothing of its sprites or code is used.
