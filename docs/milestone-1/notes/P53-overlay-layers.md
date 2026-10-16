# P53: Overlay layers, with image outlines on stitched scans

- Status: Done
- Pull request: [#66](https://github.com/lucaswewa/microscope/pull/66)
- ADRs: [ADR-0036](../../adr/0036-image-overlay-layers.md)
- Spec: [phases.md#p53](../phases.md#p53)

## Summary

Information drawn over an image, in layers that can each be turned on and off. The first layer outlines each image of a scan in its stitching preview.

- **Stitching records where each image went:**
  - `stitched/tiles.json` for the final mosaic;
  - `smart_scan`'s `latest_preview_tiles.json` for the live preview.

  Each holds the mosaic's size, and each image's file name, number and rectangle. Images are numbered in the order they were taken.
- **Overlay layers in the web app** (`web/src/overlay/`): rectangles, lines, points and labels, in an image's pixels.
  - **Drawn** in one SVG over the image, with lines and labels the same size on screen at any zoom, over a dark halo so they show on light and dark images.
  - **Placed** over a fitted `<img>` by an adapter; P53b adds one for the deep-zoom viewer.
- **A Layers menu** turns each layer on and off, and the browser remembers the choice.
- **Image outlines,** the first layer: a dashed rectangle around each image, labelled with its number, with its file name on hover. It's shown over the Slide Scan tab's preview, during a scan and after it.
- **The shared menu** gains checkbox items, which the Layers menu uses.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/stitch.rs` | `TileLayout` and `PlacedImage`; `stitched/tiles.json`; the preview's layout with its JPEG |
| `crates/microscope-things/src/smart_scan.rs` | `latest_preview_tiles.json`, beside `latest_preview_stitch.jpg` |
| `web/src/overlay/types.ts` | Shapes, layers and placements |
| `web/src/overlay/OverlayLayers.vue` | The SVG that draws the layers shown |
| `web/src/overlay/OverlaidImage.vue` | A fitted image with layers over it |
| `web/src/overlay/layers.ts` | Which layers are shown, kept in the browser |
| `web/src/overlay/LayersMenu.vue` | The Layers menu |
| `web/src/overlay/imageOutlines.ts` | The Image outlines layer, from a `TileLayout` |
| `web/src/ui/AppMenu.vue` | Checkbox items |
| `web/src/scan/SlideScanPage.vue` | The preview's layout, read with each new preview; the outlines and the Layers menu |
| `web/src/theme/tokens.css`, `tokens.ts` | The annotation colours and widths, listed for the token gallery |

**`tiles.json`,** from the end-to-end test's 2 × 1 scan, with 35% overlap:

```json
{
  "size": [1353, 616],
  "tiles": [
    { "image": "img_115_58_-2.jpeg", "number": 1, "x": 0, "y": 0, "width": 820, "height": 616 },
    { "image": "img_711_58_-2.jpeg", "number": 2, "x": 533, "y": 0, "width": 820, "height": 616 }
  ]
}
```

## How to try it

With a server running and the app open (for example `cargo run -p microscope-server -- -c configs/simulation.json`, then `npm --prefix web run dev`), and the camera–stage mapping calibrated:

1. In **Slide Scan**, choose **Snake scan**, set 3 columns and 2 rows in **Scan settings**, and press **Start Smart Scan**.
2. As the preview fills, each image has a dashed amber outline, numbered in the order it was taken: 1–3 along the first row, 4–6 back along the second. Rest the pointer on a number or a line to see the image's file name.
3. Open **Layers**, above the preview, and untick **Image outlines**: they go, and stay off when you come back.
4. Once the scan is stitched, its folder's `stitched/tiles.json` has the same layout at full size.

## Design notes and deviations from the plan

- **Numbers follow time, not place:** sorted by each image's `acquired`, so a snake or a spiral is numbered as it went, as the log shows it.
- **The layout is in the pixels of the mosaic it describes.** The app scales it to whatever size the preview is shown at, so the server needn't know the shrunken preview's exact size.
- **Shapes are placed in screen pixels,** rather than an SVG `viewBox` that scales them. A `viewBox` would scale the labels and the line widths too.
- **The pointer passes through** to the image, except on a line or a label, which shows its title.
- **One colour in both themes:** the images don't change with the theme, so the annotations don't either. Amber over a dark halo shows on the pale slide and on dark tissue.
- **Shown by default:** a layer is shown until it's turned off. The choice is kept by layer id, so later layers get theirs.
- **The menu stays open** after a choice, for turning several layers over.
- **Overlapping images overlap their outlines:** with a 35% overlap, a grid's outlines cross. That's where the images are; the numbers say which is which.
- **Size:** 881 changed lines of hand-written code, within L's 900 and above my estimate of 800–850. Tests are 310 of them.

## Tests

- **`tests/stitch.rs`:** a 3 × 3 scan taken in a snake has a `tiles.json` with each image's rectangle, numbered in the snake's order.
- **`tests/smart_scan.rs`:**
  - no preview layout before a scan;
  - after it, four images numbered 1–4, inside the mosaic and in the preview's proportions, the first where the scan started.
- **`overlay.spec.ts`:**
  - shapes drawn where their layer says, scaled and moved by the placement, with labels and titles;
  - nothing drawn until the image is placed;
  - the placement of a fitted image, letterboxed;
  - the choices kept in the browser, with bad values left out.
- **`menus.spec.ts`:** checkbox items are ticked when on, and turning one over keeps the menu open.
- **`SlideScanPage.spec.ts`:** the preview's layout is read for the drawing shown, and given to the overlay.
- **`scan.spec.ts`** (end to end): a real scan's two images are outlined, the first numbered 1, and the Layers menu turns them off and on.
- **A new visual baseline,** `slide-scan-outlines`, of a running scan's preview with four outlines, in both themes. The accessibility scan covers it too.
- **In a browser,** on my own server: a 3 × 2 snake's six outlines, numbered in its order, in both themes, and turned off and kept off.

## Follow-ups and known gaps

- **P53b** puts the layers over the gallery's deep-zoom viewer, from `stitched/tiles.json`.
- **Scans stitched before P53** have no `tiles.json` until they're stitched again.
- **Thousands of shapes,** such as recognised cells over a large mosaic, would want a lighter renderer (ADR-0036).
- **A scale bar** belongs to the screen, not the image, so it will need screen-anchored shapes.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
