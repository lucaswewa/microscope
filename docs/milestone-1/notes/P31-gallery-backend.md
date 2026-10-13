# P31: Gallery backend and data route

- Status: Done
- Pull request: [#42](https://github.com/lucaswewa/microscope/pull/42)
- ADRs: [ADR-0026](../../adr/0026-gallery-index-and-data-route.md)
- Spec: [phases.md#p31](../phases.md#p31)

## Summary

The microscope now keeps one index of everything it has saved, and serves the files safely.

- **The `gallery` Thing** lists every item in the data folder for the Gallery tab (P32), newest first: captures now, and scans and sequences when they land. Each entry has what its card shows: a name, a time, a thumbnail, the viewer's data, info lines, downloads, actions and messages.
- **The index follows every write.** It's built when the gallery starts, and brought up to date from the folders whenever it's read. So it sees a capture from any camera, and files copied or deleted by hand.
- **Thumbnails** are made when a capture is saved, and by the gallery for older captures that lack one.
- **Captures can be deleted** one by one with their sidecar and thumbnail, or all at once by card type. Each can also be downloaded with its sidecar as a ZIP.
- **`GET /api/v1/data/{*path}`** serves the data folder's files with their content types, and nothing outside the folder.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/gallery.rs` | `Gallery`: `card_types`, `list_data`, `bulk_actions`, `delete_all_data`, and the endpoints `DELETE captures/{name}` and `GET captures/{name}/zip`. Also `GalleryEntry`, `Viewer`, `InfoLine`, `CardMessage`, and the captures' provider |
| `crates/microscope-things/src/capture.rs` | Thumbnails: `thumbnail_path`, `save_thumbnail`, and one saved with each capture |
| `crates/microscope-things/src/data.rs` | `data::zip`, the ZIP helper |
| `crates/microscope-things/src/ui.rs` | `DownloadSource::Data`. The button types can now be deserialised, and have JSON schemas, so they can be property values |
| `crates/microscope-server/src/data.rs`, `cli.rs` | The data route, `data_routes` |
| `crates/microscope-things/src/registry.rs`, `configs/simulation.json` | `microscope.gallery:Gallery`, as `gallery` |
| `web/src/api/generated/` | The gallery's affordances in `openapi.json` and the generated types |
| `web/tests/e2e/`, `web/tests/visual/` | The Gallery tab is now in the rail, as its placeholder until P32 |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. Open `http://127.0.0.1:5090/#/control` and press **Capture** two or three times, with "Save to gallery". The rail now has a Gallery tab, which is still a placeholder.
2. Open `http://127.0.0.1:5090/api/v1/gallery/list_data`. It shows the captures, newest first, each with its `path`, `thumbnail`, info lines and download buttons.
3. Open the first one's thumbnail, `http://127.0.0.1:5090/api/v1/data/captures/thumbnails/<name>.jpeg` (320 × 240), and its image, `…/api/v1/data/captures/<name>.jpeg`.
4. Download `http://127.0.0.1:5090/api/v1/gallery/captures/<name>.jpeg/zip`. It holds the image and its sidecar.
5. Delete it: `Invoke-RestMethod -Method Delete http://127.0.0.1:5090/api/v1/gallery/captures/<name>.jpeg`. It's gone from `list_data` and from the data folder's `captures`, with its sidecar and thumbnail.
6. A path out of the data folder is refused: `http://127.0.0.1:5090/api/v1/data/..%2F..%2FCargo.toml` answers 404.

Captures already in your data folder from earlier phases get thumbnails when the server starts.

## Design notes and deviations from the plan

- **The index is brought up to date when it's read,** rather than told of each write, as plan §5.9 first had it.
  - **How:** each read lists the folders, and describes again only items whose files' sizes or times changed.
  - **Why:** it gives the same result, also sees changes made by hand, and no Thing has to know about the gallery.
  - The plan is updated (§5.9, Appendix D), and ADR-0026 explains it.
- **One provider for all captures,** on the gallery. OpenFlexure deletes a capture through the camera that took it. Here every camera saves to the same `captures/` folder (ADR-0025), so the gallery has the endpoints.
- **`teta-wot`'s custom endpoints** match only `{id}` segments, and hand over the whole request. So the handlers read the capture's name from the URL, percent-decode it, and accept only a JPEG directly in `captures/`.
- **Entries, compared with OpenFlexure's:**
  - **The same:** the names `card_type`, `download_buttons`, `card_actions`, `card_messages` and `delete_endpoint`.
  - **Shorter names:** `thumbnail` and `viewer`.
  - **Different:** `info` is a list of lines that the server writes, rather than data for each card type, so the web app needn't know each type. `created` is a time in RFC 3339 rather than seconds.
  - **Left out:** `thing`, `modified` and `gallery_added_data`, which nothing here needs.
- **`delete_all_data` refuses an unknown card type,** where OpenFlexure ignores it, so a mistake doesn't pass silently.
- **`bulk_actions` is empty:** captures have none, as in OpenFlexure. Scans bring some (P35).
- **Downloads:** "Image" comes through the data route, with the new `DownloadSource::Data`. "Image and metadata" is the ZIP.
  - The web app's `DownloadButton` doesn't draw `Data` yet. The Gallery tab's cards (P32) will.
- **Thumbnails** are at most 320 pixels on a side, never enlarged. A thumbnail that can't be saved is logged, and doesn't fail the capture.
- **The data route** is its own router, `data_routes`, merged beside `app_routes`, so `app_routes` and its tests didn't change.
- **Dependencies:**
  - **`zip` 8.6.0** (MIT) without its codecs, with `typed-path` 0.12.3 (MIT or Apache-2.0). Version 9 is less than two weeks old.
  - **`percent-encoding`** was already in the tree.
- **Size:** 1,033 changed lines of hand-written code against L's budget of 900, 15% over. I'd estimated about 760. The owner chose to keep one pull request.

  | Part | Lines |
  |---|---|
  | `gallery.rs` | 453: 349 code, 69 comments, 35 blank |
  | The data route and its wiring | 84 |
  | Thumbnails, the ZIP helper, the button types, the registry and the configuration | 118 |
  | Tests: the gallery 257, the data route 98, the P26 capture test 16, the end-to-end rail 7 | 378 |

- **Checked by hand,** on my own server on port 5098 with its own data folder:
  - a full-resolution capture listed with its info;
  - its image (404 KB) and its 320 × 240 thumbnail through the data route;
  - its ZIP, holding the image and the sidecar;
  - `..%2F..%2Fconfig.json` answered 404;
  - deleting it answered 204, and the list was empty.

## Tests

- **`tests/gallery.rs`** (2 tests), against the simulated camera:
  - **the index:**
    - `card_types` is `Capture`, and `bulk_actions` is empty;
    - it's built at start-up from captures already there, with a whole entry checked field by field;
    - a thumbnail is made for a capture without one, and isn't enlarged;
    - a capture without a sidecar is listed with a warning;
    - a file that isn't a JPEG is left out;
    - a new capture is listed first, with a 320 × 240 thumbnail;
    - files written and removed by hand show on the next read. A sidecar written after its image clears the warning;
  - **downloads and deletion:**
    - the ZIP holds the image and its sidecar, byte for byte;
    - names that lead out of `captures/` (`..%2F`, `..%5C`, `%2e%2e`, `thumbnails%2F…`) answer 404 for both endpoints, and delete nothing;
    - deleting removes the image, the sidecar and the thumbnail, and deleting again answers 404;
    - `delete_all_data` refuses an unknown card type, then deletes every capture, and nothing outside `captures/`.
- **`microscope-server`'s `tests/data.rs`** (2 tests):
  - the content types, including a name with a space;
  - 404 for a missing file or a folder;
  - nothing outside the folder, through `..`, encoded `..`, either slash, a path inside and back out, an absolute path, or a directory junction inside the folder.
- **`tests/capture.rs`:** a capture now also saves a 320 × 240 thumbnail.
- **The end-to-end tests:** the rail now has Gallery, and still no Sequence. The four visual baselines show the new tab.

**Mutation checks:** I broke each of 13 behaviours. A test failed for 12 of them:

- **the index:**
  - the stamp leaving out the sidecar;
  - keeping an entry whose files changed;
  - keeping items that are gone;
  - oldest first;
- **thumbnails:**
  - none made for older captures;
  - none saved with a capture;
  - small images enlarged;
- **deletion:**
  - the sidecar left behind;
  - names with a folder in them allowed;
  - unknown card types ignored;
- **the data route:**
  - links followed out of the folder;
  - no content types.

The 13th, `delete_all_data` deleting card types it wasn't given, can't fail yet, since there's only one card type. It can once scans have one (P35).

## Follow-ups and known gaps

- **The Gallery tab** (P32) draws the cards, and the `Data` download source.
- **A thumbnail deleted by hand** isn't noticed until its image or sidecar changes, so the card points at a missing file until then. P32's fallback thumbnail covers it.
- **The data route reads a whole file** before sending it, and doesn't answer range requests. That's fine for captures and tiles, but large scan files may need streaming (P35, P38).
- **A file watcher** could replace reading the folders on each request, if folders grow to many thousands of items.
- **The ZIP is built in memory.** `data::zip` takes any writer, so scans' larger archives (P35) can go to a file.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. I read what its gallery does, the names of its affordances and its entries' fields, and wrote this from that, in our own words.
