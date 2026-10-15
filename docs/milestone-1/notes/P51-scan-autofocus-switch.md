# P51: Slide Scan: an autofocus switch

- Status: Done
- Pull request: [#64](https://github.com/lucaswewa/microscope/pull/64)
- ADRs: none new
- Spec: [phases.md#p51](../phases.md#p51)

## Summary

A scan can run without autofocus, as a sequence can.

- **`smart_scan.autofocus_enabled`,** on by default and kept with the settings. It's read when a scan starts, and passed to the workflow.
- **Off, a scan doesn't autofocus.** It skips the autofocus at its start, and takes one image at each site where the stage is, instead of a smart z-stack. The stage stays on the plane it started on.
- **C-Chip** skips its starting autofocus, and keeps its basic stack at each site, which doesn't autofocus.
- **The scan's record** (`scan.json`, and the settings the tab shows) has `autofocus_enabled`, and the log says when it's off.
- **The Slide Scan tab** has a switch, "Autofocus at the start and at each site", below the stitching settings.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/workflow.rs` | `ScanWorkflow::begin` and `acquire` take whether to autofocus; `at_site`, a smart stack or one image; the grid workflows |
| `crates/microscope-things/src/histo.rs`, `c_chip.rs` | The histo and C-Chip workflows, with the switch |
| `crates/microscope-things/src/smart_scan.rs` | `autofocus_enabled`, passed to the workflow, recorded and logged |
| `web/src/scan/SlideScanPage.vue` | The switch |
| `web/src/api/generated/` | The new property, regenerated (ADR-0021) |
| `web/tests/visual/shell.spec.ts-snapshots/slide-scan-*.png` | The Slide Scan tab's four baselines, with the switch |

## How to try it

With a server running and the app open (for example `cargo run -p microscope-server -- -c configs/simulation.json`, then `npm --prefix web run dev`), and the camera–stage mapping calibrated:

1. In **Control**, move z away from focus, such as by 200 steps.
2. Open **Slide Scan**, choose **Snake scan**, turn off **Autofocus at the start and at each site**, and press **Start Smart Scan**.
3. The scan starts at once, without autofocusing, and each image stays as blurred as the start. The log says "Autofocus is off", and the scan's settings (the button by its ID) show `autofocus_enabled: false`.
4. Turn the switch back on for the next scan.

## Design notes and deviations from the plan

- **The switch belongs to `smart_scan`,** not to each workflow, so it's one switch for all four, as the Sequence tab's is one for its sequence. The workflows are told when a scan begins and at each site.
- **One image, without autofocus,** is a basic stack of one at the start's z (`run_basic_stack`). The planners then have no focus to follow, so every site is at the start's z.
- **Histo** still checks for background at each site when it's set to skip it. Only the focusing is switched off.
- **The record** gets `autofocus_enabled` in its settings from `smart_scan`, so the workflows' own settings stay as they were.
- **It's read once,** when a scan starts: turning it over during a scan changes the next one.
- **The switch sits outside the accordions,** so it shows without opening a section, as the Sequence tab's does.
- **A tilted slide** goes out of focus across a scan without autofocus; that's the price of the speed.
- **Size:** 309 changed lines of hand-written code, within M's 600 and my estimate of about 300. Tests are 161 of them.

## Tests

- **`tests/smart_scan.rs`:**
  - a snake scan with autofocus off, started 150 steps from focus, takes one image at each of its 4 sites, all at the start's z, and records and logs that it was off;
  - the default is on, and recorded so.
- **`tests/workflows.rs`:** histo and C-Chip scans with autofocus off keep to the plane they start on, with one image at each site.
- **Mutations,** each caught by these tests:
  - `smart_scan` ignoring the switch;
  - a site taking a smart stack whatever the switch says;
  - C-Chip autofocusing at its start whatever the switch says.
- **`SlideScanPage.spec.ts`:** the switch shows, on by default, and turning it off writes the property.
- **`scan.spec.ts`** (end to end): a snake scan started from the tab with the switch off finishes with one image at each site, and its settings say autofocus was off.
  - **Its two tests run in turn** now, as `workflows.spec.ts`'s do. In parallel, the first run's two scans met, and one couldn't start.
- **The visual baselines:** only Slide Scan's four changed.

## Follow-ups and known gaps

- **The scan settings** still show the autofocus range and the stack settings when autofocus is off, though they're not used then.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
