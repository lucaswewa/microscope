# P48: End-to-end, soak and performance

- Status: Done
- Pull request: [#61](https://github.com/lucaswewa/microscope/pull/61)
- ADRs: none new
- Spec: [phases.md#p48](../phases.md#p48)

## Summary

Evidence that the milestone works as a whole and keeps working.

- **A workflow suite** against the real server, from new tests and earlier ones:

  | Workflow | Test |
  |---|---|
  | Connect, jog, autofocus, capture, and see it in the gallery | `workflows.spec.ts` (new): one first session, start to finish |
  | The calibration wizard and camera–stage mapping | `connection.spec.ts` (P29) |
  | A snake scan ending in a stitched DZI | `scan.spec.ts` (P40) |
  | A histo scan | `workflows.spec.ts` (new): from the Slide Scan tab, ending stitched |
  | A sequence | `sequence.spec.ts` (P42) |
  | The Logging tab | `logging.spec.ts` (P43) |
  | Restart through the launcher | `launcher.spec.ts` (new): the app served by a launcher's server, restarted from the Power tab |

- **A soak test:** a fresh server streams to the app and scans, stitching each scan, for 30 minutes. Its memory and handles mustn't grow.
- **Budgets,** measured and checked: the preview's frame rate, a scan's time per tile, and how quickly the app changes tabs.
- **A nightly workflow** runs the soak and the budgets against a release build, and keeps their measurements.

## What was built

| Where | What |
|---|---|
| `web/tests/e2e/workflows.spec.ts` | A first session, and a histo scan |
| `web/tests/e2e/launcher.spec.ts` | Restart through the launcher |
| `web/tests/long/soak.spec.ts` | The soak |
| `web/tests/long/budgets.spec.ts` | The budgets |
| `web/tests/long/measure.ts` | Frame rate, time per tile, tab changes, and a process's memory and handles |
| `web/tests/servers.ts` | Starting servers and running actions, shared with `connection.spec.ts` |
| `web/playwright.config.ts` | A `workflows` project after the sequence's, and a `long` one, only with `MICROSCOPE_LONG` |
| `.github/workflows/nightly.yml` | The nightly workflow |
| `.github/workflows/ci.yml` | The end-to-end job builds the launcher too |
| `README.md` | Running the long tests |

## How to try it

```powershell
cd web
npx playwright test
```

1. **The whole suite** passes, now with the `workflows` project last: a first session, then a histo scan. The launcher's test runs with the others; it builds the launcher if it needs to.
2. **The long tests,** better against a release build:

   ```powershell
   npm --prefix web run build
   cargo build --release -p microscope-server
   cd web
   $env:MICROSCOPE_LONG = '1'; $env:MICROSCOPE_SOAK_MINUTES = '5'
   $env:MICROSCOPE_SERVER = 'target/release/microscope-server.exe'
   npx playwright test --project=long
   ```

   `test-results` then holds `budgets.json` and `soak.json`. Five minutes is a short soak; the nightly runs thirty.

## The budgets

Measured locally against a release build, as the nightly measures them, on the default simulated sample.

| Budget | Limit | Measured |
|---|---|---|
| Preview, main and low-resolution streams | At least 8 frames a second | 10.2 and 10.2 |
| A quick snake scan's time per tile | At most 12 s | 9.5 s, then 6.3 s |
| Changing tabs: the median, and the slowest | At most 100 ms and 500 ms | 15 ms and 16 ms, twice |

- **The preview** runs at its 10 fps target, set in P17.
- **A tile** is a smart stack: an autofocus sweep at the simulated stage's 500 z steps a second, then nine images to find the sharpest. The xy move and the capture add a little.
- **A tab change** is from the rail link's click until the new page's heading shows, measured in the page, once each page's code has loaded.

## The soak

Thirty minutes against a release build, locally, as the nightly runs it: the live image streaming to the app throughout, and 61 snake scans of 2 × 2 tiles, each stitched.

| | First sample | The first half's median | The second half's median | Range |
|---|---|---|---|---|
| Committed private memory | 20.6 MB | 10.0 MB | 11.2 MB | 9.2–22.5 MB |
| Working set | 40.2 MB | 29.5 MB | 30.2 MB | 28.6–42.0 MB |
| Handles | 141 | 141 | 141 | 141–143 |
| Seconds per tile | 6.5 | | | 6.5–12.1 |

- **Nothing grows:** memory settles after the first scans, and the handles hardly move. The halves are taken after the first fifth.
- **By the trend,** as the test measures it, memory grew 0.034 MB a scan, 1.6 MB over the steady scans, and the handles not at all.
- **The time per tile** varies with where each autofocus starts, and a 2 × 2 scan spreads its start over fewer tiles than the budget's 3 × 2.

## Design notes and deviations from the plan

- **The workflow suite** is the earlier tests plus three new ones, rather than a separate copy. The first session goes through its steps as one user would, where the earlier tests take them apart.
- **The workflows run last,** in a project of their own after the sequence's, since they move the stage and hold the microscope.
- **The histo scan** turns background skipping off and keeps to its smallest range, 1,000 steps, with quick captures, so it takes about a minute. It chooses its workflow in the tab, as someone would: the snake test leaves the snake chosen.
- **The launcher's test** builds only the launcher. Other tests run the server's binary meanwhile, which Windows won't let Cargo replace; the server beside it is Playwright's own build. CI builds both in release.
- **The soak's memory** is the process's committed private memory, from Windows (`PrivatePageCount`), with its working set beside it. Windows PowerShell's `PrivateMemorySize64` gave the same number as the paged memory, so I didn't rely on it.
- **The soak's checks:** after a fifth to warm up, the growth over the rest, by the least-squares trend of the samples, is at most 16 MB of memory and 8 handles. Every scan must complete.
  - **A trend,** not single samples, since memory goes up and down by a few megabytes as scans and stitching come and go.
  - **Over half an hour,** that catches a leak of a megabyte a scan, or a handle every five scans. A short run's noise stays within it.
  - **First,** I compared the halves' medians, with 20% and 32 MB to spare. Worked through, that would have let a leak of 1 MB a scan pass half an hour, so I changed it.
- **The soak catches a leak:** a release server leaking 4 MB a scan, run for five minutes, failed, with 29.6 MB of growth against the 16 allowed.
  - **The first leak I tried** was optimised away: the compiler removed an allocation nothing used. The second kept it with `std::hint::black_box`.
  - **The source** was restored before each run, and the clean server rebuilt after.
- **The long tests** run only with `MICROSCOPE_LONG`, and are never retried: a failed half hour isn't run again.
- **The nightly workflow** also runs on a pull request that changes it, so this one shows it working; scheduled runs start once it's on `main`.
- **Size:** 576 changed lines of hand-written code, within L's 900 and my estimate of 500–700.

  | Part | Lines |
  |---|---|
  | The workflows and the launcher's test | 145 |
  | The soak, the budgets and their measurements | 229 |
  | The shared servers, and `connection.spec.ts` using them | 113 |
  | Playwright's projects and configuration | 31 |
  | The workflows in CI | 58 |

## Tests

- **`workflows.spec.ts`:**
  - **a first session:** connected, jog right, autofocus, capture, then the capture first in the gallery, opened in the viewer;
  - **a histo scan,** from the Slide Scan tab: its live preview, then its card's stitched thumbnail.
- **`launcher.spec.ts`:** a launcher on a free port. The app it serves says "Managed by the launcher", Power's Restart brings back a new server process on the same address, and the app reconnects.
- **`soak.spec.ts` and `budgets.spec.ts`,** as above.

## Follow-ups and known gaps

- **A hung server** still isn't noticed by the launcher (ADR-0033); the soak only shows the server doesn't leak.
- **The budgets are checked nightly,** not on every pull request: shared CI machines vary too much for timing to gate a merge.
- **The soak uses the snake workflow,** at 2 × 2 tiles. The histo scan's background detection and the sequence engine aren't soaked.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
