# P30: Stage measurement tools

- Status: In review
- Pull request: [#40](https://github.com/lucaswewa/microscope/pull/40)
- ADRs: none new
- Spec: [phases.md#p30](../phases.md#p30)

## Summary

The stage measurement tools are built.

- **`stage_measure` finds how far the stage travels** in x and y by watching the image. It steps towards each end until the image stops following, then finds the end exactly from how far the image moves on the way back. With autofocus, it also measures how far the focus shifts at each end.
- **It recentres** the stage on the middle of that range and makes it the zero.
- **Settings → Stage** shows the range, and has buttons for both.
- **Checkpoint B is checked:**
  - on a fresh settings folder, the wizard runs;
  - after it, autofocus, click-to-move and capture work;
  - every Settings panel is live.

## Behaviour spec

Written before the code, in our own words, from what OpenFlexure's stage measurement does.

### What it uses

`stage_measure` (`StageMeasure`) measures how far the stage travels in x and y by watching the image, and recentres the stage on that range. It uses:

- the camera;
- the stage;
- the camera–stage mapping, which must be calibrated;
- autofocus, when the server has it.

### The range-of-motion test, `perform_rom_test`

1. **It refuses to start** if the mapping isn't calibrated, or if the camera's frames aren't the size it was calibrated at.
2. **It notes where the stage starts,** after autofocusing there when it can.
3. **Each end of each axis is found in turn:** x then y, the positive end then the negative, each time starting from the start.
4. **The sizes it moves by:**
   - **A check step** is 15% of the frame's shorter side, turned into stage steps through the mapping. It's small so that the frames before and after it overlap well, which registration needs.
   - **A big step** is 10 check steps.
5. **Towards an end,** it repeats a big step, then a check step. It measures the check step by registering the frames from before and after it, and turns the image's motion into stage steps through the mapping.
   - **The stage is still moving** while the check step's measured motion along the axis is at least 65% of what was asked.
   - **A move the stage refuses,** as the simulator's does at the end of its travel, isn't an error in itself. The image decides.
6. **Once a check step falls short,** the stage has reached the end.
   - **Stepping back:** it steps back by check steps until the image moves by more than a tenth of a check step. That's at most 15 steps, else it's an error.
   - **The end of travel** is where the stage is after that step back, plus the motion the step made. That's exact both for a stage that stops counting at its end, as the simulator's does, and for one that counts on without moving.
7. **The focus at each end:** with autofocus, it autofocuses where it stepped back to, a check step from the end, and notes how far the focus moved in z from the start. That's the parasitic z motion.
8. **Then it goes back to the start.**
9. **It stops with an error when:**
   - **a registration can't be trusted,** even after autofocusing and making the move again once. A registration is trusted when its confidence is at least 0.3, or when it found just the move that was asked for, to within a fifth of a check step. The sample must have detail over the whole range;
   - **no end is reached within 500 big steps.**
10. **The result** is each axis's lowest and highest positions, with the focus's shift at each end when it was measured. It's returned, and kept as the read-only `calibrated_range` setting.

### Recentring, `perform_recentre`

1. **It refuses to start** if the range hasn't been measured.
2. **It moves x and y** to the middle of the measured range, with z where it is, and makes that the stage's zero.
3. **It shifts the kept range by the same amount,** so that it's centred on zero.

### Settings → Stage

- **Range of motion:** each axis's measured range, its ends, and the focus's shift when measured; or "Not measured yet."
- **"Measure the range of motion"** asks first, and shows its progress in a dialog with the stream.
- **"Recentre"** is there once the range is measured, and asks first.

### Where it differs from OpenFlexure

- **Recentring:**
  - **OpenFlexure's:** it fits the focus's z against position along each axis (a flexure stage moves in an arc) and moves to the top of that curve.
  - **Ours:** as the plan says, it moves to the middle of the measured range. The simulated stage doesn't move in an arc.
- **The steps:**
  - **OpenFlexure's:** it makes five medium moves to predict z, then big moves followed by three small checks, autofocusing after each move.
  - **Ours:** one check per big step, without predicting z. It autofocuses only at the start, at the ends, and when a registration can't be trusted.
- **The end:**
  - **OpenFlexure's:** its end is the position after the step back that moved, which is up to a small step short of the end.
  - **Ours:** it adds the motion that step made.
- **The data:** OpenFlexure also saves the test's data to the data folder; ours only returns it.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/stage_measure.rs` | `StageMeasure`: `perform_rom_test`, `perform_recentre`, the `calibrated_range` setting, and the `RangeOfMotion` and `AxisRange` types |
| `crates/microscope-things/src/hardware/stage.rs`, `stage.rs`, `fakes.rs` | `StageApi::set_zero`, for recentring, on the simulated stage and the fake |
| `crates/microscope-things/src/mapping.rs` | Its frame and registration helpers, shared within the crate |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.stage_measure:StageMeasure`, as `stage_measure` |
| `web/src/settings/StageMeasure.vue`, `StageSettings.vue` | "Range of motion" in Settings → Stage |
| `web/src/settings/CameraSettings.vue` | A P28 fix: the detector's name is followed through the shared stream, since the camera can't be asked to observe it |
| `web/src/api/things/index.ts` | `stage_measure` among the typed Things |

## How to try it

The simulated stage has no ends until its travel limits are on, so turn them on first. A quicker stage makes the test take seconds, not minutes.

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. Calibrate the camera–stage mapping, if it isn't already (Settings → Camera–stage mapping).
2. At `http://127.0.0.1:5090/docs`, run `PUT /api/v1/stage/simulation`. Use the current value, but with:
   - `"limits": true`;
   - `"speed": {"x": 20000, "y": 20000, "z": 500}`;
   - a travel of a few thousand steps, such as x from −4000 to 6000 and y from −5000 to 3000.
3. Open `http://127.0.0.1:5090/#/settings/stage` and press **Measure the range of motion**. After about a minute, the range shows: x 10,000 steps, from −4000 to 6000, and so on.
4. Press **Recentre**: the stage moves to the middle, which becomes (0, 0, 0), and the range is now centred on it.

## Design notes and deviations from the plan

- **Changes to the spec while building it:**
  - **The check step** shrank from 40% of the frame to 15%. Registration compares a centred square of the frame, and a 40% shift left too little overlap: it measured the shift right, but with a confidence of 0.08.
  - **A registration is also trusted when it found just the move asked for.** On the default sample of sparse blobs, a correct shift sometimes had a confidence of only 0.15. A wrong match landing exactly on the move asked for is very unlikely.
  - **The retry makes the move again,** with a fresh frame before it. Retrying the registration against a frame taken before refocusing failed, because that frame was the blurred one.
  - **The big-step limit** grew from 100 to 500, so the simulator's default ±10 mm fits.
  - **The frames must be the size the mapping was calibrated at.** Scaling the matrix to another size couldn't be tested well, so it's an error.
  - **The focus is measured** a check step from the end, where the stage stepped back to.
- **The end is exact:**
  - **OpenFlexure's:** it takes the position after the step back that moved, which can be up to a small step short.
  - **Ours:** it adds that step's measured motion. The tests find ends to within 3 steps.
- **Moves the stage refuses aren't errors:** the simulated stage ends a move at its travel limit with an error. Real flexure stages count on without moving. Either way, the image decides.
- **Recentring:** the middle of the range becomes (0, 0, 0), with `StageApi::set_zero`, new to the interface. The kept range is shifted to match, so recentring again doesn't move the stage.
- **The parasitic z motion** is how far the focus moved at each end, from the start. It comes from the stage's arc on a real stage, or from the sample's tilt and waviness in the simulator, and it's only as good as autofocus (within about 2 µm, or 40 z steps).
- **A P28 fix:** Settings → Camera observed the camera's `background_detector_name`, which the real camera can't be asked to observe, so the browser logged an error. It's now followed through the Thing's shared stream, which skips what isn't observable. The unit tests' fake camera could be observed, so they missed it.
- **Checked in a browser, against my own server on a fresh settings folder:**
  - **Checkpoint B:**
    - the wizard ran through;
    - autofocus ended in focus;
    - a double-click 120 pixels right moved x by 108 steps;
    - Capture saved a JPEG and its sidecar to the data folder's captures;
    - every Settings section loaded its panels, with no errors.
  - **The new tools:** with limits on (x −4000 to 6000, y −5000 to 3000) and the default sample, the test found exactly those ends, with focus shifts within 20 steps. Recentring centred them.
  - **Both themes:** the Stage section in each.
- **Size:** 630 changed lines of hand-written code (about 210 of them tests) against M's budget of 600: 5% over.

## Tests

- **`tests/stage_measure.rs`** (2 tests), against the simulator, with its limits on and a calibrated mapping:
  - **the range and recentring:**
    - recentring refuses to start before a test;
    - the test refuses frames of another size than the mapping's;
    - it finds x from −2500 to 3500 and y from −3000 to 2000, to within 3 steps;
    - it keeps the range, and goes back to the start;
    - recentring moves to the middle, (500, −500), and makes it (0, 0, 0);
    - the kept range is shifted;
    - x then ends 3000 steps away;
  - **the focus shift:** a test with the zero moved to x 2000 and the sample tilted 30 µm per mm in x, with autofocus. It finds the ends to within 5 steps, and the focus 180 z steps up and down at the ends of x, within 60, and unmoved at the ends of y. It takes about 75 seconds, mostly autofocus.
- **`stage_measure`'s unit test:** a registration is trusted when sure, or when it found the move asked for, on the axis and off it.
- **`SettingsPage.spec.ts`** (1 new test, 1 extended):
  - before a test, "It isn't measured yet." and no Recentre;
  - a measured range and its focus shifts are described;
  - Recentre asks, then runs.

**Mutation checks:** I broke each of 18 behaviours, and a test failed every time:

- **finding an end:**
  - the still-moving test reversed;
  - the end without the step's motion;
  - stepping back the wrong way;
  - not going back to the start;
- **recentring:**
  - the middle as the low end;
  - not zeroing;
  - the range not shifted;
- **the range:**
  - the range not kept;
  - any frame size allowed;
- **the focus:**
  - the focus shift not taken from the start;
  - no refocus and retry;
- **trust:**
  - trusted only when sure;
  - off-axis motion ignored;
- **the web app:**
  - Recentre before measuring;
  - the range's size;
  - the focus shift left out;
  - Recentre running the test;
  - not saying it isn't measured.

## Follow-ups and known gaps

- **Untested:** giving up after 500 big steps, which would take minutes in the simulator.
- **Saving the data:** the test's data isn't saved to the data folder, as OpenFlexure's is.
- **Recentring by the arc:** a real flexure stage could recentre on the top of its z arc, as OpenFlexure does, once there's hardware to try it on.
- **Another frame size:** the mapping and these tools assume the frames are the size the mapping was calibrated at.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its behaviour,** read from its source:
    - stepping towards an end and checking the image follows;
    - stepping back until it moves;
    - autofocusing on the way;
    - recentring.
  - **Our own:** the algorithm's details, the end's correction, the recentring by the range, the text and the code.
