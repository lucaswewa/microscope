# P42: Sequence tab

- Status: In review
- Pull request: [#55](https://github.com/lucaswewa/microscope/pull/55)
- ADRs: none new
- Spec: [phases.md#p42](../phases.md#p42)

## Summary

The Sequence tab, for setting up, running and following a timelapse. With it, Checkpoint E is reached: a timelapse with autofocus runs, can be cancelled, and appears in the gallery with the image-sequence viewer.

- **Before a sequence,** the pane has:
  - its duration, as hours, minutes and seconds;
  - the interval;
  - an autofocus before each capture, and its range;
  - a name;
  - Start Sequence.
- **While one runs,** wherever it was started:
  - the main view shows its latest image, and the pane a mini stream;
  - the pane has its name, the images captured and expected, the time elapsed and remaining, the points skipped, and its log;
  - Cancel.
- **Once it ends:** how it ended, a link that opens it in the gallery's viewer, Download ZIP, and Close back to the form.
- **On the server,** `sequence.latest_sequence_live_details` says how the latest sequence is going.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/sequence.rs` | `latest_sequence_live_details`: the latest sequence's name, time points, images saved, points skipped, time run, when its last point is due, its latest image and its result. The save queue counts each image it saves |
| `web/src/sequence/SequencePage.vue` | The tab: the form before a sequence, and the sequence while it runs and once it ends |
| `web/src/sequence/sequence.ts` | `useSequence`: the latest sequence's details, read every second; the sequence shown, started here or found running; Cancel and Close. `clock`, for times as `h:mm:ss` |
| `web/src/gallery/GalleryPage.vue` | `/#/gallery?view=<path>` opens that item's viewer |
| `web/src/router/index.ts`, `web/src/api/things/index.ts` | The tab's route, and the `sequence` Thing's facade |
| `web/playwright.config.ts` | A `sequence` project, after the scan's |
| `web/src/api/generated/` | The new property's types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

At `http://127.0.0.1:5090/#/sequence`:

1. **A timelapse with autofocus:** set the duration to 0 h 1 min 0 s, the interval to 10, and turn on Autofocus before each capture, with a range of 1000. Name it `try`, and Start Sequence.
   - Once the first image is saved, the pane shows "Images 1 of 7", the time elapsed and remaining, and a mini stream.
   - The main view shows the latest image.
2. **Cancel** after a few images. The pane says "The sequence was cancelled."
3. **See it in the gallery** opens it in the viewer, where the slider steps through its images. Close leaves the gallery.
4. **Skipping:** run another with an interval of 1 s. Each autofocus takes longer than that, so the pane counts the points skipped, and the log says how many each time.
5. **From another window:** start a sequence, then open the tab in a second window. It shows the same sequence, and either can cancel it.

## Design notes and deviations from the plan

- **The live details are the server's,** as a scan's are (P35): a property the tab reads every second, since a computed property can't be observed.
  - **Images expected** are the time points less those skipped so far, so they fall as points are skipped.
  - **The time remaining** is until the last time point, not the end of the duration. A sequence of 10 s at 3 s has its last point at 9 s.
  - **Images captured** are those the save queue has saved, so the latest image is always on disk when the tab shows it.
  - **The record's skipped points** now come from the same count.
- **The main view** shows the latest image and the pane a mini stream, as the Slide Scan tab shows its stitching preview.
- **A sequence too short to be seen running,** such as a single capture, is still shown once its invocation ends: its details are those with the name it returned. Until then, the last sequence's details aren't taken for its own.
- **The form** is property controls that the web describes (ADR-0024), as the Slide Scan tab's stitching settings are. Its duration uses the `duration_hms` input. The settings apply to the next sequence; the form is hidden while one runs.
- **The gallery link,** `/#/gallery?view=sequences/<name>`, opens that item's viewer, on whichever page its card is. Closing the viewer drops `view` from the address, so the item doesn't open again.
- **`sequence.ts` follows a sequence** as `scan.ts` follows a scan. Sharing one composable would have moved P40's code, so it's left as a follow-up.
- **The skip warning** now says "1 time point" for one.
- **The end-to-end test** runs in its own Playwright project after the scan's, since each holds the microscope.
- **Size:** 834 changed lines of hand-written code, within L's 900 but over my estimate of 650–750. P42 was re-sized from M to L, at the project owner's choice.
  - **The tests** came to 353 lines, against about 280 estimated.

  | Part | Lines |
  |---|---|
  | The live details, in Rust | 119 |
  | `SequencePage.vue` | 208 |
  | `sequence.ts` | 118 |
  | The gallery link, routing, the facade and Playwright's projects | 36 |
  | Tests | 353 |

## Tests

- **`microscope-things/tests/sequence.rs`:** the live details
  - after a timelapse that ends between points, so its last point is at 2 s of 2.5 s;
  - after skipping;
  - while one runs, and once it's cancelled;
  - and none after names that are refused.
- **`tests/unit/sequence/SequencePage.spec.ts`** (6 tests), against a fake server:
  - the form, and the duration entered as hours, minutes and seconds;
  - a sequence through its progress, log, Cancel and end, and the gallery link;
  - a sequence too short to be seen running;
  - picking up a sequence started elsewhere;
  - a refused sequence;
  - times as `h:mm:ss`.
- **`tests/unit/gallery/GalleryPage.spec.ts`:** a link opens its item's viewer, and closing it drops the link.
- **`tests/e2e/sequence.spec.ts`:** Checkpoint E in the browser. A timelapse of an hour, at 1 s, with autofocus, is set up and started from the tab, shows its latest image, is cancelled, and opens in the gallery's viewer, whose slider steps through it.
- **Visual:** the tab before a sequence, in both themes.

**Mutation checks:** I broke 17 behaviours, and a test failed for each.

- **The live details:** images saved not counted, the latest image not set, points skipped mid-sequence not counted, the time run frozen while running, no result at the end, and the last point taken as the duration's end.
- **The tab:**
  - a sequence too short to be seen running never taken as begun;
  - the last sequence's details taken for its own;
  - the details taken as running whatever their result;
  - images expected not less those skipped;
  - the time remaining as the last point's time, or shown after the end;
  - the duration not as hours, minutes and seconds;
  - the gallery link without its item;
  - no latest image.
- **The gallery:** a link not opening its item, and the link kept after closing.

## Follow-ups and known gaps

- **`scan.ts` and `sequence.ts`** could share one composable for following a run.
- **The remaining time** reaches 0:00:00 when the last point is due. Its autofocus and capture still follow, so a sequence ends a little later.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
