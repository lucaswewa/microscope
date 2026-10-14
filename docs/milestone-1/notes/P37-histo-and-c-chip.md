# P37: Histo smart-spiral and C-Chip workflows

- Status: Done
- Pull request: [#50](https://github.com/lucaswewa/microscope/pull/50)
- ADRs: none new
- Spec: [phases.md#p37](../phases.md#p37)

## Summary

The last two scan workflows, so all four of plan D3 are in.

- **`histo_scan_workflow`** scans a tissue section. It spirals out from the start (P34), skips sites that look like background, and takes a smart stack (P36) where there's sample. It has its own background detector, with Set background and Check background, and is now the default workflow.
- **`c_chip_workflow`** scans a C-Chip counting chamber's well: the fixed 5 × 7 pattern, with a basic stack at each site, all on the start's focal plane.

## Behaviour spec

Written before the code, in our own words, from what OpenFlexure's histo and C-Chip workflows do.

### The histo workflow, `histo_scan_workflow`

It scans a tissue section: a smart spiral (P34) out from the start, imaging only where there's sample, with a smart stack (P36) at each site.

1. **Its name:** "Histo scan".
2. **What it uses:**
   - the camera, the stage, autofocus and the camera–stage mapping;
   - **a background detector,** in a slot of its own: `bg_channel_deviations_luv` unless configured, as the camera's is.
3. **Its settings:**
   - **the overlap:** 0.35, from 0.1 to 0.7;
   - **the maximum range:** 25,000 steps, from 1,000. No site is further than this from the start, in a straight line;
   - **equal distances:** off. When on, the steps in x and y are both the smaller of the two;
   - **skip background:** on. Sites that look like background get no stack and no image;
   - **the autofocus range:** 1,000 steps, from 400 to 3,000;
   - **the capture mode;**
   - **the stack settings:** as the snake and raster workflows have them (P36).
4. **Ready** once the camera–stage mapping is calibrated and, while it skips background, its detector has a background.
5. **`set_background`:** sets the detector's background from a frame of what the camera sees now, which should be the empty slide.
6. **`check_background`:** says whether what the camera sees now is sample, and how much of it: "62.5% sample", or "only 3.1% sample".
   - **The frame** for both is drawn wholly after the action starts, at the preview's low resolution, as the camera's own background actions take it (P27).
7. **Its panel,** `settings_ui`, has two sections:
   - **Background detect:** how to use it, then Set background and Check background;
   - **Scan settings:** the settings above.
8. **Starting a scan:**
   - **It refuses** without a calibrated mapping, or, while it skips background, without a background or when the start looks like background: "move to the sample first".
   - **The plan:** the grid's step from the overlap through the mapping, made equal if asked, and a smart spiral from the start within the maximum range.
   - **It autofocuses,** repeating until it's in focus, as the grid workflows do.
   - **The scan's record** has its settings, with the step and the stack counts it used.
9. **At each site:**
   - **Background:** while it skips background, a frame drawn after the stage arrived is checked. If it's background, nothing is taken, and the planner is told no image was taken, so the spiral doesn't grow from there.
   - **Otherwise:** a smart stack, saving even without focus, as the grid workflows take it. The planner is told of focus only if the stack found it.
10. **The scan ends** when the spiral does: when every site around the sample is background, or when the next sites would be beyond the maximum range.

### The C-Chip workflow, `c_chip_workflow`

It scans a C-Chip counting chamber's well: a fixed grid, with a basic stack (P36) at each site and no autofocus between.

1. **Its name:** "C-Chip scan".
2. **What it uses:** the stage, autofocus and the camera–stage mapping.
3. **Its settings:**
   - **the autofocus range:** 1,000 steps, from 400 to 3,000;
   - **the capture mode;**
   - **the images at each site:** 5, from 1 to 9, odd as stacks need;
   - **dz:** 50 steps, from 10 to 400.
4. **Ready** once the camera–stage mapping is calibrated.
5. **Its panel** has the settings.
6. **Starting a scan:**
   - **It refuses** without a calibrated mapping.
   - **The plan:** the C-Chip pattern from the start, 5 columns by 7 rows at an overlap of 0.1 (P34).
   - **It autofocuses** at the start, repeating until it's in focus.
   - **The scan's record** has its settings, with the step it used.
7. **At each site:** a basic stack centred on where z is, saving every image. The planner is told the stack's centre as the site's focus, so every site's stack is centred on the start's focus.

### Both

- **`histo_scan_workflow` becomes the default workflow,** in `smart_scan`'s `workflow_name`.
- **They're Things,** `microscope.workflows:HistoScanWorkflow` and `microscope.workflows:CChipWorkflow`, in the simulation configuration.

### Where it differs from OpenFlexure

- **The background detector** is the workflow's, chosen by its slot. The camera chooses its own, for its calibration.
- **A start that looks like background** is refused while skipping background, rather than ending the scan at its first site.
- **The C-Chip workflow** keeps one focal plane: each stack is centred on the start's focus.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/histo.rs` | `HistoScanWorkflow`: its settings, `ready`, `set_background`, `check_background`, its two-section panel, and its `ScanWorkflow` side |
| `crates/microscope-things/src/c_chip.rs` | `CChipWorkflow`: its settings, `ready`, its panel, and its `ScanWorkflow` side |
| `crates/microscope-things/src/workflow.rs` | What the workflows share: the grid's step from the mapping, a smart stack at a site, and the stack settings' panel entries |
| `crates/microscope-things/src/camera.rs` | `SampleCheck::of`, how a detector judges a frame, for the camera's `image_is_sample` and the histo workflow's `check_background` |
| `crates/microscope-things/src/smart_scan.rs` | `workflow_name` is `histo_scan_workflow` by default |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.workflows:HistoScanWorkflow` and `microscope.workflows:CChipWorkflow`, as `histo_scan_workflow` and `c_chip_workflow` |
| `web/src/server-ui/ActionButton.vue` | A button that says its action's output now says the output's `message`, if it has one, rather than its JSON |
| `crates/microscope-things/tests/workflows.rs`, `tests/smart_scan.rs`, `web/tests/unit/server-ui/ServerInterface.spec.ts` | The tests |
| `web/src/api/generated/` | The new Things' types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

At `http://127.0.0.1:5090/docs`:

1. **The tissue sample:** `PUT /api/v1/camera/sample_type` to `"tissue"`.
2. **Calibrate:** `POST /api/v1/camera_stage_mapping/calibrate_xy`, then `POST /api/v1/camera/full_auto_calibrate`. The camera's background detector is the histo workflow's too, so `GET /api/v1/histo_scan_workflow/ready` is now `true`.
3. **Check:** `POST /api/v1/histo_scan_workflow/check_background` says how much sample the camera sees, such as "60.9% sample".
4. **A histo scan:** `PUT /api/v1/histo_scan_workflow/max_range` to 1500, to keep it short, then `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "tissue"}`.
   - It takes about 5 minutes.
   - `scans/tissue/scan.log` lists the sites. Those that looked like background have no image.
   - The log ends "It reached its maximum range".
5. **A C-Chip scan:** `PUT /api/v1/smart_scan/workflow_name` to `"c_chip_workflow"`, then `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "chip"}`. It takes about 3 minutes. `scans/chip/images/` has five images at each of 35 sites, on the same five planes.
6. **The panels:** `GET /api/v1/histo_scan_workflow/settings_ui` has the Background detect and Scan settings sections. The Slide Scan tab shows panels from P40.

## Design notes and deviations from the plan

- **Readiness:** the phase spec asks for a mapping and a background. The histo workflow needs a background only while it skips background, since only then does it use one.
- **The background detector** is the histo workflow's slot, `bg_channel_deviations_luv` unless configured.
  - **It's the camera's default detector too,** so the camera's calibration, in the wizard or `full_auto_calibrate`, sets the histo workflow's background as well.
  - **Frames are judged the same way:** the camera's `image_is_sample` and the workflow's `check_background` share `SampleCheck::of`. Both take a frame drawn after the call, shrunk to the low-resolution stream's size.
- **Check background's message:** its output has `sample`, `coverage` and `message`. Its panel button says the output, so action buttons now say an output's `message` when it has one, rather than its JSON.
- **The C-Chip workflow** reports each stack's centre as the site's focus, so the planner (P34) keeps the next site's z on the same plane. Otherwise each site would start where the last stack ended, and the stacks would climb.
- **Sharing:** the grid and histo workflows take smart stacks through one function, and their panels share the autofocus, capture and stack entries. The histo workflow repeats the stack settings themselves, since each Thing's settings are its own fields.
- **The default workflow:** the scan tests now choose the snake explicitly. One of them checks the new default.
- **On default settings, on my own server,** with the simulator's tissue sample:
  - **A histo scan,** to a range of 1,500 steps: 36 sites visited and 30 imaged, every image 0.75 µm from focus. The 6 that looked like background were skipped. It took 5 minutes, about 10 s an imaged site.
  - **A C-Chip scan:** all 35 sites, with five images at each on the same five planes, 50 steps apart. It took 3 minutes.
- **Size:** 910 changed lines of hand-written code: 10 over L's 900, and above my estimate of 650–720. The tests grew most, to 369 lines.

  | Part | Lines |
  |---|---|
  | `histo.rs` | 255 |
  | `c_chip.rs` | 163 |
  | Shared code, the camera, the default and the registry | 99 |
  | The web app's button and its test | 24 |
  | The Rust tests | 369 |

## Tests

- **`tests/workflows.rs`** (2 tests), over a tissue island: a picture of a 70 µm disc of tissue on the empty slide, with the mapping calibrated at its centre.
  - **A histo scan:**
    - **ready:** not without a background while skipping background, but ready without one when not skipping it;
    - **refused** while not ready, and on background;
    - **the background:** set from the empty slide, then checked there ("only …") and on the island;
    - **the scan,** with the default workflow: it ends "surrounded by background", with images only where a frame meets the island, and background sites visited but not imaged;
    - **equal distances and the range** reach the plan, in a second scan's settings.
  - **A C-Chip scan:** three images at each of the 35 sites of the pattern, on the same three planes, dz apart. The pattern's step is the frames' size less a tenth, through the mapping.
- **`tests/smart_scan.rs`:** the default workflow is `histo_scan_workflow`.
- **`ServerInterface.spec.ts`:** an action button says its output's message.

**Mutation checks:** I broke 13 behaviours, and a test failed for 12.

- **Killed:**
  - **the histo workflow:**
    - ready without a background while skipping it, or needing one when not;
    - starting on background;
    - equal distances ignored;
    - Set background not setting it;
    - the sample check inverted;
  - **the C-Chip workflow:** no focus reported, so the stacks climb; stacks from where z is rather than centred on it; the grid's overlap;
  - **the rest:**
    - the default workflow;
    - the grid workflows' focus;
    - the web app's button saying an output's JSON rather than its message.
- **One survives:** the maximum range not reaching the planner. Only a scan that reaches its range shows it, which takes minutes; the manual check did.
- **Not tried:** ignoring "skip background", since the spiral would then never end.

## Follow-ups and known gaps

- **The Slide Scan tab** (P40) shows the workflows and their panels.
- **The histo scan's planner** is told only whether a site was imaged. A site skipped as background is never revisited, even if a later stack would have found sample there.
- **Untested:**
  - the histo workflow's frame drawn after the call, rather than the latest;
  - a histo scan that reaches its range, which the manual check covers.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
