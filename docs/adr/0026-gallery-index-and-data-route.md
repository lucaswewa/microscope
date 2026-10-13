# ADR-0026: Gallery index and data route

- Status: Accepted
- Date: 2026-10-13
- Phase: P31

## Context

- **What the gallery shows:** everything the microscope has saved. That's captures now (P26), and scans (P35) and sequences (P41) later. The Gallery tab (P32) needs one list, with what each card shows, how to delete it, and which files to download and view.
- **The files are the truth:** there's no database (plan §5.9, ADR-0025). OpenFlexure keeps an SQLite database for each kind of item, and checks it against the files.
- **Files change at any time:** Things write them (a capture, later a scan), and people copy or delete them by hand.
- **The web app needs the files:** images, thumbnails and, later, deep-zoom tiles, served safely from the data folder.

## Decision

- **The `gallery` Thing** (`microscope.gallery:Gallery`), with OpenFlexure's names (ADR-0007):
  - `card_types`: the kinds of item, such as `Capture`;
  - `list_data`: every entry, newest first;
  - `bulk_actions`: buttons for the sidebar (ADR-0024). Captures have none;
  - `delete_all_data(card_types)`: deletes every item of those types. An unknown type is an error.
- **An entry** (`GalleryEntry`):
  - `name`, `path` (in the data folder), `card_type`, and `created` (RFC 3339, UTC, to the millisecond);
  - `thumbnail`, its path if it has one;
  - `viewer`, how the viewer shows it: `{"type": "image", "path": …}` now. Deep-zoom images (P33, P38) and folders of images (P35, P41) add types;
  - `info`, lines of a label and a value that the server writes, so the web app needn't know each card type. Times are left to the web app, which shows them in local time (ADR-0025);
  - `download_buttons` and `card_actions`, ADR-0024's buttons;
  - `card_messages`, each a warning or an alert;
  - `delete_endpoint`, where to send `DELETE`, relative to the API prefix.
- **Providers:** one for each card type, in the gallery's code. A provider lists its items, each with a stamp of its files' sizes and modification times. It also describes an item and deletes one.
  - **Captures:** every `.jpeg` or `.jpg` directly in `captures/`. The stamp covers the image and its sidecar.
- **The index:**
  - It's kept in memory, by path, and built in the background when the gallery starts.
  - **Each read of `list_data`** lists the folders again. It describes again only the items whose stamp changed, and drops the ones that are gone.
  - **So it follows every write,** by any Thing or by hand, without any Thing having to tell the gallery.
- **Thumbnails:** `captures/thumbnails/<name>`, a JPEG at most 320 pixels on a side, never enlarged.
  - **Made when a capture is saved,** and by the gallery for a capture without one, such as those from before P31.
  - **A thumbnail that fails** doesn't fail the capture.
- **Deleting:** an endpoint on the gallery for each provider. `DELETE {prefix}/gallery/captures/{name}` deletes the image, its sidecar and its thumbnail, and answers 204.
  - **The name,** percent-decoded, must be one path segment, naming a JPEG in `captures/`. Anything else answers 404.
- **Downloads:**
  - **The image,** through the data route: a new source of download button, `DownloadSource::Data { path }`;
  - **the image and its sidecar,** as a ZIP, from `GET {prefix}/gallery/captures/{name}/zip`.
  - **`data::zip`** writes the archives. The files are stored without compression, since images are compressed already, so the `zip` crate is used without its codecs.
- **The data route:** `GET {prefix}/data/{*path}`, among the server's own routes (ADR-0006).
  - **Paths:** they're resolved by `data::resolve`, so no root, drive or `..`. The file must still be in the data folder once links are followed.
  - **Anything else, or a folder,** answers 404 with `{"detail": …}`.
  - **The content type** is set by the extension: JPEG, PNG, TIFF, JSON, DZI and XML, CSV, text and logs, ZIP, and otherwise `application/octet-stream`. It's sent with `X-Content-Type-Options: nosniff`.
  - **Cross-origin requests** are allowed, as for the other application routes.

## Consequences

- **The files stay the truth:** changes made by hand show on the next read.
- **Each read looks at every item's files.** That's quick for thousands of captures. A folder of very many items would need a file watcher.
- **The first start** after many older captures makes their thumbnails, in the background.
- **A new card type is a code change** in the gallery (a provider and its delete endpoint), not configuration.
- **`teta-wot`'s custom endpoints** match only `{id}` segments, and get the whole request. So the handlers read the name from the URL themselves.
- **The data route serves the whole data folder** to anyone who can reach the API. That's like the rest of the API, which has no authentication in milestone 1.
- **The button types** (`ActionButton`, `DownloadButton`) can now be deserialised and have JSON schemas, so they can be property values, and they appear in the generated API types (ADR-0021).

## Alternatives considered

- **SQLite, as OpenFlexure has.** It would be quicker to query, but it's a second truth to keep in step with the files, and a native dependency.
- **Things telling the gallery of each write,** as the plan first had it. That misses changes made by hand, and ties every writer to the gallery. Reading the folders again gives the same result with less coupling.
- **A file watcher.** It would update at once, but it's another dependency with platform quirks, and not needed at milestone 1's sizes.
- **Thumbnails made on request** by the data route. There'd be no files to manage, but every card shown would decode a full image.
- **`tower-http`'s `ServeDir`.** It handles ranges, but it brings more dependencies, and answers errors in its own way rather than with `{"detail": …}`.
- **Deleting through an action.** It would be typed and in the Thing Description, but a URL for each item is simpler for the cards, as OpenFlexure's are.
