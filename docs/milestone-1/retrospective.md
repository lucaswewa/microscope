# Milestone 1 retrospective

- Written: 2026-10-15, in P49
- Covers: the 57 phases P00–P49, from 2026-10-08 to 2026-10-15
- Plan: [implementation-plan.md](implementation-plan.md); the phases' notes are in [notes/](notes/README.md)

## What milestone 1 delivered

- **A simulated microscope with the OpenFlexure tabs:** View, Control, Slide Scan, Sequence, Gallery, Settings, the calibration wizard, Logging, About and Power ([plan §6](implementation-plan.md#6-feature-parity-with-openflexure)).
- **The five checkpoints:**
  - A, a live microscope;
  - B, a calibrated one;
  - C, the gallery;
  - D, slide scanning, ending in a stitched deep-zoom image;
  - E, sequences.
- **A launcher** that runs and restarts the server, with the sidecar contract the Tauri app will reuse ([ADR-0032](../adr/0032-sidecar-process-contract.md), [ADR-0033](../adr/0033-local-launcher-and-its-tauri-successor.md)).
- **A Windows zip** with the third-party licences, built and published by a tag ([ADR-0035](../adr/0035-release-packaging-and-third-party-notices.md)).
- **Evidence it holds together:** a workflow suite, visual baselines in both themes, an accessibility scan, and a nightly soak with performance budgets (P47, P48).

## In numbers

| | |
|---|---|
| Phases | 57: the 50 planned, and 7 more from splits |
| Pull requests | 62: one per phase, plus 5 others (working rules, the README, two range-of-motion fixes, and a `.gitignore` line) |
| ADRs | 35, all Accepted |
| Hand-written lines | About 38,900 changed, in the 48 phases that measured them (P08–P48) |
| Tests | About 225 Rust tests; 371 unit tests in the web app; 123 end-to-end, visual and accessibility tests; the nightly soak and budgets |
| Calendar | Eight days |

**Size against budget,** in the 48 phases that measured it:

| Budget | Phases | Over | Median |
|---|---|---|---|
| S (300) | 3 | 0 | 272 |
| M (600) | 20 | 15 | 645 |
| L (900) | 25 | 12 | 900 |

The largest overruns, each kept as one pull request at the owner's choice:

| Phase | Lines | Budget | × |
|---|---|---|---|
| P08 UI controls | 1,868 | L | 2.08 |
| P11 Connection | 1,111 | M | 1.85 |
| P16 Stage | 1,050 | M | 1.75 |
| P36 Smart z-stacks | 967 | M | 1.61 |
| P25 Specimens II | 1,440 | L | 1.60 |
| P10 Client transports | 1,325 | L | 1.47 |
| P34 Scan planners | 842 | M | 1.40 |

Seven phases were split before they were built (P09, P17, P18, P20, P23, P24 and P35), and two were re-sized (P24b and P42), all in [Appendix D](implementation-plan.md#appendix-d-change-log).

## What went well

- **Small phases, each reviewed and merged before the next began.** Every pull request could be read in one sitting, and `main` always worked. Each checkpoint was usable when it landed.
- **ADRs at the point of decision.** Later phases built on decisions they could read, and few were revisited. The interfaces for milestone-2 hardware were walked through against three real targets before any Thing depended on them ([ADR-0018](../adr/0018-hardware-abstraction-through-teta-wot-interfaces.md)).
- **Spec-first notes** kept the OpenFlexure-inspired behaviour in our own words, and the checkbox in each pull request kept the question in view ([ADR-0002](../adr/0002-independent-implementation-of-openflexure-inspired-behaviour.md)).
- **Mutation checks.** Breaking the code on purpose and watching a test fail caught tests that passed for the wrong reason, in the launcher, the sidecar contract and the soak's leak detection.
- **The owner's own runs.** Following a phase's "How to try it" on the owner's computer found what CI couldn't: settings left in a real `.microscope` folder that changed what a test saw (P47), and a failure on the default sample that dense test samples hid (#41).
- **The simulator** was worth its cost. Backlash, a tilted focal plane, travel limits and optical effects gave autofocus, mapping and scanning something real to work against, and switching them at runtime made each testable.
- **Generated API types,** checked in CI, kept the web app and the server from drifting apart ([ADR-0021](../adr/0021-generated-api-types.md)).

## What didn't

- **Size estimates ran low.** 27 of 48 measured phases went over budget, and M phases over more often than not. Three things were under-counted:
  - **Tests,** about a third of the lines, often more;
  - **CSS and comments** in web phases: P08 was about 450 lines of CSS;
  - **The glue** between a phase's parts: types, wiring, error handling and their tests.
- **Timing-dependent tests** flaked under load: the stack test (P41), the stage measurement (P48), and axe's contrast check during a transition (P47). Each was fixed by waiting for a condition rather than a time, but each cost a CI run first.
- **Environment leaks.** Tests read the developer's settings folder, a shared build directory, or a port someone else was using. Each was found, then closed: fresh temporary configurations for the Playwright server, a separate build directory, and fixed port ranges ([CLAUDE.md](../../CLAUDE.md)).
- **Dense test samples hid a sparse-field failure:** the range-of-motion measurement passed on test samples and failed on the default one (#41, #45).
- **Process memory on Windows** is easy to measure wrongly: `PrivateMemorySize64` was misleading, and the soak moved to the private page count (P48).

## Lessons for milestone 2

- **Estimate tests and styles explicitly,** as separate lines in the estimate, and size web phases with their CSS. A phase estimated near its budget should be split before it starts.
- **Test on the default settings and over real distances,** not only on samples chosen to make a test quick.
- **Wait for conditions, not times,** in every test that touches a running server or an animation.
- **Keep each test's environment its own:** settings, build directory and ports.
- **Keep the mutation check** for anything that supervises, recovers or detects: a test that can't fail proves nothing.
- **Real hardware changes the testing:** CI has no stage or camera. Drivers will need fakes at the protocol level (a serial-port fake for the Sangaboard, a recorded SDK session for a camera), and a manual check on the hardware before each merge. See the [readiness checklist](../milestone-2/readiness.md).
