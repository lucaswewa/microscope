# ADR-0027: Deep-zoom viewer

- Status: Accepted
- Date: 2026-10-13
- Phase: P33

## Context

- **What the gallery shows:**
  - single images (captures, P26);
  - stitched scans, large enough to need tiles (P38);
  - sequences of images (timelapses, P41).
- **What viewing needs:**
  - smooth zooming and panning, on big mosaics too;
  - brightness, contrast and saturation;
  - full screen;
  - controls that follow the theme (ADR-0011).
- **The plan named OpenSeadragon** (BSD-3-Clause) for this.

## Decision

- **OpenSeadragon 6.1.1:** BSD-3-Clause, with no dependencies and its own type declarations.
  - **Loaded when first needed:** a dynamic `import()` puts it in its own chunk (348 KB, 87 KB compressed), so the app's first load doesn't carry it.
- **What it shows** comes from a gallery entry's `viewer` (ADR-0026), whose files the data route serves:
  - **`image`:** one image, `path`;
  - **`dzi`:** a deep-zoom image, its `.dzi` file at `path`. The tiles are beside it, at `<name>_files/<level>/<column>_<row>.<format>`, as the DZI format has them;
  - **`directory`:** images shown one at a time, `paths`, starting at `start`.
- **One wrapper,** `DeepZoom`, owns the OpenSeadragon viewer.
  - **Its life:** it's made when the wrapper mounts and destroyed when it unmounts.
  - **Sequences:** a `directory` is shown in OpenSeadragon's sequence mode.
  - **State:** the wrapper marks itself loading, shown or failed.
- **The app's own controls,** with OpenSeadragon's navigation and sequence controls turned off, so they follow the theme:
  - zoom in, zoom out, and show it all;
  - full screen;
  - a slider for a sequence, on a new `AppSlider` (Reka UI's Slider, ADR-0013);
  - sliders for brightness, contrast and saturation, from 0 to 200 %, with Reset.
- **The filters** are a CSS filter on the view, so they work with either of OpenSeadragon's drawers (WebGL or canvas). They change how it looks, not the files.
- **Full screen:** the dialog fills the window (a new `full` size of `AppDialog`), and the host fills the screen (ADR-0010).
  - **Leaving the screen** any other way, as Escape does, returns the dialog to its size.
  - **Closing the viewer** leaves full screen.
- **Another server's images:** OpenSeadragon asks for them with `crossOriginPolicy: 'Anonymous'`, and the data route allows cross-origin requests (ADR-0026). So a remote microscope's images can still be drawn with WebGL.
- **Tests:**
  - **Unit tests** use a fake OpenSeadragon, since the tests' DOM has no WebGL.
  - **End-to-end tests** use the real one, in Chromium, with a DZI fixture in `web/tests/fixtures/dzi/` (600 × 400 pixels, 256-pixel tiles, no overlap). Playwright serves it until P38 writes real pyramids.

## Consequences

- **Other formats:** other deep-zoom formats that OpenSeadragon reads, such as IIIF, would need only a new kind of `viewer`.
- **The filters are only on screen:** a filtered image can't be downloaded, and the filters reset each time the viewer opens.
- **The first View** loads the 348 KB chunk.
- **P38's DZI writer** must produce the layout above, which the fixture also follows.

## Alternatives considered

- **Our own canvas viewer.** It would have no dependency, but zooming, panning, tiles and gestures done well are a lot of work.
- **Leaflet,** with its plain coordinate system. It's BSD-2-Clause, but it's made for maps, and reading DZI needs a plugin.
- **OpenSeadragon's filtering plugin.** It changes the pixels themselves, but it's slower, and isn't kept up for version 6's WebGL drawer.
- **An `<img>` with CSS transforms.** It's simple, but it has no tiles, so it can't show a large mosaic.
