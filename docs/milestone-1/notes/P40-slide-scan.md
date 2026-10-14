# P40: Slide Scan tab

- Status: Done
- Pull request: [#53](https://github.com/lucaswewa/microscope/pull/53)
- ADRs: none new
- Spec: [phases.md#p40](../phases.md#p40)

## Summary

The Slide Scan tab, for running a scan from the app. With it, Checkpoint D is reached: all four workflows run on the simulator, show a live stitching preview, and end as a stitched deep-zoom image in the gallery.

- **Before a scan,** the pane has:
  - the workflow, with what it does;
  - its settings, from the server;
  - the stitching settings, whose title says when they're changed;
  - a sample ID;
  - Start Smart Scan.
- **While one runs,** wherever it was started:
  - the main view shows its live stitching preview, and the pane a mini stream;
  - the pane has its ID, with its settings in a dialog, its images, its duration and its log;
  - a Cancel names the phase.
- **Once it ends:** Download ZIP, and Close back to the settings.

## What was built

| Where | What |
|---|---|
| `web/src/scan/SlideScanPage.vue` | The tab: the settings before a scan, and the scan while it runs and once it ends |
| `web/src/scan/scan.ts` | `useScan`: the latest scan's details, read every second; the scan shown, started here or found running; Cancel and Close |
| `web/src/scan/ScanInfo.vue` | A scan's ID, which opens its settings, its images and its duration |
| `web/src/router/index.ts`, `src/api/things/index.ts` | The tab's route in place of its placeholder; `smart_scan` and `LiveScanDetails` in the typed facade |
| `crates/microscope-things/src/workflow.rs`, `histo.rs`, `c_chip.rs`, `smart_scan.rs` | Each workflow says what it does, and `smart_scan.workflow_descriptions` lists them, for the tab's blurb |
| `web/tests/unit/scan/`, `tests/e2e/scan.spec.ts`, `tests/visual/` | The tests, and the tab's baselines in both themes |
| `web/playwright.config.ts` | A `scan` project, after the others, for the end-to-end scan |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. **Calibrate:** open `http://127.0.0.1:5090/`, and run the calibration wizard.
2. **Choose a workflow:** open Slide Scan, and choose "Snake scan". It says what it does, and its settings follow. Set Columns to 2 and Rows to 1, so it's quick.
3. **Scan:** type a Sample ID, and press **Start Smart Scan**.
   - **The pane** shows the scan's ID, images, duration and log. **Cancel setup** becomes **Cancel scan** as it starts visiting sites.
   - **The main view** shows the live stitching preview once the first image is in. The pane's mini stream shows the camera meanwhile.
   - **The settings icon** by the ID shows the workflow's settings for this scan.
4. **Once it ends:** "The scan has finished.", with **Download ZIP** and **Close**. The gallery has its card, stitched, and View opens the mosaic.
5. **Picking up:** start a scan, then open Slide Scan in another window. It shows the same scan, which it can cancel.
6. **Stitching settings:** turn off "Stitch the scan once it ends", and close the section. Its title says "(changed)".

## Design notes and deviations from the plan

- **The blurb is the server's:** each workflow says what it does (`ScanWorkflow::description`), and `smart_scan.workflow_descriptions` lists them by name, as `workflow_display_names` lists their names.
  - **Configured workflows:** a configuration's workflows bring their own words, so the app holds none.
- **The latest scan's details are read every second,** since they're computed when read, and so can't be observed.
- **The scan shown:**
  - **It's the one the tab starts,** or one found running, however it was started, until Close.
  - **Finding it:** when the details say a scan is running and none is shown, the tab asks for `sample_scan`'s running invocations and follows the first, for its log and Cancel.
- **A scan refused** before it begins, for a workflow that isn't ready, say, is reported in a toast, and the settings stay.
  - **Telling it apart:** the shown scan's details count as its own only once they've been seen to begin, so the previous scan's are never shown as the new one's.
- **Cancel** says "Cancel setup" while the workflow gets ready, and "Cancel scan" once it visits sites. Once pressed, it says "Cancelling…" until the scan ends.
- **The main view** shows the live preview once there is one, and the live image until then. The preview's URL carries the time it was drawn, so each is fetched once.
- **The stitching settings:**
  - **The section** is closed at first, since it rarely changes. Its title says "(changed)" while `stitch_automatically` differs from its default.
  - **The control** inside highlights the change, as every property control does (P24).
- **The settings dialog** lists the workflow's settings for the scan, as its record keeps them.
- **The end-to-end scan:**
  - **It runs last,** in its own Playwright project, since a scan holds the microscope's lock for a while.
  - **It's a 2 × 1 snake at the stream's size,** set over the API: about 20 s, then a few more to stitch.
- **Checkpoint D, on my own server,** with the tissue sample and small settings. Each scan drew its live preview and ended stitched:

  | Workflow | Images | Mosaic | Time |
  |---|---|---|---|
  | Snake, 2 × 1 | 2 | 1353 × 616 | 21 s |
  | Raster, 2 × 1 | 2 | 1353 × 616 | 21 s |
  | Histo, range 1000 | 11 | 1886 × 2217 | 82 s |
  | C-Chip, 1 image a site | 35 | 3770 × 3941 | 61 s |

- **In a browser, in both themes,** on my own server:
  - the tab before a scan, with the histo and snake workflows' panels;
  - a snake scan setting up, running with its preview and mini stream, and ended;
  - the stitching section's "(changed)".
- **Size:** 770 changed lines of hand-written code, within L's 900 and my estimate of 680–780.

  | Part | Lines |
  |---|---|
  | `SlideScanPage.vue` | 227 |
  | `scan.ts` | 101 |
  | `ScanInfo.vue` | 77 |
  | The workflows' descriptions, in Rust | 46 |
  | Routing, the facade and Playwright's projects | 20 |
  | Tests | 299 |

## Tests

- **`tests/unit/scan/SlideScanPage.spec.ts`** (5 tests), against a fake server:
  - **choosing a workflow:** its name and blurb, its panel, and choosing another writes `workflow_name`; the stitching section's "(changed)";
  - **a scan:**
    - its request with the sample ID;
    - the last scan's details not shown as its own;
    - "Cancel setup", then "Cancel scan";
    - its log, images, duration and the preview's URL;
    - Cancel, "Cancelling…", and its end with Download ZIP and Close;
  - **its settings** in the dialog;
  - **a scan started elsewhere,** picked up, with its log and Cancel;
  - **a scan refused:** the toast, and the settings stay.
- **`tests/e2e/scan.spec.ts`:** Checkpoint D in the browser.
  - A 2 × 1 snake from the tab shows its preview and finishes.
  - Its card in the gallery then shows the stitched thumbnail, and View loads `mosaic.dzi` into the deep-zoom viewer.
- **`tests/visual/`:** the tab before a scan, in both themes.
- **Rust:** `workflow_descriptions` for all four workflows.

**Mutation checks:** I broke 10 behaviours of the tab, and a unit test failed for each.

- **The scan:** a Cancel blind to the phase, the last scan's details shown, no picking up, a refused scan kept, no cancel sent.
- **The pane:** the preview's URL without its time, the stitching title never "(changed)", the workflow not written, no Download ZIP, a duration in seconds.

## Follow-ups and known gaps

- **Enumerated settings,** such as Capture mode, show as text fields in server-described panels, not as choices. The panels' property controls (P24) don't read the enums schemars describes. They could.
- **One scan's log** is the one shown. A scan picked up shows its log from its start, since the server keeps it.
- **The live image and the preview:** there's no switch between them while a preview exists. The pane's mini stream shows the camera.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
