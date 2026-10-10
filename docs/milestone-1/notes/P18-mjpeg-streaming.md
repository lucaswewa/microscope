# P18: MJPEG streaming in the client

- Status: In review
- Pull request: (added once opened)
- ADRs: none new; it extends the WoT client of [ADR-0014](../../adr/0014-client-transports-td-forms-fetch-streams-polled-invocations.md)
- Spec: [phases.md#p18](../phases.md#p18). P18 was split, at the owner's choice: `LiveImage` and the View tab are now P18b (plan Appendix D).

## Summary

The web app can now read the camera's MJPEG streams with `fetch`, and share them:

- **A multipart parser** for `multipart/x-mixed-replace`. It reads parts split anywhere between chunks, honours `Content-Length`, and hands on a JPEG as soon as it's complete.
- **`WotClient.frames()`** follows a stream with the client's headers, and reconnects like its server-sent events. **`ConsumedThing.streamUrl()`** finds a stream through the Thing Description's links.
- **Shared streams (`watchStream`):**
  - viewers of one URL share one request, and each frame is decoded once with `createImageBitmap`;
  - while a frame decodes, only the newest waits;
  - the frame rate is capped (15 fps by default);
  - the request stops once the last viewer has been gone for a second.

Nothing on screen uses it yet: P18b draws it in `LiveImage` on the View tab.

## What was built

| Where | What |
|---|---|
| `web/src/api/wot/mjpeg.ts` | `parseMultipart`, `multipartBoundary` and `byteChunks` |
| `web/src/api/wot/client.ts` | `WotClient.frames()`, sharing `stream()`'s reconnection loop, and `ConsumedThing.streamUrl()` |
| `web/src/api/wot/td.ts` | A TD's `links` |
| `web/src/live/streams.ts` | `watchStream` and the shared, reference-counted streams |
| `web/src/app/navigation.ts` | The View placeholder now says P18b |
| `web/tests/unit/api/`, `web/tests/unit/live/` | The tests below, and MJPEG helpers in the fake server |

## How to try it

```powershell
cd web
npx vitest run tests/unit/api/mjpeg.spec.ts tests/unit/api/frames.spec.ts tests/unit/live
```

P18b shows the stream in the app.

## Design notes and deviations from the plan

- **The parser** reads the format `teta-wot` writes: `--frame`, a `Content-Type: image/jpeg` header, a blank line, the JPEG and a CRLF. It's general enough for other MJPEG sources:
  - **Boundaries:** any boundary named in the response's content type, and a closing `--boundary--`.
  - **Headers:** parts with no headers are read too.
  - **Length:** `Content-Length` is used when present, so a body that happens to contain the boundary can't cut it short.
- **Handing a JPEG on early.** Without a length, a part ends at the next boundary. That would hold every frame back until the next one began, about 100 ms at 10 fps.
  - **The rule:** a JPEG part (starting with the start-of-image marker) is handed on as soon as the received bytes end with the end-of-image marker, perhaps followed by the CRLF.
  - **Why it's safe:** in JPEG's compressed data, an 0xFF byte is always followed by 0x00, so the marker can't occur by chance there.
  - **The risk:** it could in other segments, such as an embedded thumbnail, if a chunk happened to end right after one. That would hand on a truncated frame, which fails to decode and is skipped.
- **`frames()` and `stream()` share one loop.**
  - **Reconnection:** after a drop, a 5xx or no answer, it tries again after 1 s, then 2, 4 and so on, up to 30 s.
  - **Giving up:** another error ends the stream, as does an answer that isn't `multipart/x-mixed-replace`.
  - **Headers:** the client's header hook covers stream requests too (ADR-0014).
- **Shared streams:**
  - **Keys:** a stream is keyed by the client and the URL, so a new connection starts new streams.
  - **Options:** the first viewer's options apply.
  - **Pausing:** a viewer that pauses, because it's hidden, leaves, and watches again when it shows. That's how P18b pauses.
  - **The linger:** the one-second linger means switching between two tabs that both show the stream (View and Control, P20) doesn't reconnect.
- **Decoding:** one frame at a time; frames that arrive meanwhile replace the waiting one, so the display never falls behind.
  - **The cap:** it spaces shown frames at least 1/`maxFps` apart, and always shows the newest.
  - **Broken frames:** a frame that won't decode is skipped.
  - **Closing:** viewers draw each `ImageBitmap` during the callback, and it's closed straight after, freeing its memory.
- **Checked against the real server:** through `frames()` from Node, the main and lores streams each delivered 30 whole JPEGs in 3 s (10 fps, about 100 ms apart, 36 KB and 7.7 KB).
- **Size:** 749 changed lines of hand-written code (365 of them tests) against M's budget of 600, above my estimate after the split of about 500.

## Tests

14 new tests:

- **The parser** (`mjpeg.spec.ts`, 6):
  - parts after a preamble;
  - parts split at every position between two chunks and byte by byte, including within a boundary;
  - `Content-Length` with a body that looks like a boundary;
  - a part with no headers, and the closing boundary;
  - a JPEG handed on before the next boundary, while an incomplete one waits;
  - reading the boundary from a content type.
- **The client** (`frames.spec.ts`, 3):
  - `streamUrl` finds both streams and rejects a viewer page;
  - frames arrive with the `Accept` and `Authorization` headers, and stopping aborts the request;
  - it reconnects after the stream ends, but gives up on an answer that isn't a stream.
- **Shared streams** (`streams.spec.ts`, 6):
  - two viewers share one request and one decode, and the bitmap is closed after they draw;
  - the request survives a viewer coming back within the linger, and stops after it;
  - only the newest frame waits while one decodes;
  - at 10 fps with frames arriving at 50, 9 to 11 are shown, the last being the newest;
  - a frame that won't decode is skipped, even while a newer one waits;
  - viewers hear `error` and then `live`, and a late viewer hears the current state.

**Mutation checks:** I broke each of the following, and a test failed every time:

- `Content-Length`;
- the early hand-on;
- keeping a partial boundary;
- the closing boundary;
- the linger;
- sharing;
- the cap;
- skipping broken frames, which needed a stronger test;
- telling late viewers;
- closing bitmaps;
- the `Accept` header.

## Follow-ups and known gaps

- **P18b:** `LiveImage` draws `watchStream`'s frames, pauses while hidden, and shows the states. The View tab and the "Disable stream" preference come with it, along with the 10 fps check and the CPU measurement.
- **P19's** typed facades may wrap `streamUrl` for the camera.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. OpenFlexure's app shows its stream in an `<img>`; this reader is this project's own, from the `multipart/x-mixed-replace` format.
