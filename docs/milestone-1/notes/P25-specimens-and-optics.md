# P25: Specimens II and optical effects

- Status: In review
- Pull request: (added once opened)
- ADRs: none new; it serves its settings as ADR-0024 describes
- Spec: [phases.md#p25](../phases.md#p25)

## Summary

The simulated microscope now has realistic samples and imperfect optics, all switchable while it runs.

- **Two new samples:**
  - **H&E-like tissue:** pink stroma streaked with fibres, purple nuclei, and gaps, with a setting for how much of the slide it covers;
  - **a picture from the data folder:** PNG, JPEG or TIFF, at a given scale, averaged at low magnification.
- **The optics' imperfections,** all off until set:
  - a focal surface that tilts and waves, so focus changes across the slide;
  - vignetting;
  - uneven lighting;
  - exposure and gain that brighten the frame, with gain brightening the noise too.
- **The camera's settings panel,** `GET /camera/manual_camera_settings`, describes all of these (ADR-0024). The web app can now fetch and draw a Thing's panel (`ServerPanel`).

## What was built

| Where | What |
|---|---|
| `crates/microscope-sim/src/tissue.rs` | `TissueSpecimen` and `TissueConfig` |
| `crates/microscope-sim/src/slide_image.rs` | `ImageSpecimen`: a picture, its mip levels and its bounds |
| `crates/microscope-sim/src/noise.rs` | Value noise and fractal noise that leaves out what a pixel can't show |
| `crates/microscope-sim/src/render.rs` | `Effects` and `FocalSurface`, applied in `render_frame` |
| `crates/microscope-things/src/camera.rs` | `SampleType` and nine new settings, the picture's loading, and the `manual_camera_settings` endpoint |
| `web/src/server-ui/ServerPanel.vue` | Fetches a Thing's panel and draws it, again on `refresh` |
| `web/src/views/dev/ComponentsView.vue` | The camera's live panel in the components gallery |
| `Cargo.toml` | The `image` crate's `tiff` feature |
| `web/src/api/generated/` | Regenerated: the camera's new settings and endpoint |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5000
npm --prefix web run dev
```

Open `http://localhost:5173/#/dev/components`, scroll to "Server-described UI", and use the camera's settings panel below the test panel:

- **Sample → Tissue,** then look at the View tab at 40× and at 10× (the Objective control).
- **A picture:** put one in `.microscope/data/` (for example `.microscope/data/pictures/slide.jpg`), set "Picture, in the data folder" to `pictures/slide.jpg` and the scale, then choose Sample → Picture.
- **Optics:**
  - raise Vignetting and Uneven lighting to darken the corners;
  - set a focal tilt, then move the stage a millimetre to see the focus change.

These are settings, so they stay changed; put them back afterwards, to Blobs and zeros.

## Design notes and deviations from the plan

- **The focal surface is taken at the middle of the frame:** a frame is blurred evenly, by how far the stage is from the surface there. Moving across the slide changes the focus, which is what autofocus and scanning meet. Blur that varies within a frame would cost a blur per region of every frame.
- **Tissue is sampled on a grid fixed to the slide:**
  - **The grid:** the slowly varying mask (where the tissue is) every 4 µm, and the fibres every 0.5 µm, or one and a half pixels at low magnification. Values between are interpolated.
  - **Drawing in parts:** the grid is fixed to the slide, so a region drawn in two parts still matches it drawn whole.
  - **Speed:** a preview frame takes 30 ms at 40× and 46 ms at 4× and 10×, in a release build, against 72–90 ms when every pixel was computed. Blobs take 22 ms.
  - **A bug the benchmark found:** rounding could put a pixel in the grid cell after the last, and the index went out of bounds. Cells are now found exactly as the grid's edges are.
- **Coverage is a quantile:** each tissue sample measures where its mask's quantile at 1 − coverage falls, from 4,096 points, so the coverage setting gives about that share of tissue.
- **Pictures:**
  - **Where:** they come from the data folder only, through `data::resolve`, which refuses paths that leave it.
  - **Loading:** a picture is loaded once for each file and scale.
  - **A bad file:** one that can't be loaded leaves the slide empty and logs a warning.
- **Level of detail:** fractal noise leaves out layers finer than two pixels, and pictures use the mip level whose pixels are the largest no larger than the frame's.
- **Exposure and gain moved into the renderer:**
  - **Before,** the camera scaled finished frames.
  - **Now** the renderer scales the light before the noise, and multiplies the noise by the gain. So gain brightens the noise too, as on a real sensor.
  - **At the defaults,** frames are unchanged.
- **The settings panel is an endpoint,** `manual_camera_settings`, as the plan names it.
  - **Not a property:** OpenFlexure's is a property, but our model has no WoT schema (ADR-0024).
  - **Name checks:** the camera keeps a weak reference to its own registry entry, so the panel's controls are checked against its settings without a reference cycle.
- **`repeating`** applies to the blobs only; tissue and pictures end at their edges.
- **TIFF** needs the `image` crate's `tiff` feature, which brings six crates. All are MIT or Apache-2.0, and each version is at least five months old:
  - `tiff` 0.11.3;
  - `fax` 0.2.7;
  - `weezl` 0.1.12;
  - `half` 2.7.1;
  - `crunchy` 0.2.4;
  - `quick-error` 2.0.1.
- **Checked in a browser:**
  - the camera's panel in the gallery, switching to tissue through it;
  - tissue at 40× and 10×;
  - vignetting with uneven lighting at 10×.
- **Size:** 1,440 changed lines of hand-written code (about 590 of them tests) against L's budget of 900, above my estimate of 1,150–1,300. The owner chose one pull request rather than a split.

## Tests

- **`noise` unit tests** (2): value noise is continuous, across cell corners too; fractal noise drops what pixels are too coarse to show.
- **`tests/specimens.rs` in `microscope-sim`** (8 tests, and a benchmark):
  - **tissue:**
    - it draws the same wherever it's drawn from;
    - coverage 0.3 leaves about 70% of the slide empty and 0.8 about 20%;
    - coverage 0 leaves no tissue, and no nuclei, and there's nothing beyond the section;
    - it has pink stroma and purple nuclei;
  - **a picture:**
    - it's sampled exactly at its own scale, with empty slide around it;
    - coarser pixels show the average of what they cover, including every fourth column lit;
  - **files:** PNG, TIFF and JPEG load;
  - **the focal surface:**
    - a tilt in x or y blurs the slide 1 mm along, and refocusing there sharpens it;
    - the waviness stays within its height;
  - **lighting:** vignetting darkens the corners, and uneven lighting the bottom right, by the share set;
  - **exposure and gain:** half the exposure halves the brightness, and a quarter of it at four times the gain restores it, with four times the noise; too much exposure saturates.
- **`tests/samples.rs` in `microscope-things`** (3 tests), against the camera:
  - **switching:** the sample switches while it runs, to tissue and to a picture. Changing the picture's scale reloads it, and a missing file leaves the slide empty.
  - **the optics:** vignetting darkens the frame's corners, and a focal tilt blurs the view after a 1 mm move;
  - **the panel:** every control names one of the camera's settings, and none is broken.
- **`ServerPanel`** (2 tests): it fetches the camera's panel, and again when its action asks for a refresh; a missing panel says so.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in the simulator:**
  - the level of detail;
  - the coverage quantile;
  - nuclei only in tissue;
  - the stain;
  - the section's bounds;
  - halving for mip levels;
  - the picture's bounds;
  - choosing the mip level;
  - the focal surface's sign, its tilt in y, and its waviness;
  - vignetting;
  - uneven lighting;
  - gain in the noise;
  - exposure;
- **in the camera:**
  - tissue;
  - reloading the picture for a new scale;
  - vignetting;
  - the focal tilt;
  - a control dropped from the panel;
- **in the web:**
  - refetching the panel;
  - reporting a missing panel.

## Follow-ups and known gaps

- **P27** adds the camera's calibration actions and background detection. Lens-shading calibration would correct the vignetting and uneven lighting added here.
- **P28's Settings tab** will show `manual_camera_settings` through `ServerPanel`.
- **The panel shows every sample's settings** whatever the sample. A panel that changes with the sample type could use `refresh`, once a setting's control can ask for one.
- **Blur varies only between frames,** not within one (see above).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its idea:** a simulated camera whose manual settings are described for the settings panel, read for behaviour only.
  - **Our own:** the tissue, the picture specimen, the optics model and the noise.
