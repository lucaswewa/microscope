# P36: Smart z-stacks

- Status: Done
- Pull request: [#49](https://github.com/lucaswewa/microscope/pull/49)
- ADRs: none new
- Spec: [phases.md#p36](../phases.md#p36)

## Summary

Each scan site now gets a z-stack, and keeps the sharpest images.

- **`autofocus.run_smart_stack`** climbs through focus a set distance at a time, judging its latest images by sharpness. When the peak is out of reach, it autofocuses and tries again. It saves the sharpest image, and some either side if asked.
- **`autofocus.run_basic_stack`** takes a set number of images about where z is, and saves them all. P37's C-Chip workflow will use it.
- **The snake and raster workflows** take a smart stack at each site, in place of P35's autofocus and capture. The stack settings are in their panel.

## Behaviour spec

Written before the code, in our own words, from what OpenFlexure's smart and basic stacks do. Marked notes show where testing changed it.

### What a stack is

At a site, a stack is a series of images taken a set distance apart in z. The sharpest image is kept, and, if asked, some either side of it.

### Its settings

- **The images to save:** 1 by default, from 1 to 9.
- **The images to test:** 9 by default, from 3 to 9. This is how many of the latest images are judged together: the **window**.
- **dz:** the distance between images, 50 steps by default, from 10 to 400.
- **Both counts must be odd,** and no more images can be saved than are tested.
  - **An even count** is raised by one.
  - **Too many to save** become the number tested.
  - Either change is logged, and the scan's settings record what was used.
- **Fixed:**
  - the stack starts 5 × dz below where its window would be centred;
  - it settles 0.05 s after each move;
  - it takes at most 15 images beyond the window before giving up;
  - it makes up to 3 attempts.

### The smart stack, `autofocus.run_smart_stack`

1. **It starts below,** by 5 × dz plus half the window's height, (images to test − 1) × dz / 2. Starting too low only costs images; starting too high means autofocusing and starting again.
2. **It takes an image,** after settling:
   - a capture kept in memory;
   - its sharpness, the JPEG size of the next preview frame drawn wholly after the capture (ADR-0022);
   - the stage's z.
3. **It judges the window** once it has enough images:
   - **Too little rise:** if the sharpest image is less than 3% sharper than the least sharp, the window is noise: continue. *Added after testing (see the design notes).*
   - **Three images:** the middle one sharpest is success; the first is "restart"; the last is "continue".
   - **Sharpness never falling** through the window: continue, since the peak is above.
   - **Sharpness never rising:** restart, since the peak is below.
   - **A parabola fitted to the window's sharpnesses:**
     - **not a peak,** not curving downwards: continue. *Testing changed this rule: it first asked for a sure peak, with the x² coefficient below zero by at least two standard errors (see the design notes).*
     - **the fitted peak or the sharpest image in the window's first two places:** restart;
     - **in its last two places:** continue;
     - **more than one turning point** in the window's sharpnesses: continue. Only steps more than half the mean step count as turning points, so noise doesn't.
   - **Otherwise:** success, at the sharpest image.
4. **On success,** it takes more images upwards until there are enough above the sharpest for the images to save. Then it stops.
5. **Continue:** it moves up dz and takes another image, unless it has taken 15 images beyond the window, which fails the attempt.
6. **Restart, or too many images:** the attempt has failed.
   - **Another attempt:** if fewer than 3 attempts have been made, the stage goes back to the attempt's first z and autofocuses, repeating until it's in focus, then starts again at step 1.
   - **No focus:** if autofocus finds none, there are no more attempts.
7. **Only the images it might save** are kept in memory: the window, and as many again below it as could be saved either side of the sharpest.
8. **Saving:** on success, or on failure if asked to save on failure, it saves the images to save, centred on the sharpest (fewer, if the stack is too short), to the folder it was given.
   - **Names:** `img_<x>_<y>_<z>.jpeg`, each with its sidecar (ADR-0025).
   - **The result:** whether it succeeded, the sharpest image's z, the images saved, and how many attempts it made. *Testing added the attempts.*

### The basic stack, `autofocus.run_basic_stack`

1. **Its images:** the images to save, dz apart.
2. **Its origin:**
   - **start:** from where z is, upwards;
   - **centre:** centred on where z is;
   - **end:** ending where z is.
3. **It saves every image,** with no judging, and returns their z positions.
4. **The C-Chip workflow** (P37) takes one at each site.

### In the workflows

- **The snake and raster workflows** gain the stack settings, `stack_images_to_save`, `stack_min_images_to_test` and `stack_dz`, in their panel.
- **At each site** they take a smart stack, in place of P35's single autofocus and capture.
  - **They save on failure,** so every site has an image.
  - **The planner is told** where focus was found, only if the stack succeeded.

### Where it differs from OpenFlexure

- **Sharpness** is our preview frames' JPEG size (ADR-0022), measured after each capture.
- **The stack's inputs and result** are our own types. The scan engine (P35) passes the folder to save in.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/stack.rs` | The pure part: `Counts` (the counts made odd and consistent, and which images to save), `sharpest`, and `judge`, which judges a window by the spec's rules, with a least-squares parabola |
| `crates/microscope-things/src/autofocus.rs` | `run_smart_stack` and `run_basic_stack`, with `StackSettings`, `StackOrigin` and `StackResult`. `looping_autofocus`'s sweeps are now a method that says when it found no focus, for the smart stack's retries. A failed attempt is logged |
| `crates/microscope-things/src/workflow.rs` | The grid workflows' stack settings, in their panel and in the scan's record, and a smart stack at each site. Its camera slot is gone, since the stack captures |
| `crates/microscope-things/src/smart_scan.rs` | It gives workflows their images folder in the data folder, `scans/<name>/images` |
| `crates/microscope-core/tests/stack.rs`, `crates/microscope-things/tests/stack.rs`, `tests/smart_scan.rs` | The tests |
| `web/src/api/generated/` | The new affordances' types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. **A smart stack:** at `http://127.0.0.1:5090/docs`, run `POST /api/v1/autofocus/run_smart_stack` with `{"folder": "stacks/try", "stack": {"images_to_save": 3, "capture_mode": "quick"}}`.
   - The output has `success` true, `sharpest_z`, and three images saved.
   - The data folder's `stacks/try/` has those three, 50 steps apart, each with its sidecar.
2. **Out of reach:** move the stage 1000 steps up, `POST /api/v1/stage/move_relative` with `{"z": 1000}`, and run the stack again. Its first attempt fails, so it autofocuses and tries again, and succeeds near z = 0, taking about 45 s. Its log says so.
3. **A basic stack:** `POST /api/v1/autofocus/run_basic_stack` with `{"folder": "stacks/basic", "stack": {"images_to_save": 5}, "origin": "end"}` returns five z positions, ending where z was.
4. **A scan:** calibrate the camera–stage mapping, `POST /api/v1/camera_stage_mapping/calibrate_xy`.
   - `GET /api/v1/snake_workflow/settings_ui` lists "Images to save per site", "Images to test" and "Stack step". The Slide Scan tab shows panels from P40.
   - `PUT /api/v1/snake_workflow/x_count` to 2, `y_count` to 1 and `stack_images_to_save` to 3. Then `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "try"}`.
   - `scans/try/images/` has three images at each site, named for their z, and `scan.json` has the stack settings it used.

## Design notes and deviations from the plan

- **Sharpness** is the JPEG size of the first preview frame taken after each capture (ADR-0022). That waits for a frame, about 0.1 s an image, so a site takes about 9.5 s on the simulator, with the default 9-image window and full-size captures. A real camera's capture may disturb its stream, so the frame comes after it.
- **The judging is pure,** in `microscope-core`. It's tested on focus curves measured on the simulator, on a dense sample and on the default one: preview frames' sizes every 50 steps through focus. Climbing either from far below, windows of 3, 5, 7 and 9 images first succeed at focus.
- **Testing on the default sample changed two rules of the spec.**
  - **The simulator's focus curve** is a cusp: on the default sample, about 7,700 bytes in focus, 6,600 one step away, and a flat 4,550 beyond 350 steps, varying by up to 1.2% from frame to frame.
  - **"Surely a peak" passed noise and failed real peaks.**
    - A parabola fits a cusp badly, so windows about focus were only 2 to 4 standard errors below zero, and a 5-image window centred on focus not even two. A stack testing 5 images never found focus on the default sample.
    - With 9 points, a fit has only 6 degrees of freedom, so about 1 noise window in 20 passed. One such window, 30 µm from focus, ended a stack in the first version of the tilted-sample test.
  - **Now a window must rise:** the sharpest at least 3% sharper than the least sharp. Noise far from focus rises by 1.2% at most, and a window about focus by 16% or more. So the parabola need only curve downwards; its vertex still places the peak for the edge rules.
  - **The dense sample** used at first hid both.
- **The folder** is a folder in the data folder, checked as other paths there are. So the actions are safe to call over HTTP, and `ScanWorkflow::acquire` now takes one too, a `String` rather than P35's path. ADR-0029 says so.
- **The counts** are made odd and consistent where they're used:
  - **The actions** log any change.
  - **A scan** logs it once, as it begins, and records the counts used; each site then takes them as they are.
- **Keeping images:** a stack holds its window and, below it, as many as could be saved either side of the sharpest. On success it takes more above the sharpest until there are enough to save.
- **Retries** go back to the attempt's first z and run `looping_autofocus` over `autofocus_dz`, a parameter of the action: 2000 steps by default, and the workflow's own setting in a scan. If it finds no focus, the stack stops trying. Other errors, such as the stage's, end the action.
- **Where the stage ends:** at the stack's last image. A scan moves on to its next site; a stack run by hand doesn't go back.
- **The workflows' settings** take OpenFlexure's names and ranges. The record has them as `stack_images_to_save`, `stack_min_images_to_test` and `stack_dz`.
- **On default settings,** on my own server:
  - **A 3 × 2 snake** found focus at every site, each image 0.55 µm from it, at about 9.5 s a site.
  - **A 4 × 3 snake on a sample tilted** 100 µm per mm in x and 50 in y, where focus moves about 120 steps between columns: 11 of 12 sites were within half a step of focus. At the other, the image was 6.4 µm below it. The follow-ups say why.
- **Size:** 967 changed lines of hand-written code, over M's 600 and above my estimate of 600–750. The project owner chose to keep one pull request at 918 lines. The 49 lines since are the fixes and tests from testing on the default sample.

  | Part | Lines |
  |---|---|
  | `microscope-core`'s `stack.rs`, with its tests | 312 |
  | `autofocus.rs` | 365 |
  | `workflow.rs` and `smart_scan.rs` | 85 |
  | The Things' tests | 205 |

## Tests

- **`microscope-core/tests/stack.rs`** (8 tests):
  - the counts: even counts raised, too many to save capped, both kept in range;
  - the images saved, centred on the sharpest, fewer at the stack's ends;
  - **each judging rule:**
    - three images;
    - climbing, levelling off, falling, and falling ever more slowly, throughout;
    - too little rise;
    - a peak away from the edges, by the fit and by the sharpest image;
    - a valley;
    - two peaks, and a wobble ignored;
  - climbing the two measured curves from far below, windows of 3 to 9 succeed first at focus;
  - every window of measured noise, of 3 to 9 images, continues.
- **Unit tests:**
  - in `stack.rs`: the parabola's peak, and which turns count;
  - in `autofocus.rs`: a stack's sharpness comes from a frame taken after it asks, not one taken before that arrives late.
- **`microscope-things/tests/stack.rs`** (3 tests), on the simulator's default sample:
  - **a tilted sample,** 100 µm per mm in x, at two sites. At each, nine images are saved about the sharpest, the middle one within half a step of the focal surface by its sidecar.
    - **At the first,** where z starts in focus, it succeeds in one attempt, and takes more images above the sharpest to have enough.
    - **At the second,** focus is 40 µm higher, beyond the first attempt's images, so it autofocuses and succeeds in a second attempt;
  - **with the light off,** no focus: nothing saved, then, asked to save on failure, one image. Autofocus finds no focus either, so it makes one attempt;
  - **basic stacks** from each origin, and a folder outside the data folder refused.
- **`tests/smart_scan.rs`:** scans now take a stack at each site, with a window of 5, set as 4 to test the rounding. Each site still saves one image, the planner is told its z, and the record has the stack settings.

**Mutation checks:** I broke 26 behaviours, and a test failed for 24.

- **Killed:**
  - **the counts:** even counts kept, too many saved, saving to one side;
  - **the rules:** the three-image rule, too little rise, any curve taken for a peak, both edge rules, turning points ignored, noise counted as turns, ties to the higher image;
  - **the stack:**
    - one attempt only;
    - saving on failure ignored, or always saving;
    - the basic stack's centre;
    - a folder outside the data folder;
  - **the scan's record:** counts as set, not as used.
- **Seven survived at first,** and stronger tests now catch them:
  - **the never-rising and never-falling rules:** windows falling ever more slowly, and levelling off;
  - **skipping autofocus on a retry, ignoring that it found no focus, or starting at the window rather than 5 steps below it:** the result now says how many attempts a stack made, and the test checks them;
  - **a frame taken before the stack asked:** a unit test of a late frame;
  - **no images taken above the sharpest:** the test saves nine of nine, so climbing always needs more above.
- **Two survive,** the known gap below: a scan site whose stack fails, telling the planner of focus anyway, or not saving.

## Follow-ups and known gaps

- **P37's C-Chip workflow** will take a basic stack at each site.
- **A narrow peak can fall between images.** On sparse fields of the default sample, the sharpness peak can be narrower than dz.
  - **At one site of the tilted scan,** the image 22 steps from focus measured no sharper than one 128 steps below it, so the stack kept that one.
  - **Autofocus** measures frames all along its moves, and found focus there within 15 steps.
  - **What would help:** a smaller `stack_dz`, more images saved, or a sharper measure than ADR-0022's JPEG size. The defaults follow OpenFlexure's.
- **The 3% rise** suits the simulator's noise. A real camera's should be measured in milestone 2.
- **Untested:**
  - a scan site where the stack fails: the workflow still saves its images, and doesn't tell the planner of focus;
  - the memory bound: holding only the images it might save.
- **Speed:** waiting for a preview frame after each capture is most of a stack's time on the simulator. It could measure the frame in parallel with the capture, if a real camera's stream allows.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
