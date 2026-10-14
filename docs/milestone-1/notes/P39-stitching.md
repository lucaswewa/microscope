# P39: Live and final stitching

- Status: Done
- Pull request: [#52](https://github.com/lucaswewa/microscope/pull/52)
- ADRs: none new
- Spec: [phases.md#p39](../phases.md#p39)

## Summary

Scans are now stitched, using P38's mosaic core.

- **While a scan runs,** a preview of its mosaic is drawn again after each site, at `smart_scan/latest_preview_stitch.jpg`.
- **Once it ends,** it's stitched in the background into a deep-zoom mosaic, if `stitch_automatically` is set, as it is by default.
- **On request:** `stitch_scan` stitches one scan, and `stitch_all_scans` stitches every scan that isn't.
- **In the gallery,** a stitched scan's card opens its mosaic in the deep-zoom viewer, and offers the overview JPEG and a ZIP of the tiles. An unstitched scan's card has a Stitch button.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/stitch.rs` | A scan's images, one a site, placed by their sidecars; `stitch`, which writes the scan's `stitched/` folder; `Preview`, the live preview |
| `crates/microscope-things/src/smart_scan.rs` | The `stitch_automatically` setting; the live preview after each site, its endpoint, and `preview_stitch_time` in the live details; the stitch as a scan ends; `stitch_scan`, `stitch_all_scans` and `download_stitched_zip` |
| `crates/microscope-things/src/gallery.rs` | Scan cards: the mosaic in the deep-zoom viewer, its thumbnail and downloads, and the Stitch button |
| `crates/microscope-things/tests/stitch.rs`, `tests/smart_scan.rs`, `tests/gallery.rs`, `tests/workflows.rs` | The tests |
| `web/src/api/generated/` | The new affordances' types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

At `http://127.0.0.1:5090/docs`:

1. **Calibrate** the camera–stage mapping: `POST /api/v1/camera_stage_mapping/calibrate_xy`.
2. **Scan:** `PUT /api/v1/smart_scan/workflow_name` to `"snake_workflow"`, then `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "try"}`.
3. **While it runs,** open `http://127.0.0.1:5090/api/v1/smart_scan/latest_preview_stitch.jpg` and reload it: the mosaic grows a site at a time. `GET /api/v1/smart_scan/latest_scan_live_details` has its `preview_stitch_time`.
4. **Once it ends,** `scans/try/scan.log` says "Stitched", and `scans/try/stitched/` has `mosaic.dzi`, `mosaic_files/`, `overview.jpeg` and `thumbnail.jpeg`.
5. **In the gallery,** `http://127.0.0.1:5090/#/gallery`, the scan's card shows the mosaic's thumbnail.
   - **View** opens the mosaic in the deep-zoom viewer.
   - **Download** offers "Mosaic (JPEG)" and "Mosaic tiles (ZIP)" too.
6. **On request:** `PUT /api/v1/smart_scan/stitch_automatically` to `false` and scan again. The new card has a **Stitch** button, which stitches it and shows the mosaic. `POST /api/v1/smart_scan/stitch_all_scans` stitches every scan that isn't.

## Design notes and deviations from the plan

- **One image a site:** a site with a stack (P36) gives the middle of its images, by z. With images to save centred on the sharpest, that's the sharpest.
- **Where the mosaic goes:** `scans/<name>/stitched/`, as plan §5.9 has it, holding `mosaic.dzi`, `mosaic_files/`, `overview.jpeg` and `thumbnail.jpeg`.
  - **Written beside it first,** in `stitched.partial/`, then moved into place, so the gallery never shows a half-written mosaic. A stitch replaces the one before.
- **The live preview** keeps a copy of each site's image, an eighth of its size, and blends them all again after each site, at up to 1024 pixels on its longer side.
  - **The layout changes** as a spiral grows in every direction, so it's drawn whole each time rather than added to.
  - **If it can't be drawn,** that's logged, and the scan carries on.
- **The final stitch** runs in the background once the scan has ended and its folder is tidied, however it ended, if it saved an image.
  - **So a scan's action ends** without waiting for it.
  - **The scan's log** says when it's done: "Stitched: W × H pixels", or why it failed.
- **One stitch of a scan at a time:**
  - **A second request** is refused while one runs.
  - **`stitch_scan` refuses** the scan being run.
  - **`stitch_all_scans` skips** both, and logs any scan it can't stitch, such as one whose images have no stage positions.
- **No global lock:** stitching and its download run without it, on a blocking thread, as plan §5 has it for stitching and downloads.
- **The gallery card** of a stitched scan shows its `stitched/thumbnail.jpeg`, and the gallery notices a new stitch by the `.dzi`'s stamp.
- **Downloads:** the overview through the data route, and the tiles as a ZIP built in memory, as the images' ZIP is (P35b).
- **Tests turn automatic stitching off,** except where a test waits for the stitch. Otherwise a stitch still writing could fail a test removing its data.
- **Size:** 758 changed lines of hand-written code, over M's 600 and above my estimate of 520–650. The project owner chose to keep one pull request.

  | Part | Lines |
  |---|---|
  | `stitch.rs` | 174 |
  | `smart_scan.rs` | 221 |
  | `gallery.rs` and `lib.rs` | 61 |
  | Tests | 302 |

## Tests

- **`tests/stitch.rs`** (2 tests), with scans written as a scan leaves them: 9 tiles cut from a known picture, with sidecars of their stage positions and mapping. The middle site has a stack of three, of which only the middle is right.
  - **A scan, stitched:**
    - its size, levels and images;
    - its overview, the picture again, so the middle of the stack was used;
    - no partial folder left;
    - its card: the deep-zoom viewer, the mosaic's thumbnail, the four downloads, no Stitch button;
    - the tiles' ZIP;
    - stitched again, replaced;
    - refusals for names outside `scans/`, and for the ZIP of an unstitched scan.
  - **Stitching all:** only the scan not stitched yet, skipping one without sidecars.
- **`tests/smart_scan.rs`:**
  - no preview before a scan;
  - after it, the preview's JPEG and its time;
  - the stitch the scan started, and its log line.
- **`tests/gallery.rs`:** an unstitched scan's card has the Stitch button.

**Mutation checks:** I broke 13 behaviours, and a test failed for each.

- **Stitching:** the first of a stack used, not its middle; the old stitch kept; no thumbnail.
- **The live preview:** never drawn, its time not set, or never served.
- **The final stitch:** never started.
- **Stitching all:** restitching stitched scans; stopping at the first that fails.
- **Downloads:** an unstitched scan's tiles zipped.
- **The card:** not switching to the deep-zoom viewer; no Stitch button; the gallery not noticing a new stitch.

The "Untested" list below names three I left out, since no test could reliably catch them.

**In a browser, in both themes,** on my own server, after a 3 × 2 snake scan:

- **the live preview** grew as the scan went: 5.5 KB after its first site, 15.8 KB after its sixth, each drawn with its time in the live details;
- **the mosaic,** stitched 5 seconds after the scan ended: 3771 × 2033 pixels;
- **its card's thumbnail,** its four downloads, and View opening it in the deep-zoom viewer;
- **an unstitched scan's card** with its Stitch button. Through the API, `stitch_scan` stitched it, and `stitch_all_scans` then had nothing left to do.

## Follow-ups and known gaps

- **The Slide Scan tab** (P40) shows the live preview as a scan runs.
- **Untested:**
  - a scan stitched although `stitch_automatically` is off, since the stitch would run in the background, after the test had looked;
  - `stitch_scan` refusing the scan being run;
  - the live preview cleared as a new scan starts.
- **The tiles' ZIP is built in memory,** as the images' is. A large mosaic should be written to a file.
- **The live preview** reads every sidecar after each site. A scan of thousands of sites would want it kept as it goes.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
