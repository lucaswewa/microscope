# P18b: View tab: live image

- Status: Done
- Pull request: [#25](https://github.com/lucaswewa/microscope/pull/25)
- ADRs: none new
- Spec: [phases.md#p18b](../phases.md#p18b), split from P18 (plan Appendix D)

## Summary

The View tab shows the camera's live image, full window:

- **`LiveImage`** draws the shared MJPEG stream from P18 to a canvas, contain-fit with letterboxing. It says why when there's no image: connecting, turned off, no connection, or an error.
- **It pauses its stream** while it's off screen or the page is hidden. The stream also stops when you leave the tab.
- **It flashes** when a capture is taken; P26 triggers this.
- **A "Disable stream" button** on the View tab turns the stream off and on, and the choice is remembered in this browser.

On the development machine, the View tab runs at 9.9 frames a second, using 9.3% of one core across the browser's processes.

## What was built

| Where | What |
|---|---|
| `web/src/live/LiveImage.vue` | The live image: drawing, states, pausing and the flash |
| `web/src/live/preferences.ts` | `useLiveImagePreferences`: `streamDisabled`, kept in `localStorage` |
| `web/src/live/flash.ts` | `flashLiveImage()`, which P26's capture will call |
| `web/src/views/ViewPage.vue`, `web/src/router/index.ts` | The View tab, the first page that isn't a placeholder |
| `web/src/theme/` | The `--color-capture-flash` token, and a `.visually-hidden` class for the View page's heading |
| `web/tests/unit/live/LiveImage.spec.ts`, `web/tests/e2e/view.spec.ts` | The tests below, and the smoke and app tests updated for the View page |

## How to try it

From the repository's root, build the web app and let the server serve it:

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Open `http://127.0.0.1:5090/#/view`. Then:

- **Watch the stream:** move the stage from `http://127.0.0.1:5090/docs` and see the image follow.
- **Turn it off:** use the eye button at the top right.
- **Leave the tab** and watch the stream's request end in the browser's network panel.

## Design notes and deviations from the plan

- **Drawing:**
  - **The canvas** takes each frame's size, and CSS `object-fit: contain` fits it into the window, letterboxed on the image backdrop. So a resize costs nothing, and a frame is drawn at its own resolution.
  - **Counting:** the canvas counts the frames it has drawn in `data-frames`, which the end-to-end tests read.
- **When it watches:** only while all of these hold:
  - the app is connected;
  - the camera's description has the stream;
  - the stream isn't disabled;
  - the image is on screen (an `IntersectionObserver`), and the page is visible.

  Otherwise it leaves the shared stream, which stops a second after its last viewer goes (P18).
- **States:** `data-status` says `connecting`, `live`, `disabled`, `offline` or `error`. A message on a raised card says the same in words, and while connecting, a spinner shows.
  - **Offline:** "No connection to the microscope" follows the connection store.
  - **Errors** show the stream's last error, such as "The microscope could not be reached" after the server stops, over the last frame.
- **The flash:** `flashLiveImage()` makes every live image fade from 70% white over 400 ms. With reduced motion, the base styles shorten it to nothing. The flash colour is a new semantic token, `--color-capture-flash`, white in both themes, as ADR-0011 asks.
- **"Disable stream":** OpenFlexure offers this option for slow connections. Here it's an icon button over the View tab's top right, named "Disable stream" or "Enable stream". The message offers "Turn it on" too.
  - **Where it's kept:** in this browser, under `microscope.stream-disabled`.
  - **What it affects:** every live image. The Control tab (P20) will show one too.
- **The View page's heading** is visually hidden: the page is all image, and the heading still names it for screen readers.
- **Measurements:** Chromium against a release server, 1280 × 800, in focus, over 10 s each.

  | Page | Frames a second | Browser CPU (one core) | Page main thread busy |
  |---|---|---|---|
  | Control tab, no stream | 0 | 0.9% | 0.3% |
  | View tab, live | 9.9 | 9.3% | 1.3% |
  | View tab, stream disabled | 0 | 0.7% | 0.2% |

  Most of the cost is decoding and compositing, off the main thread. The server limits the stream to 10 fps, below the client's 15 fps cap.
- **Size:** 548 changed lines of hand-written code (about 220 of them tests), within M's budget of 600.

## Tests

- **`LiveImage.spec.ts`** (5 tests):
  - **States:** no connection, then connecting, then live, with frames counted and the canvas sized to the frame.
  - **Errors:** an error with the server's message, then live after it reconnects.
  - **Pausing:** while off screen and while the page is hidden, the request stops, and a new one starts on return.
  - **The flash.**
  - **The View page's button** turns the stream off. The choice survives a new page, and "Turn it on" restores it.
- **`app.spec.ts`:** the View route shows the live image.
- **End to end** (`view.spec.ts`, 3 tests, against the real server):
  - frames arrive;
  - the stream's request ends when you go to Control;
  - "Disable stream" survives a reload, and "Turn it on" brings frames back.
- **The smoke test** now finds the live image on the View page.

**Mutation checks:** I broke each of the following, and a test failed every time:

- pausing;
- pausing for page visibility alone;
- honouring "Disable stream";
- remembering it;
- the offline state;
- the flash;
- sizing the canvas.

**Checked by hand:** screenshots in both themes, letterboxed in a tall window, with the stream off, and after the server stopped.

## Follow-ups and known gaps

- **P20** shows `LiveImage` on the Control tab, alongside the stage controls.
- **P26** calls `flashLiveImage()` when a capture is taken.
- **Settings:** the preference could also appear on a Settings page once one exists (P24 onwards).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The View tab's purpose and its "disable stream" option follow OpenFlexure's (plan §6); the design and code are this project's own.
