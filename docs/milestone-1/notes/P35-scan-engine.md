# P35: Scan engine with snake and raster workflows

- Status: In review
- Pull request: [#47](https://github.com/lucaswewa/microscope/pull/47)
- ADRs: [ADR-0029](../../adr/0029-scan-engine-and-workflow-things.md)
- Spec: [phases.md#p35](../phases.md#p35)

## Summary

The microscope can now scan: `smart_scan` runs the chosen workflow over its sites, and keeps each scan in a folder of its own.

- **The `ScanWorkflow` interface,** and the snake and raster workflows, which are one Thing type, `GridWorkflow`, configured as `snake_workflow` and `raster_workflow`.
- **`smart_scan.sample_scan(scan_name)`:**
  - one scan at a time, under a name made unique;
  - at each site it moves, at the planner's suggested z, then autofocuses and captures;
  - the scan's folder keeps the images with their sidecars, a record of its settings and how it went, and its log;
  - however it ends (it finishes, it's cancelled, or it fails), the stage goes back to the start.
- **`latest_scan_live_details`** shows the latest scan's progress.

Managing scans afterwards (`purge_empty_scans`, `download_zip`, deleting, and scans in the gallery) is P35b, split from this phase.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/workflow.rs` | `ScanWorkflow` (with `Begun` and `Acquired`), and `GridWorkflow`: its settings, `ready`, `display_name` and `settings_ui` |
| `crates/microscope-things/src/smart_scan.rs` | `SmartScan`: `workflow_name`, `all_workflow_names`, `workflow_display_names`, `latest_scan_live_details` and `sample_scan`, and the scan's folder, record and log |
| `crates/microscope-things/src/hardware/camera.rs`, `camera.rs`, `fakes.rs` | `CaptureMode` is now part of the camera interface, and `capture_to_memory` takes one |
| `crates/microscope-things/src/capture.rs` | `save_to`, for a capture whose name its caller chooses, and `mapping_of` |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.workflows:GridWorkflow`, as `snake_workflow` and `raster_workflow`, and `microscope.smart_scan:SmartScan`, as `smart_scan` |
| `web/src/api/generated/`, `web/tests/` | The new Things' types. The rail now has Slide Scan, as its placeholder until P40 |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

1. Calibrate the camera–stage mapping, if it isn't: at `http://127.0.0.1:5090/docs`, run `POST /api/v1/camera_stage_mapping/calibrate_xy`.
2. Run `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "try"}`. A 3 × 2 snake runs: it autofocuses, then at each site moves, autofocuses and captures. It takes about a minute.
3. Meanwhile, `GET /api/v1/smart_scan/latest_scan_live_details` shows its phase and image count.
4. When it's done, the stage is back where it started. The data folder's `scans/try/` has:
   - `images/`, with six images named for where they were taken, each with its sidecar;
   - `scan.json`, the record, its `result` `success`;
   - `scan.log`.
5. Run it again with the same name: the folder is `try-2`.
6. Start a longer scan (`PUT /api/v1/snake_workflow/x_count` to 6), then cancel it (`DELETE` its invocation). The stage goes back to the start, and the record says `cancelled`.

## Design notes and deviations from the plan

- **The split.** P35 was estimated at 1,050–1,300 lines against L's 900. The owner chose to split it before it was built: P35b has `purge_empty_scans`, `download_zip`, the delete endpoint and scans in the gallery. The plan, `phases.md` and Appendix D are updated.
- **One Thing type for both grid workflows,** `GridWorkflow`, with its `order` in the configuration's `kwargs`.
  - **OpenFlexure** has a class for each. Here they would repeat their fields and code, since `teta-wot` Things can't be generic.
  - **The Thing names are OpenFlexure's:** `snake_workflow` and `raster_workflow`.
- **The workflows' work:** OpenFlexure's grid workflows autofocus before the scan, then take a smart z-stack at each site. Here each site has a fast autofocus and one capture, since smart z-stacks are P36.
- **One scan at a time:**
  - in the shipped configuration, the global lock refuses a second scan, or any other action, while one runs;
  - `smart_scan`'s own lock refuses a second scan when the global lock is off.
- **The scan's name** is checked, since it names a folder: letters, digits, spaces, `-`, `_` and `.`, not starting with `.`. Without one, it's `scan-<UTC time>`, as captures are named (ADR-0025).
- **The return to the start** includes z, as OpenFlexure's does, so the start is where the scan began, before its first autofocus.
- **The record** (`scan.json`) is written when the scan begins, and again when it ends, with the result. A scan the server didn't finish has no result.
- **Where focus was found** is what the planner learns: the z the autofocus ended at, or the z it moved to if it didn't find focus. A failed autofocus still takes the image, as OpenFlexure's grid scans do.
- **The camera interface** gained the capture mode, so a workflow's `capture_mode` (`standard` or `quick`) works with any camera driver.
- **Checked by hand,** on my own server (port 5098) with its own data folder, after the end-to-end tests had calibrated it:
  - a 2 × 1 snake took 21 s;
  - its two images were named for their sites;
  - the second move started at the z where the first site focused;
  - the log was complete;
  - the stage ended at its start.
- **Size:** 1,038 changed lines of hand-written code against L's budget of 900, 15% over even after the split. I'd estimated about 800 for the split phase. The tests grew most, from what the mutation checks called for.

  | Part | Lines |
  |---|---|
  | `smart_scan.rs` | 311 |
  | `workflow.rs` | 228 |
  | The capture mode in the camera interface, `capture.rs`, the registry and the configuration | 112 |
  | Tests: `smart_scan.rs` 373, the camera's and the interface's 13, the end-to-end rail 1 | 387 |

## Tests

`tests/smart_scan.rs` (4 tests), on the simulator, each with both workflows and a calibrated mapping:

- **a 2 × 2 snake:**
  - the display names;
  - the scan's name returned;
  - the stage back at the start, z included;
  - four images, each named for its site, with its sidecar saying the same position;
  - the record's name, workflow, settings, start, image count and result;
  - four moves in the log, and why it finished;
  - the live details, complete;
  - each move after the first at the z where the site before it found focus;
  - **then the same name again,** as a 2 × 2 raster on a sample tilted 100 µm per mm in x, captured `quick`:
    - it becomes `first-2`;
    - its third site starts from the first site's focus, the nearest, not from the second's, where the stage is;
    - its images are the stream's size;
- **cancelling a 4 × 4 raster** once it has an image:
  - another scan is refused while it runs;
  - the invocation ends `cancelled`;
  - the stage is back at the start;
  - the record says `cancelled`, with fewer than 16 images;
- **a scan that fails:** the second column is beyond the stage's travel, so the move there fails. The stage is back at the start, and the record says `error: …`, after one image;
- **refusals:**
  - a workflow that isn't ready (no mapping);
  - an unknown workflow;
  - names with `..`, a slash, a leading dot or a colon;
  - no folder is made for a refused scan.

The camera's and the hardware interface's tests now pass a capture mode.

**Mutation checks:** I broke each of 12 behaviours. A test failed for 11 of them:

- no return to the start;
- names not made unique, or not checked;
- the planner's z ignored;
- the planner never told of focus;
- a cancelled scan recorded as an error;
- no final record;
- the image count not kept;
- a workflow that isn't ready allowed;
- the capture mode ignored;
- the snake run as a raster.

At first, two of the 11 survived, and a third was caught only by a hang:

- **the z ignored, and focus never reported:** in a 2 × 2 snake, the nearest focused site is always the one just visited, where the stage already is. The raster on a tilted sample tells them apart;
- **the image count:** the cancel test waited for an image for ever. It now gives up after a minute.

The 12th, the check for cancellation at each site, makes no difference here, since autofocus and the stage's moves notice cancellation themselves. It's kept, so a scan still ends between sites with a workflow that doesn't.

## Follow-ups and known gaps

- **P35b:** `purge_empty_scans` (a refused or failed scan can leave an empty folder), `download_zip`, deleting, and scans in the gallery with the folder viewer.
- **Smart z-stacks** (P36) replace the single capture at each site.
- **Untested:** `smart_scan`'s own lock, which only matters when the global lock is off; and the check for cancellation at each site, as above.
- **The Slide Scan tab** (P40) will run scans and show their progress.
