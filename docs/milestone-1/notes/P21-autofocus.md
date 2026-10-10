# P21: Autofocus

- Status: In review
- Pull request: [#29](https://github.com/lucaswewa/microscope/pull/29)
- ADRs: [ADR-0022](../../adr/0022-sharpness-metric-and-fast-autofocus.md)
- Spec: [phases.md#p21](../phases.md#p21)

## Behaviour spec

Written before the code, in this project's own words, from what OpenFlexure's autofocus does.

**Sharpness.**
- A frame's sharpness is the size, in bytes, of the camera's small preview frame as a JPEG. More detail compresses less, so the sharpest frame is the largest.
- Each frame is timestamped when it was taken.

**Watching a move.**
- While the stage moves in z, record every preview frame's time and size, and sample the stage's z every 20 ms.
- A frame's z is the stage's z at its timestamp, interpolated between the samples either side of it.
- After a move ends, wait (at most a second) for a frame taken after the end, so that frames still being encoded are counted.
- The focus curve of a move is its frames, each with its z and sharpness, in order.

**Fast autofocus** (`fast_autofocus(dz = 2000, start = "centre")`):
1. With `start` "centre", move down `dz/2`, so the sweep is centred on where the stage was. With "base", start where it is.
2. Sweep up by `dz`, watching.
3. The focus is the z of the sharpest frame of the sweep, rounded to a step. If no frames arrived, fail: the camera isn't streaming.
4. Move down by `dz`, then up to the focus. Approaching from below, as the sweep did, the stage's backlash lies the same way, so the stage stops where the sharpest frame was seen.
5. Return the sweep's curve, the focus, and one sweep.

`dz` must be positive.

**Looping autofocus** (`looping_autofocus(dz = 2000, start = "centre")`):
- Run fast autofocus. If its focus lies in the middle three fifths of the sweep's z range (more than `dz/5` from each end), return its result, with the number of sweeps.
- Otherwise the focus may lie beyond the sweep: run it again, centred on the new focus. Give up after 10 sweeps.

**Measuring** (`z_move_and_measure_sharpness(dz: [steps…], wait = 0)`): make each relative z move in turn, waiting `wait` seconds between them, and return the curve of all of them, with no focus.

**Cancelling** any of them stops the stage where it is.

## Summary

The `autofocus` Thing, and an Autofocus button on the Control tab:

- **`fast_autofocus`, `looping_autofocus` and `z_move_and_measure_sharpness`**, as the spec above says. Each returns a focus curve.
- **Hardware:** they work through `CameraApi` and `StageApi` only, so they'll work with milestone-2 hardware. The camera interface gains `lores_stream()`.
- **The Control tab** gains an Autofocus button, which becomes "Cancel autofocus" while it runs, and the `a` key.

In the simulator, autofocus ends a mean of 0.65 µm from focus (at worst 1.25 µm) from ten starting points up to 40 µm away, with backlash on or off.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/autofocus.rs` | The monitor, the `Autofocus` Thing and its focus curve, with a unit test of the monitor |
| `crates/microscope-things/src/hardware/camera.rs`, `camera.rs`, `fakes.rs` | `CameraApi::lores_stream`, for the simulated camera and the fake |
| `crates/microscope-things/src/hardware/preview.rs`, `camera.rs` | Frames timestamped when they're taken, not when they're ready |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.autofocus:Autofocus`, as `autofocus` |
| `web/src/control/AutofocusControl.vue`, `web/src/views/ControlPage.vue` | The button and the `a` key; shown only when the server has an autofocus Thing |
| `web/src/api/generated/`, `web/src/api/things/index.ts` | The regenerated API types, and `autofocus` among the typed Things |
| `crates/microscope-things/tests/autofocus.rs`, `web/tests/` | The tests below; the Control tab's visual baselines updated for the button |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Open `http://127.0.0.1:5090/#/control`, hold PgUp for a couple of seconds to defocus, then press Autofocus or `a`. It takes about 10 s. `http://127.0.0.1:5090/docs` runs the actions with their parameters, and shows the focus curve.

## Design notes and deviations from the plan

ADR-0022 records the metric and the algorithm. The details:

- **Interface and frames:**
  - **`CameraApi::lores_stream()`** returns the small preview stream itself, whose frames carry their size and timestamp. It's an additive interface change (ADR-0018). The fake camera's never sends a frame.
  - **Frames are stamped when the camera reads the stage for them,** not after drawing and encoding. That removes a lag of 50–100 ms, up to 50 steps at the simulator's z speed.
- **Two timing bugs, found by tests:**
  - **Frames still being encoded:** the first version read the sweep's frames as soon as the move ended, and lost the last ones. It showed as a flaky test (4 frames instead of 8). Now the monitor waits for a frame taken after the move.
  - **Shared CPU:** the autofocus tests run one at a time, because each runs a camera and their frame rates fell when they shared the CPU.
- **Sampling:** the stage's z is sampled every 20 ms during the sweep, rather than only at the start and end as OpenFlexure does. So stages that accelerate are followed too.
- **Deviations from OpenFlexure:**
  - **No backlash flag on looping moves:** OpenFlexure's looping autofocus asks the stage for backlash-compensated z moves. `StageApi` has no such option, so the approach from below does the same job, as in fast autofocus.
  - **The focus curve** is this project's own shape (`points`, `focus_z`, `sweeps`), not OpenFlexure's raw arrays of times and positions.
  - **No alternative sharpness metric:** there's only the JPEG size. OpenFlexure can also use a Pi camera's focus figure of merit, which the simulator doesn't have.
- **The button:** OpenFlexure's Control tab runs `fast_autofocus` with `dz` 2000, and so does this one. The button is hidden when the server has no autofocus Thing.
- **Size:** 659 changed lines of hand-written code (about 240 of them tests) against M's budget of 600, within my estimate of 600–700.

## Tests

- **`tests/autofocus.rs`** (5 tests, against the simulated camera and stage, one at a time):
  - **Convergence:** fast autofocus converges to within 2 µm of focus from three offsets each, with backlash off and on, ending exactly at the focus it reports.
  - **Looping:** with focus beyond the first sweep, looping autofocus sweeps again and converges.
  - **Measuring:** measured moves down 200 and up 400 cover the range, and the sharpest frame is within 40 steps of focus.
  - **Cancelling and refusing:** cancelling looping autofocus stops the stage, and a `dz` of 0 is refused.
- **`autofocus.rs`'s unit test:** with the stage sampled at 0 and 100 steps 100 ms apart, frames taken at 25 ms and 90 ms get z 25 and 90. The second is counted though it arrives after the move. A frame taken after the move isn't.
- **`ControlPage.spec.ts`** (1 new): `a` starts `fast_autofocus` with `dz` 2000; while it runs, `a` doesn't start another, and the button cancels it; then the button starts it again.
- **End to end** (`control.spec.ts`, 1 new): from 15 µm out, the Autofocus button brings the stage back near focus.

**Mutation checks:**
- **Caught:** approaching from above, interpolation, taking the least sharp frame, never looping, and not waiting for late frames each made a test fail. Interpolation and the wait were caught only once the monitor's unit test was added.
- **Survives:** stamping frames when they're ready, rather than when they're taken. It costs about 0.75 µm in debug builds, inside the 2 µm tolerance.

## Follow-ups and known gaps

- **P23:** camera–stage mapping, and the scans (P35 onwards), will use autofocus through `useThing('autofocus')` and the Thing's actions.
- **Settling:** OpenFlexure's smart z-stacks (`run_smart_stack`) and `measure_settling_time` belong to later phases (P36).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The behaviour spec above is written in this project's own words, from reading OpenFlexure's source for behaviour only. The code is written from the spec.
