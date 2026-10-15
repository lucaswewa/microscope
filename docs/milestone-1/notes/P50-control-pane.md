# P50: Control pane: a steady layout and the wide pane

- Status: Done
- Pull request: [#63](https://github.com/lucaswewa/microscope/pull/63)
- ADRs: none new
- Spec: [phases.md#p50](../phases.md#p50)

## Summary

The first phase of Stage 9, added after the `v0.1.0` release.

- **"Moving…" no longer shifts the pane.** The line used to be added while the stage moved and removed when it stopped, so everything below it jumped up and down. It now sits in the live position's row, which is always there, and is empty while the stage is still.
- **The Control tab's pane is wide,** as Slide Scan's and Sequence's are: 400 pixels instead of 180. The Position section uses the width: x, y and z side by side, and Move, Set Home and Move Home in one row.
- **The plan has a Stage 9:** P50–P54, with their specs in `phases.md` and an entry in Appendix D.

## What was built

| Where | What |
|---|---|
| `web/src/control/PositionSection.vue` | The status in the live row, always there, as a screen-reader status; the fields and buttons in rows of three |
| `web/src/views/ControlPage.vue` | The wide pane |
| `web/src/ui/ControlPane.vue` | Its comment: only Settings uses the narrow pane now |
| `web/tests/unit/control/ControlPage.spec.ts` | A test that the status line is there whether or not the stage moves |
| `web/tests/visual/shell.spec.ts-snapshots/control-*.png` | The Control tab's four baselines, regenerated |
| `docs/milestone-1/implementation-plan.md`, `phases.md` | Stage 9 |

## How to try it

With a server running and the app open (for example `cargo run -p microscope-server -- -c configs/simulation.json`, then `npm --prefix web run dev`):

1. Open **Control**. The pane is as wide as Slide Scan's.
2. In **Position**, set z to 1500 and press **Move**. "Moving…" shows beside the position, by the refresh button, and nothing below it moves. It clears when the stage stops.

## Design notes and deviations from the plan

- **The status sits in the live position's row** rather than a line of its own kept empty. The row is there anyway, so the pane doesn't spend a line on it, and the status is beside the numbers it's about.
- **It's announced:** the status has `role="status"`, so a screen reader reads "Moving…" when it appears, without moving focus.
- **The Navigation section** keeps its full-width fields, like the forms in Slide Scan and Settings, which share it. A row of three would have crowded Settings.
- **The direction pad** stays centred, at its size.
- **The live image is narrower** by the 220 pixels the pane gained.
- **Size:** 60 changed lines of hand-written code, within S's 300.

## Tests

- **`ControlPage.spec.ts`:** the status line exists and is empty while the stage is still, and says "Moving…" while it moves. With the old `v-if`, the test fails.
- **In a browser,** on my own server: the Autofocus button stays at the same height before, during and after a move, in both themes.
- **The visual baselines:** only the Control tab's four changed.
- **The end-to-end suite** passes against the new layout, double-click-to-move included.

## Follow-ups and known gaps

- None.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
