# P33: Image and deep-zoom viewer

- Status: In review
- Pull request: [#44](https://github.com/lucaswewa/microscope/pull/44)
- ADRs: [ADR-0027](../../adr/0027-deep-zoom-viewer.md)
- Spec: [phases.md#p33](../phases.md#p33)

## Summary

The gallery's View now opens a deep-zoom viewer, on OpenSeadragon.

- **What it shows:** one image, a deep-zoom (DZI) image, or a sequence of images with a slider.
- **The wheel, a pinch, and the buttons** zoom. Dragging pans. "Show it all" fits the image again.
- **Brightness, contrast and saturation** each have a slider, with Reset.
- **Full screen** fills the screen, and Escape leaves it.
- **The controls are the app's,** so they follow the theme.
- **Checkpoint C is checked:** captures appear as cards that you can filter, page through, download, delete and open in the viewer.

## What was built

| Where | What |
|---|---|
| `web/src/gallery/DeepZoom.vue` | The wrapper around OpenSeadragon: made on mount, destroyed on unmount, marked loading, shown or failed; zoom and "show it all" for its parent |
| `web/src/gallery/ImageViewer.vue` | The viewer's dialog: the buttons, the view, the sequence's slider, and the filters |
| `web/src/ui/AppSlider.vue` | A slider, on Reka UI's, also in the components gallery (`/#/dev/components`) |
| `web/src/ui/AppDialog.vue` | A `full` size, which fills the window |
| `web/src/gallery/GalleryPage.vue` | View opens the viewer, in place of P32's image dialog |
| `crates/microscope-things/src/gallery.rs` | The `dzi` and `directory` kinds of `viewer`, for scans (P38) and sequences (P41) to use |
| `web/tests/fixtures/dzi/` | A 600 × 400 DZI fixture with 256-pixel tiles, drawn by the simulator |
| `web/package.json` | `openseadragon` 6.1.1 |

## How to try it

```powershell
npm --prefix web install
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. Capture an image or two from `http://127.0.0.1:5090/#/control`, with "Save to gallery".
2. In **Gallery**, press **View** on a card, or click its thumbnail. The image opens in the viewer.
3. Zoom with the wheel or the + and − buttons, and drag to pan. "Show it all" fits it again.
4. Move **Saturation** to the left: the image turns grey. **Reset** brings it back.
5. Press **Full screen**. The viewer fills the screen. Press Escape to leave.

Deep-zoom images and sequences come with scans (P38) and timelapses (P41). Until then, the end-to-end test shows both, using the fixture.

## Design notes and deviations from the plan

- **OpenSeadragon is loaded when first needed.** It's in its own chunk, 348 KB (87 KB compressed), so the app's first load doesn't grow by it. The app's own chunk grew by about 20 KB.
- **The filters are a CSS filter** on the view, from 0 to 200 %. They work whichever drawer OpenSeadragon picks (WebGL or canvas), but they're only on screen: a download is the file as saved. ADR-0027 compares this with the filtering plugin.
- **Full screen is two things:** the dialog fills the window (`AppDialog`'s new `full` size), and the host fills the screen (ADR-0010). If the screen is left another way, as Escape does, the dialog returns to its size. Closing the viewer leaves full screen.
- **Other servers' images:** OpenSeadragon asks for them with `crossOriginPolicy: 'Anonymous'`, since WebGL can only draw another server's images with its permission. The data route gives it (ADR-0026).
- **The kinds of `viewer`,** added to the Rust model so the generated types have them:
  - `dzi { path }`, the `.dzi` file, with its tiles in `<name>_files/`;
  - `directory { paths, start }`.
  - Nothing makes them yet. P38's DZI writer must follow the fixture's layout.
- **OpenSeadragon's types** leave images out of a sequence's sources, which it accepts, so `DeepZoom` casts them, with a comment.
- **The fixture** was drawn by the simulator's blobs and cut into tiles by a throwaway program, not committed. It's 18 files, 117 KB. As fixtures, they don't count towards the size.
- **The dependency:**
  - `openseadragon` 6.1.1 is BSD-3-Clause, from 2026-09-09, with no dependencies. It was installed with `--before=2026-09-29`.
  - `npm audit` finds the same 5 high-severity advisories as on `main` (in `braces` and `source-map-js`, from the development tools), and none new.
- **Checked in a browser, in both themes,** on my own server (port 5098):
  - a capture in the viewer;
  - zoomed in, with saturation at 0;
  - full screen;
  - the DZI fixture, through a faked gallery list.
- **Size:** 643 changed lines of hand-written code against M's budget of 600, 7% over, within my estimate of 500–650.

  | Part | Lines |
  |---|---|
  | `ImageViewer.vue` | 140 |
  | `DeepZoom.vue` | 88 |
  | `AppSlider.vue` | 87 |
  | `AppDialog`, `GalleryPage`, the components gallery, Rust and `package.json` | 66 |
  | Tests: the gallery's unit tests 147, `AppSlider`'s 34, end-to-end 81 | 262 |

## Tests

- **`tests/unit/gallery/GalleryPage.spec.ts`** (4 new tests for the viewer), with a fake OpenSeadragon:
  - **an image:**
    - opened from its card, with its name;
    - its source from the data route, and not a sequence;
    - loading, then shown;
    - zoom in, zoom out and "show it all";
    - closing destroys the viewer;
  - **a DZI and a sequence:**
    - the DZI's `.dzi` URL;
    - "It couldn't be shown." when it fails;
    - no slider for one image;
    - the sequence's images, its sequence mode and its first page;
    - "2 of 3", and the slider moving to the next;
  - **the filters:** each slider changes the CSS filter, and Reset restores it;
  - **full screen:** it asks the host and fills the window; it returns when the host's screen is left; closing leaves full screen.
- **`tests/unit/ui/choices.spec.ts`:** `AppSlider` is named, has its range, and moves with the arrows, Page Up, Home and End.
- **`tests/e2e/viewer.spec.ts`** (2 tests), with the real OpenSeadragon in Chromium and the fixture:
  - a DZI's tiles are fetched, and zooming in fetches the most detailed level's;
  - Brightness at its end, and Reset, change the computed filter;
  - a sequence steps on with its slider, and fetches the next image.

**Mutation checks:** I broke each of 13 behaviours, and a test failed every time:

- never a sequence;
- failures not marked;
- the page not followed;
- the viewer never destroyed;
- "show it all" unwired;
- a DZI taken for an image;
- the first page ignored;
- no saturation;
- Reset unwired;
- zoom in zooming out;
- leaving the screen unnoticed;
- full screen kept after closing;
- the slider unnamed.

## Follow-ups and known gaps

- **Untested:** asking for another server's images with CORS. The tests' images come from the same server.
- **The filters reset** each time the viewer opens, and can't be saved with the image.
- **Real DZIs and sequences** arrive with P38 and P41. Their phases should view one end to end.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Its gallery's viewer was the reference for what to offer; the viewer is OpenSeadragon (BSD-3-Clause), with our own controls.
