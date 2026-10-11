# P22: Image registration core

- Status: In review
- Pull request: (added once opened)
- ADRs: [ADR-0023](../../adr/0023-image-registration-by-phase-correlation.md)
- Spec: [phases.md#p22](../phases.md#p22)

## Summary

`microscope-core` can now measure how far an image's content has moved between two frames, to a fraction of a pixel, and how sure it is. That's the primitive that camera–stage mapping (P23), stage measurement (P30) and correlation stitching will use.

- **`registration::register(a, b, options)`:** phase correlation. It shrinks, cuts a centred power-of-two square, windows, takes the normalised cross-power spectrum, then finds the peak, a sub-pixel refinement and a confidence.
- **`registration::Gray`:** greyscale images, made from camera frames' RGB bytes, and shrunk by blocks.
- **`fft`:** a radix-2 FFT, so the core still has no dependencies.
- **An example** (`microscope-sim`) that registers frames the simulator draws.

On rendered frames at 40×, the measured shift is within 0.2 px of the stage's move, in about 9.5 ms a pair in a release build.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/registration.rs` | `register`, `Gray`, `Options` and `Shift` |
| `crates/microscope-core/src/fft.rs` | `fft`, `fft2` and `Complex`, with tests against the definition |
| `crates/microscope-core/tests/registration.rs` | Synthetic shifts, noise, shrinking, periodic patterns and unrelated images |
| `crates/microscope-sim/examples/register.rs` | Rendered frames, registered against the stage's moves |
| `crates/microscope-sim/tests/registration.rs` | The same, as tests, and an empty slide |

## How to try it

```powershell
cargo run --release -p microscope-sim --example register
cargo test -p microscope-core
```

The example prints, for three stage moves, the shift expected from the move, the shift measured, the confidence and the time taken.

## Design notes and deviations from the plan

ADR-0023 records the method and its numbers. The details:

- **The sign:** `register(a, b)` says how far `b`'s content moved from `a`'s, so `b(x, y) ≈ a(x − dx, y − dy)`. In the simulator, the image moves against the stage: a 10 µm move in x moves the content 89.3 pixels left at 40×.
- **Sub-pixel accuracy:** I tried three refinements of the peak over 21 synthetic shifts:

  | Refinement | Mean error | Worst error |
  |---|---|---|
  | A parabola | 0.047 px | 0.088 px |
  | The neighbour's share (for unwindowed peaks) | 0.074 px | 0.124 px |
  | A Gaussian | 0.32 px | 0.52 px |

  The parabola is used; the Gaussian failed because the surface goes negative.
- **The periodic-pattern ambiguity, documented:** for stripes 16 pixels apart, a shift of 20 looks exactly like 4, or −12. Phase correlation finds the shift only modulo the period. The confidence says so: it falls below 0.2 because the next peak is as high. A test pins this down.
- **Shrinking:** shrinking by 2 (the default) costs little. At 4×, a fine texture loses detail, and the error grows to a fraction of a shrunk pixel (up to 2 camera pixels), which the test allows.
- **Range:** the shift must be less than half the compared square, and the images must overlap well. At the defaults, that's 128 shrunk pixels, or 256 camera pixels, about 29 µm at 40×.
- **The example lives in `microscope-sim`,** since `microscope-core` can't depend on the simulator (ADR-0005).
- **Size:** 645 changed lines of hand-written code (about 280 of them tests and the example) against M's budget of 600.

## Tests

- **`fft`** (2 unit tests): matches the transform's definition and inverts; a 2-D transform of a single point has unit magnitude, with that point's phase.
- **`tests/registration.rs` in `microscope-core`** (7 tests), on a smooth noise texture that can be drawn at any fraction of a pixel:
  - **whole shifts** to 60 px, within 0.1 px, with confidence above 0.5;
  - **fractional shifts,** within 0.12 px;
  - **noise** of ±20% of the signal: within 0.3 px, and confidence above 0.2;
  - **shrinking by 4:** within 2 px, half a shrunk pixel;
  - **periodic stripes:** the shift modulo the period, and confidence below 0.2;
  - **unrelated images:** confidence below 0.3;
  - **luminance and block shrinking.**
- **`tests/registration.rs` in `microscope-sim`** (2 tests): rendered frames 10 µm and 3.3 µm apart, within 0.5 px, with confidence above 0.5. An empty slide gets confidence below 0.3.

**Mutation checks:** I broke each of the following, and a test failed every time:

- the conjugate's side (the shift's sign);
- the normalisation;
- the window;
- the sub-pixel refinement;
- the second peak's exclusion zone;
- undoing the shrinking;
- wrapping negative shifts.

## Follow-ups and known gaps

- **P23:** camera–stage mapping moves the stage, registers the frames, and fits the matrix from pixels to steps. It sets a confidence threshold from these numbers.
- **Rotation and scale** aren't measured; mapping doesn't need them for small moves.
- **`rustfft`** could replace `fft` behind the same API if speed ever matters.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. OpenFlexure's camera–stage mapping uses a correlation library of its own; this is phase correlation written from the method's definition.
