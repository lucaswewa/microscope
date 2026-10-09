# P05: OpenFlexure reference screenshots

- Status: Done
- Pull request: [#6](https://github.com/lucaswewa/microscope/pull/6)
- ADRs: none
- Spec: [phases.md#p05](../phases.md#p05)

## Summary

A repeatable way to capture OpenFlexure's interface, and this app's, in both themes at two window sizes, for side-by-side visual review. [`docs/reference/README.md`](../../reference/README.md) explains how to run OpenFlexure's simulator. `web/tests/reference/capture.ts` prepares it through its API, captures 16 states × 2 themes × 2 sizes, and measures OpenFlexure's layout and colours. The screenshots and measurements stay out of git (ADR-0002). This page records the measurements P06 builds on.

## What was built

| Where | What |
|---|---|
| `docs/reference/README.md` | Setting up OpenFlexure's simulator (uv, Python 3.12, its web app), running it, capturing, and what to look for in review |
| `web/tests/reference/capture.ts` | The capture script, run with Node (type stripping) and Playwright's Chromium: `npm run capture:reference -- --target ofm\|microscope --url …` |
| `web/package.json` | The `capture:reference` script |
| `web/tsconfig.node.json` | Type-checks `tests/reference/` too |
| `.gitignore` | Also ignores `docs/reference/compare.html` |
| `docs/README.md` | Links the reference guide |

**States captured** (file names `<state>.<theme>.<width>x<height>.png`):

- the tabs: `view`, `control`, `slide-scan`, `slide-scan-running`, `sequence` (OpenFlexure's Timelapse tab), `gallery`, `logging`, `about`, `power`;
- the Settings sections: `settings-display`, `settings-stage-control`, `settings-camera`, `settings-stage`, `settings-mapping`;
- the calibration wizard: `calibration-wizard` (the welcome page) and `calibration-wizard-from-settings`.

Each comes in `light` and `dark`, at 1280×800 and 1920×1080: 64 screenshots per app.

## How to try it

1. Set up and run OpenFlexure's simulator on port 5095, as [`docs/reference/README.md`](../../reference/README.md) describes.
2. From `web/`: `npm run capture:reference -- --target ofm --url http://127.0.0.1:5095/`. The first run on an uncalibrated microscope takes about ten minutes, including calibration and a short scan.
3. With `npm run dev -- --port 5195` running: `npm run capture:reference -- --target microscope --url http://localhost:5195/`.
4. Open `docs/reference/compare.html`.

## Measurements

Measured by the capture script in OpenFlexure's rendered page at 1280×800 (`docs/reference/ofm/measurements.json`; Control, Slide Scan and Settings → Camera), and rounded. These are observations for comparison. P06 chooses this app's own tokens, taking proportions and colour roles from them, not the exact values.

**Layout**

| Element | Light | Dark |
|---|---|---|
| Rail | 85 px wide, plus a 1 px right border; a 10% grey wash over the page | the same, over the dark background |
| Rail item | 85 × 67 px; padding 10 px 8 px; 24 px icon above a 14 px label | the same |
| Inactive rail item | grey text (#666) | white text |
| Active rail item | white on the accent (#C32280) | white on the accent (#C32280) |
| Control pane (Control tab) | about 187 px wide outside: about 165 px of content, 6 px left padding, 1 px right border, and a scrollbar gutter | the same |
| Control pane (Slide Scan) | about 401 px wide outside | the same |
| Content area | the rest (1194 px at 1280 wide), with an almost invisible grey wash | the same |

**Controls**

| Element | Light | Dark |
|---|---|---|
| Buttons | 40 px tall (default style 38 px); padding 12 px 8 px; 3 px radius; 1 px accent border; accent text on a 10% grey wash | 40 px; lighter accent (#E151A5) text and border, on black |
| D-pad buttons | 40 × 40 px | the same |
| Selects | 40 px tall; square corners; white with a #D5D5D5 border | 10% white on dark, with a 20% white border |
| Text inputs (small) | 30 px tall; padding 0 8 px; square corners | as selects |

**Type and colour**

| Element | Light | Dark |
|---|---|---|
| Body | 14 px / 21 px; ProximaNova, falling back to Segoe UI on Windows; black | 14 px / 21 px; white at 80% |
| Page background | white | #222 (charcoal) |
| Section headings (h3) | 24 px, weight 300, accent | 24 px, weight 300, white |
| Large headings (h2) | 30 px, weight 300 | the same, white |
| Accordion titles | 14 px, weight 500, #333 | 14 px, weight 500, white at 80% |
| Accent | #C32280 (magenta) | #C32280 for the active tab; #E151A5 for text and borders |

**Takeaways for P06:**

- **Compact density:** 14 px text, 40 px controls (30 px small), 3 px button radius, square inputs.
- **An 85 px icon-over-label rail** whose active item is a solid accent block.
- **Narrow control panes** (about 180 px on Control, about 400 px on Slide Scan) beside a dominant live image.
- **Light headings** (weight 300).
- **One magenta accent,** lightened for text on dark backgrounds.
- **Neutral surfaces from translucent greys:** white in light, charcoal #222 in dark.

## Design notes and deviations from the plan

- **Themes come from the system colour scheme,** which Playwright emulates, not from OpenFlexure's stored preference as the spec said. Both apps follow the system by default, so one switch works for both, with no dependence on OpenFlexure's storage format.
- **The wizard is two states.** `calibration-wizard` is the welcome page, which opens by itself only while calibrations are missing. `calibration-wizard-from-settings` is the wizard as Settings launches it. A re-run on a calibrated microscope keeps the earlier welcome captures instead of overwriting them.
- **OpenFlexure's simulator config has no C-Chip workflow or stage measurement tools,** so they aren't captured.
- **Preparing the simulator through its API:** `camera/full_auto_calibrate`, `camera_stage_mapping/calibrate_xy`, three `camera/capture`s at least a second apart (OpenFlexure names captures to the second, and two in one second collide on a database UNIQUE constraint), then a 3×2 snake scan.
- **Setup choices:**
  - Python 3.12 is managed by uv, because Pillow 10.4, which OpenFlexure pins, has no Windows wheels for 3.13.
  - Packages are installed with uv's `--exclude-newer` and pip's `--uploaded-prior-to` two weeks back, matching ADR-0009's rule.
  - OpenFlexure's web app builds with Node 24, although it asks for 26.
  - Everything the setup creates in the OpenFlexure clone is git-ignored there; its `git status` stays clean.
- **`sentry-sdk`** comes with OpenFlexure's dependencies, through FastAPI's cloud CLI. Nothing in OpenFlexure, LabThings or the stitching package imports it, so the simulator reports nothing.
- **Vite's development server answers only on `localhost`** (IPv6 `::1` on Windows), so captures of this app use `http://localhost:…`.
- **The script moves the pointer to a corner** before each screenshot, so no tooltip or hover state shows.
- **Measurements use the first visible match** of each selector. OpenFlexure keeps every tab in the page and hides the inactive ones, so the first match is often hidden and measures 0 × 0.
- **Scans need OpenFlexure's environment on `PATH`.** OpenFlexure stitches by running `openflexure-stitch` as a subprocess. Started without its `.venv\Scripts` on `PATH`, a scan fails after a couple of images with "The system cannot find the file specified". The README says to activate the environment first.
- **Captures of this app show blank pages** for now: the routes arrive with P07 and later phases.

## Tests

The script isn't run in CI. It is type-checked and linted with the rest of the web app. Runs, in order:

1. **OpenFlexure, uncalibrated.** It captured the welcome page and calibrated. That run exposed the capture-naming collision and the stitching `PATH` problem, both fixed.
2. **OpenFlexure, fresh simulator** (its `openflexure/` data folder deleted, environment on `PATH`). The welcome page was captured again, then the run was stopped to fix the measurements.
3. **OpenFlexure, final** (calibrated). All 64 screenshots: the welcome pages from run 2, the rest new. The snake scan completed with stitching, and `measurements.json` and `compare.html` were written.
4. **This app,** through Vite on port 5195: 64 screenshots of placeholder routes.

Spot checks: the welcome page in light and dark, Control in light and dark (no tooltip), Settings → Camera in dark, Slide Scan while scanning ("Cancel Scanning" and a live image mid-autofocus), and the Gallery with three captures and a scan.

## Follow-ups and known gaps

- P06 builds the theme tokens from the measurements above. P07 and P47 compare against the screenshots.
- Phases that add states, such as P29's calibration wizard, update the routes in the script.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The screenshots stay out of git. The measurements are observations of the rendered page (sizes and colours), recorded for comparison. P06 chooses this app's own tokens.
