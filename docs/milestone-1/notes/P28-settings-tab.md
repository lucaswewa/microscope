# P28: Settings tab

- Status: Done
- Pull request: [#38](https://github.com/lucaswewa/microscope/pull/38)
- ADRs: none new
- Spec: [phases.md#p28](../phases.md#p28)

## Summary

The Settings tab is built, with OpenFlexure's sections and Connection. Its sections are listed in the control pane, and the chosen one shows beside them. The section is in the address, such as `/#/settings/camera`.

- **Application:**
  - **Display:** the theme, full screen, and turning the live image off;
  - **Stage control:** the navigation preferences (P20b);
  - **Connection:** the Connect screen (P11), staying in Settings.
- **Microscope:**
  - **Calibration wizard:** a button, disabled until P29;
  - **Camera:**
    - a mini stream;
    - the camera's calibration actions and manual settings;
    - its background detector's settings, all as the Things describe them (ADR-0024);
  - **Stage:** whether z is inverted, and Invert z;
  - **Camera–stage mapping:**
    - Calibrate, with a question first and the stream in the progress dialog;
    - the matrix, the scale and the field of view;
    - the calibration data, to download.
- **A reusable mini stream,** `MiniStream`, which the progress dialogs now use too.

## What was built

| Where | What |
|---|---|
| `web/src/views/SettingsPage.vue` | The page and its section list; the route now goes there, not to the placeholder |
| `web/src/settings/DisplaySettings.vue`, `StageSettings.vue`, `CameraSettings.vue`, `MappingSettings.vue` | The sections |
| `web/src/live/MiniStream.vue` | A small live image from the low-resolution stream |
| `web/src/host/types.ts`, `browser.ts` | `FullscreenControl`: `active`, `set` and `watch` |
| `web/src/connection/ConnectView.vue` | `embedded`: without its page frame, and staying put after connecting |
| `web/src/server-ui/ActionButton.vue` | Its dialog uses `MiniStream` |
| `web/tests/unit/settings/`, `host/browser.spec.ts`, `app.spec.ts` | The tests below |
| `web/tests/visual/…/settings-camera-*.png` | The baselines, now of the Camera section |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Open `http://127.0.0.1:5090/#/settings`:

- **Display:** choose the theme, or try Full screen.
- **Camera:** the mini stream, then the calibration actions and settings, and the background detector's settings.
- **Stage:** press Invert z; the text changes, and the d-pad's z buttons swap. Press it again to undo.
- **Camera–stage mapping:** press Calibrate and confirm. A dialog shows the stream for about 20 seconds. Then the details appear, and "Download the calibration data" saves the JSON.

## Design notes and deviations from the plan

- **Sections are routes:** `/#/settings/<section>`, as the placeholder's route already was, so a section can be linked to and survives a reload. An unknown section shows Display.
- **The calibration wizard's button** sits under "Microscope", disabled, until P29 wires it up. It's labelled "Calibration wizard": "Launch calibration wizard" didn't fit the narrow pane, as the first screenshot showed.
- **Full screen goes through the host** (ADR-0010).
  - **The browser:** the browser host uses the Fullscreen API.
  - **The desktop app (later):** the Tauri host will fill the window.
  - **The toggle follows changes from elsewhere,** such as Esc leaving full screen.
- **Connection reuses the Connect screen,** embedded, rather than a second copy. Connecting from Settings stays in Settings, where the Connect screen goes to View.
- **Camera is all server-described:** the calibration actions (primary, then secondary), the manual settings and the detector's settings are each a `ServerPanel` (P25). So a milestone-2 camera brings its own.
  - **The detector** is the one `background_detector_name` names, followed live.
  - **No chooser yet:** there's no choice of detector in the UI; the setting can be changed through the API.
- **Stage:**
  - **The inversion:** it shows the inversion of z from the stage's `axis_inverted`, followed live, and Invert z runs `invert_axis_direction` with `{"axis": "z"}`, as an `ActionButton` built in the web app.
  - **P30's tools** go in the section's default slot.
- **The mapping's details:**
  - **The matrix,** to three decimals: values that round to zero show as 0.000, not −0.000, as the first browser check did;
  - **the scale,** as √|det A| steps per pixel;
  - **the field of view in steps:** the lengths of A's columns times the calibration's frame size;
  - **the frame size and time of the calibration.**

  The download is `last_calibration`, saved as `camera_stage_mapping_<time>.json`.
- **Checked in a browser, in both themes:**
  - every section;
  - a calibration from the Camera–stage mapping section, with the dialog and stream, then the details;
  - a downloaded `camera_stage_mapping_2026-10-11T21-13-29Z.json`.
- **Size:** 767 changed lines of hand-written code (about 250 of them tests) against L's budget of 900. My estimate was 1,000–1,200; reusing the Connect screen, the navigation preferences, `ServerPanel` and `ActionButton` saved most of the difference.

## Tests

- **`SettingsPage.spec.ts`** (7 tests), the whole app against a fake server:
  - **sections:** the six are listed, Display without a section and for an unknown one, and links switch the section and mark it current;
  - **Display:**
    - the theme is saved;
    - full screen goes to the host;
    - the live image's preference is saved;
  - **Connection:** the Connect screen is shown, and connecting stays in Settings;
  - **Camera:** the mini stream, and the four panels (primary and secondary actions, manual settings, the detector's settings);
  - **Stage:** "isn't inverted", and Invert z sends `{"axis": "z"}`;
  - **the mapping:**
    - the matrix (a tiny negative shows as 0.000), the scale and the field of view;
    - the download's name and contents;
    - "It isn't calibrated yet." without a matrix.
- **The browser host** (1 new test): full screen is entered and left, only when needed, and changes are reported until the watch stops.
- **`app.spec.ts`:** Settings is no longer a placeholder, and opens on the section in the address.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **the page:**
  - the default section;
  - marking the current section;
- **Display:**
  - the theme;
  - full screen;
  - the live image's preference;
- **the embedded Connect screen staying put;**
- **Camera:** the detector's panel;
- **Stage:**
  - Invert z's input;
  - the inversion text;
- **the mapping:**
  - the scale;
  - the field of view;
  - the download's name;
  - not calibrated;
  - rounded zeros;
- **the browser host:**
  - entering full screen only when needed;
  - stopping the watch.

Two survived at first, marking the current section and the scale. The tests now check the mark at `/#/settings`, and use a matrix whose scale differs from its first entry.

## Follow-ups and known gaps

- **P29** wires the calibration wizard's button.
- **P30** adds the stage measurement tools to the Stage section.
- **Choosing the background detector** in the UI: the setting has no list of names in its schema yet. A small panel on the camera could offer them.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its behaviour:** the sections and what each holds, read from its source.
  - **Our own:** the layout, the text and the code.
