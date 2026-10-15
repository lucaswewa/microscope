# ADR-0034: Visual regression baselines

- Status: Accepted
- Date: 2026-10-15
- Phase: P47

## Context

- **The app's look** was matched to OpenFlexure's references by hand, phase by phase (P05, P06, P07, P47). Nothing would notice a later change that undoes it.
- **Screenshots differ between platforms:** fonts, anti-aliasing and scrollbars. A baseline from one operating system fails on another.
- **The app is Windows-first** (ADR-0003), and CI runs on Windows.
- **Pages show what changes from run to run:** the live stream, the computer's name, the build's version, the time, and whatever the other tests save to the gallery or log.
- **Accessibility** should stay checked as the app changes, not only reviewed once.

## Decision

- **Baselines** are Playwright screenshots of the served app, in `web/tests/visual/shell.spec.ts-snapshots/`, committed:
  - **every tab and Settings section, and a few key states:** the gallery's viewer, the calibration wizard, and the shut-down question;
  - **in both themes,** at 1280 × 800;
  - **the main layouts again at 1920 × 1080:** Control, Slide Scan, Gallery and Settings' Camera section.
- **Windows only:** the baselines are `*-chromium-win32.png`. CI runs them on Windows; another platform has none, and would need its own.
- **The tolerance** is 1% of the pixels (`maxDiffPixelRatio: 0.01` in `playwright.config.ts`), for small differences in font rendering between Windows machines. A real change to the layout or colours moves far more.
- **What changes from run to run is fixed** ([`tests/visual/pages.ts`](../../web/tests/visual/pages.ts)):
  - the live stream is off;
  - the test answers for the stage's position, inversion and measured range, the computer's name and version, the gallery, the log, and the scan's and sequence's settings;
  - one time zone and locale;
  - reduced motion, so no transition is caught half-way.
- **The server Playwright starts** keeps its settings, data and logs in a temporary folder, emptied for each run, as CI's is fresh. It doesn't share the repository's `.microscope` with development runs.
- **Updating:** `npx playwright test tests/visual --update-snapshots`, after a change to the look made on purpose, with no other server on port 5098, so Playwright starts a fresh one. The new images are reviewed in the pull request.
- **Accessibility checks** use the same pages and states ([`tests/e2e/accessibility.spec.ts`](../../web/tests/e2e/accessibility.spec.ts)):
  - **axe's WCAG 2.1 A and AA rules** on each page and state, in both themes, must find nothing;
  - **with the keyboard alone,** Tab must reach each page's controls, each showing its focus ring.
- **axe** comes from `@axe-core/playwright` and `axe-core`, which are MPL-2.0: a file-level copyleft, allowed at the project owner's choice. They're dev dependencies for the tests, and nothing of them is in the app.

## Consequences

- **A change to the look** fails CI until its baselines are updated, which puts the new images in front of a reviewer.
- **About 40 images,** a few megabytes, are in git, and change only when the look does.
- **On another platform,** the visual tests fail until it has its own baselines. Linux and macOS aren't milestone 1's (ADR-0003).
- **Mocked answers** keep the pages steady, but the pages then don't show the real server's values. The end-to-end tests check those.
- **axe catches** missing names, low contrast and invalid ARIA, but not everything: the keyboard walk and a person's review still matter.

## Alternatives considered

- **Baselines on every platform:** three sets to keep up, for platforms milestone 1 doesn't ship on.
- **Masking what changes,** rather than answering for it: the masks would hide parts of the pages, such as About's facts and the log.
- **A screenshot service, such as Percy or Chromatic:** an outside account and a cost, for one platform's baselines.
- **A stricter tolerance:** font rendering differs by a little between Windows machines, and a pixel-exact match would fail without a real change.
- **pa11y, or Lighthouse's accessibility checks:** both use axe underneath, with a browser of their own, where the end-to-end tests already have Playwright's.
