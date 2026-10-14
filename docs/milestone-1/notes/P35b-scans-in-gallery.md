# P35b: Scans in the gallery

- Status: Done
- Pull request: [#48](https://github.com/lucaswewa/microscope/pull/48)
- ADRs: none new
- Spec: [phases.md#p35b](../phases.md#p35b)

## Summary

Finished scans can now be listed, viewed, downloaded and deleted. Split from P35.

- **Scans are a card type in the gallery:**
  - a thumbnail of the first image;
  - the images, the workflow and the duration;
  - a message if a scan was cancelled, stopped with an error, or didn't finish;
  - **View** steps through the images with the sequence viewer (P33), until stitching (P39) makes a mosaic.
- **Downloads:** each scan downloads as a ZIP of its images, sidecars, record and log, or as its record alone.
- **Deleting:** `DELETE /api/v1/gallery/scans/{name}` deletes a scan, but not the one being scanned. Delete All deletes every scan.
- **Purging:** `smart_scan.purge_empty_scans` removes scan folders without an image, and every scan does so as it ends, as OpenFlexure's does.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/gallery.rs` | The `Scans` provider, and the `DELETE scans/{name}` endpoint. The gallery now knows `smart_scan`'s name and the server, for the ZIP button and the running scan |
| `crates/microscope-things/src/smart_scan.rs` | `purge_empty_scans`, `download_zip(scan_name)` (a ZIP Blob), and `scanning()`, the name of the scan running. A scan purges empty folders as it ends |
| `crates/microscope-things/tests/gallery.rs`, `tests/smart_scan.rs` | The tests |
| `web/src/api/generated/` | The new affordances' types |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. With the camera–stage mapping calibrated, run a small scan: at `http://127.0.0.1:5090/docs`, `PUT /api/v1/snake_workflow/x_count` to 2 and `y_count` to 1, then `POST /api/v1/smart_scan/sample_scan` with `{"scan_name": "try"}`.
2. Open `http://127.0.0.1:5090/#/gallery`. The scan is a card: its thumbnail, "Images: 2", the workflow and the duration.
3. Press **View**: the viewer steps through its images with the slider.
4. Open **Download**: "Images (ZIP)" holds the images with their sidecars, `scan.json` and `scan.log`; "Record" is `scan.json`.
5. Press **Delete**, and confirm. The folder `scans/try` is gone.

## Design notes and deviations from the plan

- **The delete endpoint is on the gallery,** `DELETE gallery/scans/{name}`, as ADR-0026 (P31) has it for every card type. The phase spec, written before that ADR, put it on `smart_scan`.
  - **Refusing the running scan:** the gallery asks `smart_scan` which scan is running, and answers 409.
  - **Delete All** takes the global lock, so it can't run during a scan.
- **The ZIP** is built in memory and returned as a Blob from `download_zip`, as OpenFlexure's action is. It holds the images and their sidecars under `images/`, with `scan.json` and `scan.log`, stored without compression.
- **A scan's thumbnail** is `thumbnail.jpeg` in its folder, a small copy of its first image, made when the gallery first lists it.
- **Its messages** come from the record's result:
  - `cancelled`: a warning;
  - `error: …`: an alert;
  - no result: a warning that it hasn't finished. That's a scan running, or one the server didn't finish.
- **Purging at the end of each scan,** as OpenFlexure's does, removes a scan that failed before its first image, along with its record. Its log was in the action's log too.
- **Checked in a browser, in both themes,** on my own server (port 5098):
  - a scan's card;
  - View, stepping through its images;
  - the download menu.
- **Size:** 507 changed lines of hand-written code, within M's budget of 600.

  | Part | Lines |
  |---|---|
  | `gallery.rs` | 231 |
  | `smart_scan.rs` | 96 |
  | Tests | 180 |

## Tests

- **`tests/gallery.rs`** (1 new test), with scans written to the data folder as a scan leaves them:
  - **a scan's whole entry:**
    - its sequence viewer, with the images in order;
    - its thumbnail, made, and the size of the first image;
    - its info lines;
    - its two downloads;
    - its delete endpoint;
  - **messages** for an error, a cancellation and a scan without a result;
  - **the ZIP:** its four files, and refusals for an unknown name, `..`, and none;
  - **deleting:** traversal attempts and an unknown name answer 404, and a scan is deleted with its folder;
  - **purging** removes only the folder without an image;
  - **Delete All** of scans leaves the captures.
- **`tests/smart_scan.rs`:**
  - a scan removes an empty scan folder as it ends;
  - the gallery won't delete the scan being run (409).
- **P31's test** of an unknown card type now uses `Sequence`, since `Scan` is known.

**Mutation checks:** I broke each of 10 behaviours. A test failed for 9 of them:

- **purging:** purge removing scans with images; no purge as a scan ends;
- **deleting:** deleting the scan being run; names leaving `scans/`;
- **the ZIP:** without the log; any name allowed;
- **the card:** a cancellation shown as an error; no thumbnail; no ZIP button.

At first, "any name allowed" for the ZIP survived. `../captures` failed anyway, since that folder has no `images/`, so the test now also tries `../scans/stopped`, which leads back into a real scan.

The 10th, the images left unsorted, can't fail on Windows, since NTFS lists a folder's files in name order. The sort stays, for other file systems.

## Follow-ups and known gaps

- **Stitching** (P39) switches a scan's card to the deep-zoom viewer, and adds the mosaic's downloads and a Stitch action.
- **The ZIP is built in memory.** A large scan should be written to a file, which `data::zip` allows.
- **Untested:** `purge_empty_scans` sparing the scan being run, before its first image.
- **No bulk actions yet:** "Purge empty scans" and "Stitch all scans" could join the gallery's sidebar (P39).
