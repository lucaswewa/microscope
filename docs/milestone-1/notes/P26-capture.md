# P26: Capture to the data folder

- Status: In review
- Pull request: [#36](https://github.com/lucaswewa/microscope/pull/36)
- ADRs: [ADR-0025](../../adr/0025-data-folder-layout-and-capture-metadata.md)
- Spec: [phases.md#p26](../phases.md#p26)

## Summary

The microscope can now capture images: saved to the data folder with a record of how they were taken, or downloaded.

- **`camera.capture(capture_mode, retain_image)`:**
  - **The size:** a full-resolution image (`standard`), or the stream's size (`quick`).
  - **Kept:** with `retain_image`, it's saved to the data folder's `captures/` as a JPEG, beside a JSON sidecar of how it was taken.
  - **Either way,** the image comes back as a Blob to download.
- **The sidecar** records:
  - the time;
  - the stage position, in steps and in µm;
  - the objective, exposure and gain;
  - the camera–stage mapping, once calibrated;
  - the simulator's state.
- **Capture in the Control tab:**
  - **On capture:** "Save to gallery" or "Download", a choice the browser keeps;
  - **Capture:** a button, and the `c` key;
  - **the flash:** the live image flashes as an image is taken.

ADR-0025 sets the data folder's layout and the capture naming scheme, as the gallery (P31) will read them.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/capture.rs` | `Sidecar` and its parts, `save` (unique names, never overwriting), `jpeg`, and `mapping`, which finds a calibrated camera–stage mapping |
| `crates/microscope-things/src/camera.rs` | The `capture` action and `CaptureMode`; the simulator's state in each capture's metadata |
| `crates/microscope-things/src/hardware/camera.rs` | `CaptureMetadata.simulation` |
| `crates/microscope-things/src/mapping.rs` | `CameraStageMapping::calibrated` |
| `crates/microscope-things/tests/capture.rs` | The tests below |
| `web/src/control/CaptureControl.vue`, `web/src/views/ControlPage.vue` | Capture in the control pane |
| `web/tests/visual/…/control-*.png` | The Control tab's baselines, with the new section |
| `docs/adr/0025-data-folder-layout-and-capture-metadata.md` | The layout, the names and the sidecar |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

(after `npm --prefix web run build`). Open `http://127.0.0.1:5090/#/control`:

- **Capture** with "Save to gallery": a toast says it's saved. `.microscope/data/captures/` now holds `capture-<time>.jpeg` and its `.json` sidecar.
- **Download:** choose "Download" and press `c`; the browser downloads the image, and nothing is added to `captures/`.

To read a sidecar: `Get-Content .microscope/data/captures/*.json | Select-Object -First 40`.

## Design notes and deviations from the plan

ADR-0025 records the layout, names and sidecar. The details:

- **Names are UTC to the second:** for example `capture-2026-10-11T17-54-53Z.jpeg`.
  - **Same-second captures:** a second capture in the same second becomes `-2`, then `-3`.
  - **No overwriting:** the image file is created only if it doesn't exist (`create_new`), so two captures can never overwrite each other, even at once.
- **Kept captures return the saved file:** a capture that's kept returns the saved file as its Blob, so the download is the same image. One that isn't kept is encoded in memory, and nothing is written.
- **The camera finds the mapping itself:** it looks up a camera–stage mapping Thing by its type, so it needs no slot for it. Without a mapping Thing, or before calibrating, the sidecar's `camera_stage_mapping` is null.
- **Saving lives in `capture.rs`,** not the camera, so a milestone-2 camera saves the same layout. The simulator's state reaches the sidecar through the new `CaptureMetadata.simulation`, which is null for real cameras.
- **`capture_to_memory`** (P17b) is unchanged. The camera's private `capture` became `take(sensor)`, since the new action has its name.
- **The UI follows OpenFlexure's behaviour:** an "On capture" choice of the gallery or a download, as OpenFlexure's capture control offers.
  - **Downloads are named** `capture-<time>Z.jpeg`, like the saved files.
  - **The flash** comes when the capture starts, as the image is taken.
  - **The shortcut** `c` is listed under "Camera" in the `?` help.
- **The control pane is taller:** at 1280 × 800 it now reaches the bottom of the window, so on smaller screens it scrolls. The visual baselines are updated, and checked in both themes.
- **Checked in a browser,** against my own server:
  - a capture saved to the gallery, with its sidecar (including the mapping the end-to-end run had calibrated);
  - a download with `c`: a 405 KB JPEG, with nothing added to `captures/`.
- **Size:** 636 changed lines of hand-written code (about 255 of them tests) against M's budget of 600, a little over my estimate of 550–600.

## Tests

- **`capture` unit test** (1): names are in UTC to the second, whatever the time's offset.
- **`tests/capture.rs`** (3 tests):
  - **a kept capture:**
    - the JPEG and its sidecar are saved, and the Blob downloads the saved bytes;
    - the sidecar records the format, the image's name and size, and µm per pixel;
    - it records the stage in steps and µm, the camera's settings, and the simulator's sample and true position;
    - it records no mapping, then the calibrated matrix and resolution after `calibrate_xy`;
  - **a quick capture that isn't kept** saves nothing, and downloads at 820 × 616;
  - **path safety:** three captures in the same second get the names `…Z.jpeg`, `…Z-2.jpeg` and `…Z-3.jpeg` in `captures/`, each with its own sidecar.
- **The Control tab** (1 new test):
  - Capture saves to the gallery, and the image flashes;
  - after choosing Download, `c` downloads the file under a capture name;
  - the choice is kept.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in Rust:**
  - creating files only if new;
  - UTC names;
  - µm from steps;
  - writing the sidecar;
  - finding the mapping;
  - not keeping;
  - the quick mode's size;
  - the simulator's state;
- **in the web:**
  - the gallery-or-download choice;
  - the flash;
  - the `c` key;
  - keeping the choice;
  - saving the download.

## Follow-ups and known gaps

- **P31, the gallery,** lists `captures/` from the sidecars, and shows their details in local time.
- **Capture formats:** only JPEG is offered. OpenFlexure's `image_format` also allows others; PNG could be added if anyone needs lossless captures.
- **The local end-to-end runs** still share `.microscope` (P23b's notes). This phase's tests write only to temporary folders.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its behaviour,** read from its source: the capture action's parameters, and the "On capture" choice.
  - **Our own:** the sidecar, the names and the code.
