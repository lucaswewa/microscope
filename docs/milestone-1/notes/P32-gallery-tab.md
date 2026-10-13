# P32: Gallery tab

- Status: In review
- Pull request: [#43](https://github.com/lucaswewa/microscope/pull/43)
- ADRs: none new
- Spec: [phases.md#p32](../phases.md#p32)

## Summary

The Gallery tab shows everything the microscope has saved, from the gallery backend (P31, ADR-0026).

- **Cards, newest first,** 18 or 25 to a page, with "Showing a–b of n" and page buttons. The layout is detailed, or thumbnails only.
- **Each card has:**
  - a thumbnail, or a placeholder if it has none or it won't load;
  - its name and when it was made, in local time;
  - its download: a button, or a menu when there are several;
  - its actions, then View and Delete, which asks first;
  - its info lines, and its warnings and alerts.
- **A sidebar,** shown or hidden from the top bar, has:
  - the types shown;
  - the cards per page;
  - the bulk actions;
  - **Delete all,** whose question lists the types it will delete.
- **The gallery is read again** when the tab opens, on Refresh, and after anything that changes it.
- **View** shows the item's image in a dialog, until the viewer (P33) takes its place.

## What was built

| Where | What |
|---|---|
| `web/src/gallery/GalleryPage.vue` | The tab: the top bar, the grid, paging, the sidebar, Delete all, and the image dialog |
| `web/src/gallery/GalleryCard.vue` | A card |
| `web/src/server-ui/download.ts` | `useDownload` and `dataUrl`: saving what a download button describes, now shared by panels and cards, and the data folder's files as a source |
| `web/src/server-ui/DownloadButton.vue`, `types.ts` | The button uses `useDownload`, and knows the `data` source (P31) |
| `web/src/api/things/index.ts`, `web/src/router/index.ts` | `gallery` among the typed Things, with `GalleryEntry`; the tab's page in place of its placeholder |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. Open `http://127.0.0.1:5090/#/control`. Press **Capture** a few times, with "Save to gallery", moving the stage between captures.
2. Open **Gallery**. The captures are cards, newest first, with their thumbnails, sizes, positions and objective.
3. On a card, open **Download**, and choose "Image" or "Image and metadata" (a ZIP).
4. Press **View** to see the image, then **Delete**, and confirm. The card is gone.
5. Press the layout button beside Refresh for thumbnails only, and again for details.
6. Show the sidebar with the button at the top right:
   - **Cards per page** chooses 18 or 25;
   - **Show** chooses the types;
   - **Delete all** asks first, and lists the types it will delete.
7. Copy a capture's JPEG in the data folder's `captures` to a new name, and press **Refresh**. It shows, with a warning that its metadata is missing.

## Design notes and deviations from the plan

- **View shows a plain image** in a dialog. The deep-zoom viewer, with its filters and fullscreen, is P33.
- **Card actions are buttons, not a menu.** The spec has "action buttons or a menu", but captures have no actions, so there's nothing to put in a menu yet. A menu also needs the ActionButton's confirmation and progress outside a button. Scans bring the first card actions (P35), and the menu can come with them.
- **Delete all deletes the types shown,** and its question lists them, so it can't delete more than you can see.
- **The sidebar starts hidden,** as OpenFlexure's does. The layout, the cards per page and the types shown aren't kept between visits.
- **The layout:**
  - **The shape:** OpenFlexure's top bar (the layout toggle, Refresh and the sidebar), cards about 272 pixels wide (`17rem`), and full-width buttons.
  - **Only our parts:** our own components and tokens. A card is an `AppCard`.
  - **Info lines** are a description list, with the colon drawn by CSS.
- **"When the tab becomes visible"** means when the Gallery tab opens. Pages are made again each time they open, so the gallery is read then. A browser tab coming back into view isn't watched, since that would need a host API (ADR-0010), and Refresh is there.
- **Downloads:**
  - **Shared:** the server-UI download logic moved from `DownloadButton` into `useDownload`, so a card's menu saves files the same way a panel's button does.
  - **The data folder:** `data` downloads use `dataUrl`, which encodes each part of the path, so a name with a space or a `#` still works.
- **The gallery's buttons** (P31) are the panels' buttons (ADR-0024) without their `kind`. The card adds it.
- **Checked in a browser, in both themes,** on my own server (port 5091) with four items, one of them a copy without a sidecar:
  - the detailed grid;
  - thumbnails only;
  - the sidebar;
  - View;
  - Delete's question;
  - Delete all's question, listing "Capture".

  That's where the cards first stretched to fill the row and the Download menu was narrower than the other buttons, both now fixed.
- **Size:** 900 changed lines of hand-written code, exactly L's budget, within the estimate of 850–1,100. The owner chose one pull request before it was built.

  | Part | Lines |
  |---|---|
  | `GalleryPage.vue` | 263 |
  | `GalleryCard.vue` | 232 |
  | Downloads | 122 |
  | Wiring | 5 |
  | Tests: unit 253, end-to-end 25 | 278 |

## Tests

- **`tests/unit/gallery/GalleryPage.spec.ts`** (6 tests), against a fake server:
  - **paging:**
    - the newest 18 of 21, then the other 3 on the next page;
    - 25 to a page, without page buttons;
  - **a card:**
    - its thumbnail from the data route, the placeholder without one, and the placeholder when it won't load;
    - its info line and its warning;
    - its time, in local time;
    - a menu of two downloads, and a single download button, saved under their names. One has a space and a `#` in its path;
    - thumbnails only hides the buttons and the info;
  - **deleting:** the question names the item; the `DELETE` goes to its endpoint; the gallery is read again;
  - **the sidebar:**
    - the type filter, which also resets the page;
    - a bulk action runs;
    - Delete all lists only the types shown, and sends just those;
  - **Refresh and View:** Refresh reads the gallery again; View shows the image from its viewer path;
  - **empty:** it says nothing has been saved.
- **`tests/e2e/gallery.spec.ts`,** against the real server:
  - a capture from the Control tab becomes the first card, with a 320-pixel thumbnail and its size;
  - deleting it removes it.
  - **A longer wait:** the capture waits up to 20 s, for other tests holding the microscope's lock. Four copies at once took over 5 s.
- **`app.spec.ts`:** Gallery is no longer a placeholder.

**Mutation checks:** I broke each of 13 behaviours, and a test failed every time:

- paging;
- the type filter;
- going back to page 1 when the filter changes;
- the thumbnail's placeholder;
- the menu saving the wrong download;
- deleting without asking;
- not reading the gallery again after a delete;
- Delete all sending every type, or not listing them;
- Refresh;
- View showing the thumbnail;
- thumbnails only;
- unencoded data paths.

## Follow-ups and known gaps

- **The viewer** (P33) replaces the image dialog.
- **A menu of card actions,** when scans bring them (P35).
- **Untested:** after deleting the last card on the last page, it goes to the page before.
- **Not live:** the gallery doesn't update by itself while it's open. A capture made from elsewhere shows on Refresh.
- **No visual baseline** for the tab, since its cards' names and times change with each run.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The layout follows its gallery's shape, from the reference screenshots (kept out of git), with our own components and tokens.
