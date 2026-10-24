# P13: Correlation in stitching

- Status: In review
- Pull request: [#82](https://github.com/lucaswewa/microscope/pull/82)
- ADRs: none new; [ADR-0046](../../adr/0046-correlation-stitching.md) decided it
- Spec: [phases.md#p13](../phases.md#p13)

## Summary

A scan's final mosaic has its images where their overlaps put them, not just where the stage said they were, using P12's correlation placement. A new stitching setting, on by default, turns it off. `tiles.json` records both placements, the gallery's viewer shows how far each image moved, and the scan's log says so. The live preview keeps the stage's positions.

## What was built

| Where | What |
|---|---|
| `crates/microscope-core/src/placement.rs` | `place_with`: placement for images read as they're needed, each once, and dropped after its last pair |
| `crates/microscope-things/src/stitch.rs` | `stitch(scan, refine)`: refined by correlation, registering images shrunk to about 400 pixels across. `Stitched::correlation` (`Corrections`: how many moved, the furthest) and `summary`. `PlacedImage::stage_x` and `stage_y` |
| `crates/microscope-things/src/smart_scan.rs` | The `refine_with_correlation` setting, on by default, for `stitch_scan`, `stitch_all_scans` and the stitch once a scan ends |
| `web/src/overlay/corrections.ts` | The Corrections layer |
| `web/src/gallery/ImageViewer.vue` | The layer, over a stitched mosaic whose images moved |
| `web/src/scan/SlideScanPage.vue` | "Refine with image correlation" in the Stitching settings, and their "(changed)" for either setting |
| `web/src/api/generated/` | Regenerated |

### What's recorded

| Where | What |
|---|---|
| `stitched/tiles.json` | Each image's `x` and `y`, where it is in the mosaic, and its `stage_x` and `stage_y`, where the stage put it, in the same pixels |
| `stitch_scan`'s output | `correlation`: `moved`, how many images moved a pixel or more, and `largest`, the furthest, in pixels; `null` without refinement |
| `scan.log` | "Stitched: 3778 × 2826 pixels; correlation moved 8 of the 9 images, the furthest by 13.5 pixels", or, without refinement, "…, where the stage put the images" |

### The Corrections layer

At each image's middle, how far correlation moved it, such as "14 px", and a line from where the stage put its middle. Hovering on it names the image. The Layers menu turns it off and on, beside Image outlines. A mosaic whose images all stayed where the stage put them has no Corrections layer, and neither does one stitched before P13.

## How to try it

1. **The tests:**

   ```powershell
   cargo test -p microscope-core placement
   cargo test -p microscope-things --test correlation -- --nocapture
   ```

2. **In the app,** with the server and the web app run as usual:
   - Calibrate the camera–stage mapping, then turn the drift on (P12's [How to try it](P12-correlation-placement.md#how-to-try-it)).
   - **Slide Scan:** open **Stitching settings**. "Refine with image correlation" is on.
   - **Scan** a 3 × 3 snake. Once it's stitched, its `scan.log`, in `.microscope\data\scans\<its name>\`, ends "correlation moved … of the 9 images, the furthest by … pixels".
   - **Gallery:** view the scan. Zoomed in where images overlap, the blobs' edges are single. The **Layers** menu has **Corrections**, with how far each image moved.
   - **Without correlation:** turn the setting off, and scan again. The seams show doubled edges, and there's no Corrections layer.
   - **Turn the drift off** when you're done.

## Hardware

None yet. P13 has no driver, and was tried on the simulator, on Windows. Checkpoint B's scans, on the Zaber stage and on the Sangaboard, wait for P05 and P10, which are skipped for now.

## Design notes and deviations from the plan

- **Without P10:** P13 depends on P10, which is skipped for now. It's built and tested on the simulator, and its scans on hardware wait.
- **Shrunk to about 400 pixels across** for correlation: by 4 for a 1640 × 1232 capture, and by 2 for a quick one.
  - Halved, as P12 registered by default, a full-resolution capture's JPEG blocks were a stronger match than the sparse default sample. A scan of mine put one image 10 pixels wrong from a pair 0.30 sure, and the test's scan came out 28.6 pixels wrong.
  - Shrunk by 4, the right pairs were 0.64 to 0.97 sure and the wrong ones 0.25 at most.
  - Shrunk by 8, some wrong pairs were 0.4 to 0.5 sure.
- **Pairs that disagree are dropped,** a fix after P13 was merged. In a busy CI run, the drifting scan's test failed once: one wrong pair, 0.36 sure, put an image 14 px out. Now the used pair that disagrees most with where the solve put its two images, by more than 3 px, is dropped, and the images are solved again, until none does. An image's other pairs outvote a wrong one.
- **The stitch tests of P39 and P53 turn refinement off.** Their picture is a smooth gradient and wave, cut exactly. Through JPEG, little but the JPEG's blocks is left for correlation to match, and it moved the images by up to 7 pixels. They test placement by the stage, and the drifting scan tests correlation.
- **Each image is read once more,** for the pairs it's in, and dropped after its last, so a 20 × 20 scan holds a column or two of its images at a time.
- **"Moved"** is by a pixel or more. Each image's move is measured against its own stage placement, before the mosaic's corner is moved to the leftmost and topmost image.
- **The live preview keeps the stage's positions,** as the spec says. Its `tiles.json` has each image's stage placement as its place.
- **Size:** 551 changed lines (499 added, 52 removed), within M's 600. The estimate was about 450: the drifting scan's test, and reading images as they're needed, came to more.

## Tests

- **Placement** (`microscope-core`): images read as they're needed are each read once, placed as `place` places them, and an error reading one is the placement's.
- **A drifting stage's scan** (`correlation.rs`, moved there after P13 from `smart_scan.rs`, so it doesn't run beside the other scans and slow their cameras):
  - a 3 × 3 snake on the default sample, autofocused and captured whole, as scans are by default, with 2% of the steps lost;
  - stitched once it ends, refined: within 2.04 pixels of where the images really are, by their sidecars, the corner image the furthest. The test asks for 2.5. Its log says how many moved, and `tiles.json` records where the stage put them;
  - stitched again without refinement: 28.6 pixels out. The test asks for more than 10.
  - With the images only halved for correlation, it fails, 28.6 pixels out.
- **Stitching** (`stitch.rs`): `tiles.json` has each image's stage placement, and the output says it wasn't refined.
- **The web app:**
  - the Corrections layer: a line and a label for each image, and none where no image moved or the layout doesn't say where the stage put them;
  - the viewer shows it over a stitched mosaic;
  - the Slide Scan tab offers the setting, and shows its change on the closed section.
- **In the app**, on a server of my own: a 3 × 3 snake on the default sample, autofocused, with the drift at its defaults. Refined, every image was within 1.7 pixels of where it really is, against up to 14.3 by the stage. Refining took the stitch from 7 to 9 seconds in a debug build. I looked at the viewer in both themes.

## Follow-ups and known gaps

- **Checkpoint B:** a scan on the Zaber stage and one on the Sangaboard, once P05 and P10 are done.
- **An image correlation can't place keeps the stage's placement,** such as one autofocus missed, or an empty field. A better autofocus on sparse fields would help.
- **The Pi:** refining a large scan there isn't measured yet.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
