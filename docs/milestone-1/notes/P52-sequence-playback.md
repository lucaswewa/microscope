# P52: Sequence viewer: smooth switching and playback

- Status: Done
- Pull request: [#65](https://github.com/lucaswewa/microscope/pull/65)
- ADRs: none new
- Spec: [phases.md#p52](../phases.md#p52)

## Summary

A sequence in the gallery's viewer can be stepped through or played without the image flashing.

- **No more flash.** OpenSeadragon's sequence mode closed the image shown before opening the next, so the view went empty for a moment, and the zoom went back to the whole image each time. Now:
  - a sequence's images are kept in one view, hidden until shown;
  - a new image is shown once it has fully loaded, and only then is the one before it hidden;
  - the zoom and pan stay as they were.
- **Loaded ahead:** the two images after the one shown load while hidden, so stepping forward or playing is immediate.
- **Playback:** a Play/Pause button, the seconds each image shows (0.5 by default), and Loop.
  - Each image shows for its seconds from when it's drawn, so a slow connection slows the playback rather than skipping images.
  - At the end it stops, or loops back to the first.
  - Play on the last image starts from the first.
  - The slider and the count follow it, and moving the slider while playing carries on from there.

## What was built

| Where | What |
|---|---|
| `web/src/gallery/DeepZoom.vue` | A sequence's images in one view, each shown once loaded, the next two loaded ahead; `shown` when an image is drawn, and `data-page` |
| `web/src/gallery/playback.ts` | `usePlayback`: playing, pausing, the seconds each, and looping |
| `web/src/gallery/ImageViewer.vue` | Play/Pause, Seconds each and Loop, beside the slider |

## How to try it

With a server running and the app open (for example `cargo run -p microscope-server -- -c configs/simulation.json`, then `npm --prefix web run dev`):

1. Run a sequence from the **Sequence** tab, such as 10 seconds at a 0.5-second interval, or open one already in the gallery.
2. In **Gallery**, press **View** on the sequence. Zoom in, then step with the slider or the arrow keys: the image changes without flashing, and the zoom stays.
3. Set **Seconds each** to 0.2 and press **Play**. It plays to the end and stops; with **Loop** ticked, it starts again.

## Design notes and deviations from the plan

- **One view, many images:** OpenSeadragon draws every image in its world, and a hidden image (opacity 0, loaded with `preload`) costs nothing to draw. Showing one and hiding the other in the same frame swaps them without an empty frame.
- **Only what's near is kept:** once an image is shown, those behind it and more than two ahead are removed, so a long sequence doesn't fill the memory. Going back loads an image again, which the browser usually has in its cache.
- **Loop doesn't load ahead around the end:** the view doesn't know about looping, so the first image loads when it's wanted. The last one stays until it has.
- **Each image's time starts when it's drawn,** not when it's asked for. Measured on my own server at 0.2 seconds each, images showed for 202 milliseconds, zoomed in or not, and at 1 second for 1,002.
- **One image or a deep-zoom pyramid** opens as before.
- **`data-page`** on the view says which image is drawn, for tests, as `data-status` says whether it is.
- **The settings aren't kept:** Seconds each and Loop start at 0.5 and off each time the viewer opens.
- **Size:** 399 changed lines of hand-written code, within M's 600 and above my estimate of 300–350. Tests are 224 of them.

## Tests

- **`playback.spec.ts`:**
  - each image shows for its seconds from when it's drawn, and playback stops at the end;
  - Play on the last image starts from the first, and Loop starts again;
  - Pause stops, and a page moved to while playing carries on from there.
- **`GalleryPage.spec.ts`,** with a fake OpenSeadragon whose images load when the test says:
  - a sequence's first image is hidden until loaded, then shown with the next loaded ahead;
  - stepping keeps the shown image until the next has loaded, then swaps them, without going home;
  - Play from the last image plays from the first to the end, and stops.
- **`viewer.spec.ts`** (end to end):
  - with the last image's loading slowed by 1.5 seconds, the view stays the same, pixel for pixel, until it's drawn;
  - playing at 0.2 seconds each reaches the end and stops.
- **Mutations:** hiding the shown image before the next has loaded fails the unit test and the end-to-end screenshot comparison. Never stopping at the end fails the playback tests.
- **In a browser,** on my own server: a 10-image sequence, zoomed in, with each image delayed by 0.8 seconds, kept the image shown until the next was drawn, at the same zoom.

## Follow-ups and known gaps

- **The viewer's images** still come straight from the data route, so they can't carry a token when authentication comes (milestone-2 readiness checklist).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
