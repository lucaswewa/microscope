# ADR-0036: Image overlay layers

- Status: Accepted
- Date: 2026-10-15
- Phase: P53

## Context

- **The owner asked** for each image's outline, numbered, over a scan's stitched image, which can be turned on and off.
- **More will follow, on the same images:** recognised objects, a scale bar, measurements.
- **The images are shown two ways:**
  - the Slide Scan tab's preview is a plain `<img>`, fitted to its box;
  - the gallery's viewer is OpenSeadragon (ADR-0027), which pans and zooms.
- **Stitching knows where each image went,** from the stage positions and the camera–stage mapping (ADR-0030), but it didn't record it.
- **Annotations must stay readable** at any zoom, and on light and dark images.

## Decision

- **Stitching records where each image went:**
  - `stitched/tiles.json` for the mosaic;
  - `smart_scan`'s `latest_preview_tiles.json` for the live preview, drawn with it.

  Both hold a `TileLayout`: the mosaic's size, and for each image its file name, its number in the scan and its rectangle, in the mosaic's pixels.
  - **The number** follows when the images were taken (their sidecars' `acquired`), not where they are.
  - **The preview's layout** is in the pixels of the mosaic it was shrunk from. The app scales it to the image it's shown, so the two needn't match in size.
- **A layer** (`web/src/overlay/`) has an id, a label, the size its shapes are measured in, and shapes:
  - rectangles, which can be dashed and labelled;
  - lines;
  - points;
  - labels.

  Any shape can have a title, shown on hover. Shapes are in the layer's own pixels, not the screen's.
- **One SVG over the image draws the layers shown.**
  - **Each shape is placed in screen pixels** from where the image is drawn (a `Placement`). So lines and labels keep their size at any zoom, and the SVG needs no scaling transform.
  - **Each is drawn over a dark halo,** in the annotation colours, which are the same in both themes because the images are.
  - **The pointer passes through to the image,** except on a shape's line or label, where its title shows.
- **Adapters say where the image is:**
  - `OverlaidImage` measures a contain-fitted `<img>`, again whenever its box changes size;
  - P53b adds one for OpenSeadragon's viewport.
- **A Layers menu** has a choice for each layer, ticked while it's shown, from the shared menu's new checkbox items. The choices are kept in the browser by layer id, and a layer not yet chosen is shown.
- **The first layer, Image outlines,** is a dashed rectangle around each image, labelled with its number, with its file name on hover.

## Consequences

- **A new layer** is a function from data to shapes, plus where its data comes from. The drawing, the adapters and the menu need no change.
- **Shapes are drawn with Vue,** so thousands of them, such as recognised cells over a big mosaic, would need a lighter renderer, or only the shapes in view. Hundreds are fine.
- **The preview's image and layout are fetched separately,** each for the same drawing time. Between two drawings they could briefly disagree, until the next.
- **Scans stitched before P53 have no `tiles.json`;** stitching them again writes it.
- **Shapes in image pixels** fit annotations that belong to the image, like outlines and detections. A scale bar belongs to the screen, so it will need screen-anchored shapes added to the model.

## Alternatives considered

- **OpenSeadragon's own overlays** (HTML elements in viewport coordinates): they only work in the deep-zoom viewer, not over a plain image, and one element per shape is heavier than SVG.
- **Drawing on a canvas:** it scales to many shapes, but titles on hover and testing what's drawn are harder. It remains an option inside the same layer model.
- **Drawing the outlines into the stitched image on the server:** they couldn't be turned off, or styled for the screen, and every new layer would mean stitching again.
- **Fixing the numbers by position (row and column):** a spiral or snake scan's numbers would then not follow the order the images were taken in, which is what the log and the files show.
