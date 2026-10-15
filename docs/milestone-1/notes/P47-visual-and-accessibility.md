# P47: Visual fidelity and accessibility pass

- Status: In review
- Pull request: [#60](https://github.com/lucaswewa/microscope/pull/60)
- ADRs: [ADR-0034](../../adr/0034-visual-regression-baselines.md)
- Spec: [phases.md#p47](../phases.md#p47)

## Summary

A last look at the whole app beside OpenFlexure's references, and checks that keep it as it is.

- **The review** compared every tab and key state, in both themes, at 1280 × 800 and 1920 × 1080. It found one real problem: Slide Scan hid Start Smart Scan below the fold, so it now opens with its sections closed. The rest is close, or different on purpose (below).
- **Visual baselines** for every tab, Settings section and key state, in both themes, and the main layouts at 1920 × 1080: 42 images, Windows only, with a 1% tolerance (ADR-0034).
- **Accessibility tests:**
  - axe's WCAG 2.1 A and AA rules on every page and state, in both themes;
  - with the keyboard alone, every stop on each page, each showing its focus ring.

  Both pass with no changes to the app.

## What was built

| Where | What |
|---|---|
| `web/tests/visual/pages.ts` | Every page and state the visual and accessibility tests open, with what changes from run to run answered by the test |
| `web/tests/visual/shell.spec.ts` | The baselines, now for every page |
| `web/playwright.config.ts` | The server Playwright starts keeps its settings, data and logs in a temporary folder, emptied for each run |
| `web/tests/e2e/accessibility.spec.ts` | axe, and the keyboard walk |
| `web/src/scan/SlideScanPage.vue` | Its sections start closed |
| `web/tests/reference/capture.ts` | The mapping section's route |
| `web/package.json` | `@axe-core/playwright` |
| `docs/adr/0034-visual-regression-baselines.md` | ADR-0034 |

## How to try it

```powershell
npm --prefix web run build
cd web
npx playwright test tests/visual tests/e2e/accessibility.spec.ts
```

With no server on port 5098, Playwright starts one, from a fresh folder of its own; with one, it uses that.

1. **Everything passes:** 42 screenshots, axe on 17 pages and states in both themes, and 14 keyboard walks.
2. **Slide Scan** opens with its sections closed, and Start Smart Scan in view, at 1280 × 800.
3. **A change shows:** make muted text fainter (`--color-text-muted` in `tokens.css`, `--palette-neutral-300` for the light theme), rebuild, and run them again. The screenshots differ, and axe reports the contrast.

## The review

From the app's screenshots beside OpenFlexure's, taken with `npm run capture:reference` (`docs/reference/`, kept out of git), on a calibrated simulator with captures, a scan and a sequence in its gallery.

| What | OpenFlexure | This app | Verdict |
|---|---|---|---|
| The rail | About 85 px wide, items about 67 px high | 83 px, 64 px | Kept: within a few pixels |
| Control's pane | About 186 px | 180 px | Kept |
| Slide Scan's pane | Sections closed, Start in view | The scan settings open, Start below the fold at both sizes | **Fixed:** closed by default |
| The live image | Fills the height, letterboxed | The same | Kept |
| Gallery cards | About 272 px, three to a row at 1280 | The same | Kept |
| Logging | Groups in a narrower column | The full width | Kept: long sources and messages need it |
| Dark theme | Near-black surfaces, accent fills | The same roles | Kept |

**Differences kept on purpose,** from earlier phases:

- **Control** puts the position and navigation first and the direction pad last (P20), and fills its primary buttons.
- **The rail** ends with the connection indicator (P11), which OpenFlexure doesn't have.
- **The live image** has its own off switch, top right (P18b).
- **Gallery cards** show their info lines and messages (P32).
- **About** adds the app's version and the launcher's status (P46).

## Accessibility

- **axe, by hand first,** from a scratch folder, on every tab, Settings section and dialog, in both themes: no violations.
  - **Two "incomplete" kinds**, which axe can't judge, both fine:
    - contrast where a field is partly covered at the edge of a scrolled pane;
    - `aria-hidden` on the app behind a modal dialog, which the dialog's focus trap handles.
- **The keyboard walk,** by hand on every page: every control is reached, in reading order, rail first, and every stop shows the focus ring.
- **The tests** now check both on every page and state, in both themes for axe.
- **No app changes were needed.** One test-only trap: axe once saw the Control page's Move button with no background. Its fill comes in by a transition as it's enabled, and transitions advance only when a frame is drawn. The tests wait for transitions to finish, and run with reduced motion.

## Design notes and deviations from the plan

- **The baselines' pages answer for what changes from run to run,** rather than masking it, so About's facts and the log are in the images. See ADR-0034.
- **They're independent of the other tests:** the scan and sequence tests leave their own settings behind, which the pages answer for.
- **And of what a server has saved.** The project owner's first run of "How to try it" failed Settings > Stage.
  - **Why:** with no server on 5098, Playwright started one that kept its settings in the repository's `.microscope`, shared with the owner's own runs. There, the stage's range of motion had been measured.
  - **Two fixes:**
    - the pages now answer for the stage's inversion and its measured range too;
    - Playwright's own server keeps its settings, data and logs in a temporary folder, emptied for each run, as fresh as CI's. It no longer reads or writes `.microscope`, whose captures stayed as they were through a whole run.
  - **Checked** with a copy of the owner's settings: Settings > Stage failed without the fix, as the owner saw, and passes with it.
- **The 1920 × 1080 baselines** are the four main layouts, not every page: the others only grow.
- **axe's licence** is MPL-2.0, a file-level copyleft, allowed at the project owner's choice as a dev dependency. It isn't in the app.
- **`npm audit`** reports the same five findings as on `main`, all in existing development tooling; the new packages add none.
- **Size:** 299 changed lines of hand-written code, within L's 900, over my estimate of about 260 by the fix above. The 42 images, 2.5 MB, aren't counted.

  | Part | Lines |
  |---|---|
  | The pages and their answers | 145 |
  | The visual tests | 54 |
  | The accessibility tests | 53 |
  | Playwright's own server, in a fresh folder | 32 |
  | Slide Scan and its test | 10 |
  | The capture route, a fixture's import, `package.json` | 5 |

## Tests

- **Visual** (42): every tab and Settings section in both themes at 1280 × 800, with the gallery's viewer, the calibration wizard and the shut-down question; Control, Slide Scan, Gallery and Settings' Camera section also at 1920 × 1080.
- **axe** (34): every page and state in both themes.
- **The keyboard** (14): every page.
- **Slide Scan's unit test:** its sections start closed, with Start in view, and open to their panel.

**Mutation checks:** I broke 4 things, and a test failed for each: the focus ring removed, muted text too faint, Slide Scan's settings open again, and buttons without their names.

All ran twice in a row, and again after the whole end-to-end suite, without a flake. They pass on a fresh server, on one the whole suite has used, and on a copy of the project owner's settings.

## Follow-ups and known gaps

- **The calibration wizard's welcome page** isn't in the baselines: it only opens on an uncalibrated microscope, and the tests calibrate theirs.
- **The reference capture script** still sends this app's wizard states to routes it doesn't have.
- **Scan cards** name the workflow by its Thing's name, `snake_workflow`, rather than "Snake scan".
- **A person with a screen reader** hasn't walked the app; axe and the keyboard tests don't replace that.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The comparison is of proportions and roles, from the screenshots, which stay out of git.
