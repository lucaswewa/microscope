# P23b: Click-to-move

- Status: In review
- Pull request: (added once opened)
- ADRs: none new
- Spec: [phases.md#p23b](../phases.md#p23b), split from P23 (plan Appendix D)

## Summary

On the Control tab, double-clicking the live image now moves the stage, so that the point double-clicked comes to the centre. It goes through the camera–stage mapping that P23 calibrates.

- **Contain-fit:** the click is placed on the image as it's drawn, letterboxed; the letterboxing itself does nothing.
- **Scaling:** the point is scaled to the frames the mapping was calibrated with, whatever size the stream is now.
- **The move lock:** while one move runs, further double-clicks are ignored.
- **Not calibrated yet:** a double-click says to calibrate the mapping first.

## What was built

| Where | What |
|---|---|
| `web/src/live/contain.ts` | `containPoint`: where a point in a box falls on a contain-fit image, as shares of the image's size from its centre |
| `web/src/live/LiveImage.vue` | A `doubleClick` event with that point, once a frame has been drawn |
| `web/src/control/clickToMove.ts` | `useClickToMove`: scales the point to the calibration's `image_resolution`, runs `move_in_image_coordinates`, and holds the move lock |
| `web/src/views/ControlPage.vue` | Double-clicking the live image moves there |
| `web/src/api/things/index.ts` | `camera_stage_mapping` joins the typed Things |
| `web/tests/unit/`, `web/tests/e2e/control.spec.ts` | The tests below |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

Open `http://127.0.0.1:5090/#/control` and double-click a blob: a note says to calibrate first. Calibrate the mapping as P23's notes say, `Invoke-Action camera_stage_mapping/calibrate_xy` (about 20 s), then double-click a blob again: it comes to the centre of the image. Double-clicking the letterboxing does nothing.

## Design notes and deviations from the plan

- **The click as shares of the image:** `LiveImage` reports where the image was double-clicked as shares of its width and height from its centre, from −0.5 to 0.5. It doesn't use the stream's pixels.
  - **Why:** the Control tab multiplies the shares by the `image_resolution` the mapping was calibrated at. Calibrating at 820 × 616 and then streaming at full resolution still moves the right distance.
  - **Unlike OpenFlexure,** which sends the stream's pixels.
- **The move lock** ignores double-clicks while a click-move runs, as OpenFlexure's does. It doesn't block the d-pad or the keys, whose jogs replace whatever move is in progress (P20).
- **Before a frame,** a double-click does nothing: there's no image to place it on yet.
- **The end-to-end test calibrates the mapping** on the server it runs against. Locally that server uses the checkout's `.microscope` folder, as the other end-to-end tests do, so the calibration is saved in your settings. It's the true calibration for the camera as your settings have it, but it replaces yours, and its data joins `.microscope/data/calibration/`.
- **Checked in a browser:**
  - **the note** when not calibrated, in both themes;
  - **after calibrating,** a double-clicked blob comes to the image's centre.
- **Size:** 227 changed lines of hand-written code (about 135 of them tests) against S's budget of 300.

## Tests

- **`containPoint`** (2 unit tests): on images letterboxed above and below, and left and right; the image's edges are at ±0.5, and the letterboxing gives nothing.
- **`LiveImage`** (1 test): a double-click is reported as shares of the image, allowing for where the canvas is on the page; nothing before a frame, or on the letterboxing.
- **The Control tab** (2 tests):
  - **moving:** a double-click runs `move_in_image_coordinates` in the calibration's pixels (0.25 of 820 is 205), and a second double-click during the move is ignored, while one after it isn't;
  - **not calibrated:** nothing moves, and the note says to calibrate first.
- **End to end** (`control.spec.ts`): after calibrating through the API, a double-click 100 × 50 CSS pixels from the image's centre moves the stage by the mapping's matrix, within a step. It takes about 30 s, mostly calibrating.

**Mutation checks:** I broke each of the following, and a test failed every time:

- waiting for a frame;
- allowing for where the canvas is;
- the letterboxing check;
- the contain-fit scale;
- the move lock;
- releasing the lock after a move;
- scaling to the calibration's resolution;
- the note when not calibrated.

## Follow-ups and known gaps

- **Calibrating from the app:** the Settings tab (P28) and the calibration wizard (P29) will calibrate the mapping. Until then, it's calibrated through the API.
- **The end-to-end server's settings:** it could use a folder of its own, so that running the tests locally leaves `.microscope` alone. That's a change to the test set-up, outside this phase.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Its behaviour (double-click, contain-fit scaling and a move lock) was read from its web app; this is written from scratch, and scales to the calibration's resolution, which OpenFlexure doesn't.
