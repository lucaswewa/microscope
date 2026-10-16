# P53b: Overlay layers in the deep-zoom viewer

- Status: Done
- Pull request: [#67](https://github.com/lucaswewa/microscope/pull/67)
- ADRs: none new (ADR-0036)
- Spec: [phases.md#p53b](../phases.md#p53b)

## Summary

A stitched scan's images are outlined over its mosaic in the gallery's viewer, as they are over the Slide Scan preview (P53).

- **The gallery's deep-zoom viewer** gains `tiles`, the path of the scan's `stitched/tiles.json`, when stitching recorded it.
- **The viewer reads it** and draws the Image outlines layer over the mosaic.
- **The overlay follows the view:** it's placed again whenever OpenSeadragon's view pans, zooms or changes size, including full screen.
- **The viewer's Layers menu,** beside its zoom buttons, turns the outlines on and off. The choice is the same one the Slide Scan tab keeps.
- **Stitch again:** a scan stitched before P53 has its mosaic but no `tiles.json`, so its card now offers **Stitch again**, which writes it. The gallery's index notices `tiles.json` coming and going.
- **Menus and selects open above dialogs:** a new layer token, `--z-menu`, sits between dialogs and toasts. Before, a menu opened from a dialog was hidden behind it.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/gallery.rs` | `Viewer::Dzi`'s `tiles`; Stitch again; `tiles.json` in a scan's stamp |
| `web/src/gallery/DeepZoom.vue` | `layers`, drawn over the image shown, placed from the viewport on every update |
| `web/src/gallery/ImageViewer.vue` | Reads `tiles.json`; the Layers menu |
| `web/src/theme/tokens.css`, `tokens.ts`, `web/src/ui/AppMenu.vue`, `select.css` | `--z-menu`, for menus and selects |
| `web/src/api/generated/` | The viewer's new field, regenerated (ADR-0021) |

## How to try it

With a server running and the app open, and a scan stitched since P53 (see P53's notes):

1. In **Gallery**, press **View** on the scan. Each image is outlined over the mosaic and numbered.
2. Zoom in and drag: the outlines stay on their images, and their lines stay thin.
3. **Layers → Image outlines** turns them off, here and in the Slide Scan tab.

A scan stitched before P53 has no outlines until it's stitched again: press **Stitch again** on its card.

## Design notes and deviations from the plan

- **The path comes from the gallery,** rather than being guessed from the DZI's: a viewer only reads a layout the gallery says exists, and other layers' data can be added beside it later.
- **The placement is the image shown,** found in OpenSeadragon's world by opacity. A sequence's hidden pages (P52) are skipped, so layers would work over a sequence too.
- **OpenSeadragon reports every frame** of a pan or zoom (`update-viewport`), so the overlay moves with the animation, not after it.
- **`--z-menu`** is new, rather than raising `--z-dropdown`, which the offline overlay also uses and which should stay below dialogs.
- **Stitch again wasn't in the spec.** Without it, a scan stitched before P53 could only get its outlines through the API, since the card offered Stitch only to scans not stitched at all.
- **Size:** 216 changed lines of hand-written code, within M's 600 and above my estimate of 150–200, with Stitch again added. Tests are 115 of them.

## Tests

- **`tests/stitch.rs`:**
  - a stitched scan's card gives its `tiles.json` to the viewer;
  - without `tiles.json`, its card offers Stitch again, which brings it back, and the card follows. Leaving `tiles.json` out of the scan's stamp fails this.
- **`GalleryPage.spec.ts`,** with a fake OpenSeadragon placed where the test says:
  - the outlines are drawn where the view puts the mosaic, with their titles;
  - zoomed and panned, they follow;
  - a mosaic without a layout has no Layers menu.

  Without the viewport handler, the test fails.
- **`scan.spec.ts`** (end to end): the stitched scan's two images are outlined in the viewer, and zooming in widens their outlines.
- **In a browser,** on my own server, in both themes: a 3 × 2 scan's six outlines; zoomed in twice, the first went from 326 to 734 pixels wide, and a 150-pixel drag moved it 150 pixels; then turned off from the Layers menu.
- **Stitch again, in a browser:** with that scan's `tiles.json` removed, its card offered Stitch again, and afterwards its viewer had its six outlines.

## Follow-ups and known gaps

- **No visual baseline** of the viewer with outlines: the mosaic is drawn with WebGL, which the baselines leave out elsewhere too.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
