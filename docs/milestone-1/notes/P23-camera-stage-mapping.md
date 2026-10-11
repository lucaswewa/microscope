# P23: Camera–stage mapping

- Status: In review
- Pull request: [#31](https://github.com/lucaswewa/microscope/pull/31)
- ADRs: none new; it uses [ADR-0023](../../adr/0023-image-registration-by-phase-correlation.md)'s registration
- Spec: [phases.md#p23](../phases.md#p23). P23 was split, at the owner's choice: double-click-to-move is now P23b (plan Appendix D).

## Behaviour spec

Written before the code, in this project's own words, from what OpenFlexure's camera–stage mapping does. It was updated while building, where measurements changed the design: how untrusted registrations are handled, and the step's size (see the design notes).

**The mapping.**
- The image-to-stage displacement matrix A, by rows `[[a, b], [c, d]]`, says how to move the field of view.
- Moving the stage by `(a·dx + b·dy, c·dx + d·dy)` steps moves the field of view by `(dx, dy)` pixels, right and down. So what was at `(dx, dy)` from the image's centre comes to the centre.
- **Interpreting a frame:** the image's content then moves by `(−dx, −dy)`, which is what registering the new frame against the old measures (P22).

**Calibrating** (`calibrate_xy`):
1. **Start:** note where the stage is, to return there. Grab a frame: its size is the image resolution. The target motion is a tenth of the frame's shorter side, in pixels.
2. **For each axis, x then y:**
   - **Sizing the step:** move along the axis by 1, 2, 4, … steps. After each move, grab a fresh frame and register it against the frame from before the first move, until the image has moved further than the target. The step is the total distance moved, scaled to what would move the image by the target.
   - **Can't size it:**
     - If a registration can't be trusted (confidence below 0.3), size the step from the last one that saw the image move at least a pixel. If there's none, fail ("the image has too little detail to track…").
     - If the image still hasn't moved further than the target after the move of 2¹⁴ steps, fail ("no motion was detected…").
   - **Measuring:** make four runs of five moves of that step size, alternating direction (+, −, +, −). After each move, register a fresh frame against the previous one: the move's field-of-view displacement.
   - **Untracked moves:** a move whose registration has confidence below 0.3 is untracked. It's left out of the fit and the backlash estimate. If fewer than five of the axis's twenty moves can be used, fail ("the image has too little detail to track…").
   - **Returning:** move back to the starting position, so both axes are measured around it.
3. **Backlash:** each run after an axis's first starts by reversing. Its first moves shift the image less than the rest, while the play is taken up.
   - **The typical move** is the run's median tracked displacement, along its direction.
   - **Taking up the play:** the run's leading moves, up to the first tracked one that goes at least 90% of the typical move, are left out of the fit.
   - **The shortfall** is the sum of how much those leading moves fall short of the typical one.
   - **The estimate** is the mean, over the runs tracked throughout, of shortfall ÷ typical × step size, in steps.
4. **Fitting:**
   - **A** is the matrix with each kept move's stage displacement ≈ A × its field-of-view displacement, by least squares over both axes' moves.
   - **The check:** if the RMS of the residuals exceeds 10% of the mean stage move, fail ("the stage didn't move as expected…").
5. **Saving:** once both axes are measured, save the data as JSON in the data folder's `calibration` folder, named by the time (UTC). Save it even when fitting fails, for diagnosis. The data:
   - every move: its stage and field-of-view displacements, its confidence, and whether the fit kept it;
   - the matrix, the backlash and the image size.
6. **Finishing:** move back to the start, whether calibrating succeeded or not. On success, store the matrix, the image resolution and the timestamp, and return what was saved.

**Using it:**
- **`move_in_image_coordinates(x, y)`** moves the stage by A·(x, y), rounded to steps, so the point `(x, y)` pixels from the centre comes to the centre.
- **`convert_image_to_stage_coordinates(x, y)`** gives A·(x, y) as whole steps.
- **`convert_stage_to_image_coordinates(x, y)`** gives A⁻¹·(x, y) as pixels.
- **Uncalibrated:** without a matrix, these three fail: "camera-stage mapping isn't calibrated yet".
- **`calibration_required`** says whether there's no matrix yet.
- **`can_calibrate`** is true.
- **`last_calibration`** is the saved data of the last successful calibration, or nothing.

**In the simulator:** the camera's `orientation` setting turns the camera relative to the stage by `rotation_deg`, and mirrors it left to right if `flipped`. That makes the matrix more than a scale.

## Summary

The microscope can now learn how stage moves move the image, and move by pixels.

- **The `camera_stage_mapping` Thing:**
  - `calibrate_xy` moves the stage along each axis and registers the frames (P22). It estimates the backlash, and fits the matrix from field-of-view pixels to stage steps;
  - `move_in_image_coordinates` brings a point of the image to its centre;
  - two actions convert displacements between pixels and steps.
- **The simulator's camera** can be turned and mirrored relative to the stage (`orientation`), so the matrix is a real rotation, not only a scale.

On the simulator, the matrix is within 0.3% of the truth for each orientation tried, and a move in image coordinates lands within 0.2 px. Calibrating takes about 20 s at the simulator's default speeds.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/mapping.rs` | `Matrix2`, `apply`, `invert`, `fit` (least squares, with the residual check), `run_backlash` |
| `crates/microscope-things/src/mapping.rs` | The `CameraStageMapping` Thing: settings, properties, the four actions, and saving the calibration |
| `crates/microscope-things/src/camera.rs` | `Orientation` and the camera's `orientation` setting |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.camera_stage_mapping:CameraStageMapping`, served as `camera_stage_mapping` |
| `crates/microscope-things/tests/mapping.rs` | Calibrating against the simulator's ground truth, backlash, sparse places, moving and converting |
| `web/src/api/generated/` | Regenerated: the new Thing and the camera setting |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

Then, in another terminal, also in the repository's root:

```powershell
$api = 'http://127.0.0.1:5090/api/v1'
function Invoke-Action($path, $body = '{}') {
    $run = Invoke-RestMethod -Method Post "$api/$path" -ContentType application/json -Body $body
    while ($run.status -in 'pending', 'running') { Start-Sleep -Milliseconds 100; $run = Invoke-RestMethod $run.href }
    $run
}
Invoke-Action camera_stage_mapping/move_in_image_coordinates '{"x": 100, "y": 0}'   # error: not calibrated yet
Invoke-RestMethod -Method Put "$api/camera/orientation" -ContentType application/json -Body '{"rotation_deg": 30, "flipped": false}'
$cal = Invoke-Action camera_stage_mapping/calibrate_xy                               # about 20 s
$cal.output.matrix | ForEach-Object { $_ -join ', ' }                                # about 0.97, 0.56 / -0.56, 0.97
Invoke-Action camera_stage_mapping/move_in_image_coordinates '{"x": 100, "y": 0}'   # output: the stage's new position
Get-ChildItem .microscope/data/calibration
```

With `http://127.0.0.1:5090/#/view` open beside it, the last move brings what was 100 px right of the centre to the centre.

The orientation and the calibration are settings, so they stay after a restart. Afterwards, put `{"rotation_deg": 0, "flipped": false}` back and calibrate again.

## Design notes and deviations from the plan

- **Untrusted registrations changed the spec.**
  - **The first spec** failed the calibration on any registration with confidence below 0.3. On the simulator's default sample, that failed often. The doubling first reached a step of 113 px, and about 4% of registrations of moves that size fell below 0.3 (16 of 360 pairs).
  - **Retrying doesn't help:** registering fresh frames of the same places scored as low again. The cause is sparse content, not noise.
  - **Two changes:**
    - **The step is scaled to the target,** rather than being the power-of-two total that first passed it: moves of 62 px, not 113 px. Then only 1% of registrations fall below 0.3 (4 of 360).
    - **An untrusted registration is no longer fatal:** an untracked move is left out, and an untrusted sizing registration falls back to the last trusted one.
  - **Measured:** I tried 49 start positions spread over 2.2 × 1.7 mm of the default sample. Every calibration succeeded, within 0.7% of the true matrix at the hardest places.
  - **Leaving out whole runs** with an untracked move, which I tried first, failed at 9 of those 49 positions. Every run passes the same places, so one sparse spot spoiled all four runs on an axis.
  - **An empty slide still fails:** its registrations score at most 0.23.
- **The per-axis minimum guards against a silent failure.** While I was trying the run-level rule, a sparse sample lost every y move. The fit still "succeeded" from x's moves alone, with a meaningless y column. Now each axis needs five moves it can use.
- **Backlash:**
  - **Only the run's leading moves** count. A stray short move later in a run isn't play being taken up.
  - **Not applied:** the estimate is returned and saved, but nothing uses it yet. `StageApi` has no backlash compensation.
- **Each axis is measured around the start:** the stage returns after each axis. It returns again at the end, whether calibrating succeeded or not.
- **The orientation** applies to the stage position the camera draws: it mirrors x if flipped, then turns by `rotation_deg`. The tests derive the true matrix from that and from the 40× scale.
- **Errors are plain ASCII:** Windows PowerShell 5.1 showed the en dash in the uncalibrated error as garbage, so the message says "camera-stage".
- **The saved data** has this project's own layout:
  - `timestamp`, `image_resolution`, `matrix` and `backlash_steps`;
  - `moves`, each with `stage`, `image`, `confidence` and `kept`.
- **Settings are read-only over HTTP:** only calibrating writes the matrix, resolution and timestamp.
- **Time:** most of the 20 s is spent waiting for fresh frames. Each registration grabs two frames at 10 fps, so that the second is drawn wholly after the move.
- **Size:** 850 changed lines of hand-written code (about 345 of them tests) against L's budget of 900.

## Tests

- **`mapping` in `microscope-core`** (2 unit tests):
  - **fitting:**
    - it recovers a known matrix exactly, and inverts it;
    - a stray pair is more error than the fit allows;
    - moves along one line can't be fitted;
    - a singular matrix can't be inverted;
  - **backlash:**
    - it estimates the leading moves' shortfall (80 steps);
    - there's no backlash when moves are even;
    - a stray short move later in a run isn't backlash;
    - an untracked move leaves the estimate unknown.
- **`tests/mapping.rs`** (7 tests, about 22 s), against the simulator:
  - **the matrix:**
    - it's within 1% of the truth for a square, a turned (35°), and a turned and mirrored (−120°) camera;
    - each move shifts the image by about the target;
  - **backlash:** it estimates 80 steps within 10, and leaves short moves out; the matrix is within 1%;
  - **a sparse place:** it leaves out an untrusted sizing registration and the untracked moves; the matrix is within 1%;
  - **moving and converting:**
    - before calibrating, the actions fail and there's no saved calibration;
    - after calibrating, `last_calibration` has all 40 moves, and the stage is back at its start;
    - converting there and back is within 1 px;
    - `move_in_image_coordinates(120, 60)` moves the content by (−120, −60), within 1 px;
  - **no detail:** an empty slide and a sample too sparse to track both fail with "too little detail", and the stage is back at its start.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in the Thing:**
  - the sizing fallback;
  - leaving out untracked moves;
  - leaving out backlash moves;
  - the field of view's sign;
  - each return to the start;
  - the per-axis minimum;
  - saving;
  - scaling the step;
  - the backlash estimate;
  - the move's axes;
  - the inverse;
  - the camera's mirroring;
- **in the core:**
  - the fit's error check;
  - an untracked move ending the leading moves;
  - an estimate despite untracked moves;
  - all short moves counting;
  - the 90% threshold;
  - the estimate's scale;
  - the singular check.

## Follow-ups and known gaps

- **P23b:** double-clicking the live image will move there, through `move_in_image_coordinates`.
- **Sparse samples:** on sparser samples (I tried 150–400 blobs per mm², against the default 1,500), some places can't be calibrated, and the error says so. Registration compares a centred 512-pixel square of the 820 × 616 frame. Comparing more of the frame would track sparser fields; P30 may need that.
- **The fit's error check** is tested only in the core. The simulator's stage moves exactly as asked, so the check never fails end to end.
- **Backlash compensation** could use the estimate, once a stage supports it.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **The spec:** it was written in this project's own words from what OpenFlexure's calibration does.
  - **Our own design:** the handling of untrusted registrations, the step scaling and the per-axis minimum are this project's, from the measurements above.
