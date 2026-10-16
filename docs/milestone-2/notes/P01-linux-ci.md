# P01: Linux builds and CI

- Status: In review
- Pull request: [#72](https://github.com/lucaswewa/microscope/pull/72)
- ADRs: [ADR-0039](../../adr/0039-teta-wot-on-linux.md)
- Spec: [phases.md#p01](../phases.md#p01)

## Summary

The workspace builds and passes its tests on Linux x86_64 and Linux ARM64, as on Windows, and CI checks all three on every pull request.

- **CI runs on three platforms:**
  - the Rust job (format, clippy, tests, docs) and the web app's job on Windows x86_64, Linux x86_64 and Linux ARM64;
  - the end-to-end suite on Windows and Linux x86_64, with the visual baselines on Windows only.
- **`teta-wot` needed no change** (ADR-0039). It's portable apart from its stop signal, which on Linux heard only Ctrl-C. The server now also stops cleanly on SIGTERM, which systemd sends.
- **Three things Linux showed up,** each fixed:
  - a data-route test made its link with Windows' `mklink`;
  - the data folder's paths took `C:` and `\` as a drive and a separator on Windows only;
  - a launcher test used Windows paths.

## What was built

| Where | What |
|---|---|
| `.github/workflows/ci.yml` | Each job on a matrix of platforms, keeping the Windows jobs' names |
| `crates/microscope-server/src/cli.rs` | `stop_signal`: `teta-wot`'s shutdown signal, and SIGTERM on Unix |
| `crates/microscope-server/tests/cli.rs` | On Unix: SIGTERM stops the server, which exits 0 |
| `crates/microscope-things/src/data.rs` | `resolve` refuses `:` and `\` on every platform |
| `crates/microscope-server/tests/data.rs` | A symbolic link on Unix where Windows makes a junction |
| `crates/microscope-launcher/src/setup.rs` | Its test's paths built from the temporary folder, absolute everywhere |
| `web/playwright.config.ts` | The visual tests on Windows only |
| `web/tests/e2e/launcher.spec.ts` | Skipped off Windows, until P02 ends the server with the launcher there |

## How to try it

- **CI:** the pull request's checks list nine jobs: Rust and the web app on all three platforms, the end-to-end suite on Windows and Linux x86_64, and the contract tests.
- **On a Linux machine:** `cargo test --workspace --locked`, then in `web/`, `npm ci && npm run test:unit`. `npx playwright test` runs the end-to-end suite, without the visual tests.
- **SIGTERM, on Linux:** start a server (`cargo run -p microscope-server -- -c configs/simulation.json`), then `kill <its pid>`. It logs "received SIGTERM", stops its Things, and exits 0.

## Hardware

None: the phase has no hardware. Linux ran on GitHub's runners: Ubuntu x86_64 (`ubuntu-latest`) and Ubuntu ARM64 (`ubuntu-24.04-arm`). This PC has no WSL, so Linux was tried only there.

## Design notes and deviations from the plan

- **The jobs keep their Windows names,** "Rust (Windows x86_64)" and so on, so anything that waits for them still finds them. The new ones are named for their platform.
- **The contract job stays on Windows:** the API is the same everywhere, and the job's PowerShell starts and stops its server. The end-to-end suite on Linux exercises the same API through the app.
- **Paths refused everywhere:**
  - **Before,** on Linux, `C:/a.jpg` and `a\b.jpg` were names inside the data folder, harmless but answered differently from Windows.
  - **Now** both are refused on every platform. The app's own paths always use `/`.
  - **One effect:** the simulator's sample-image setting, typed with `\` on Windows, is now refused, and needs `/`.
- **SIGTERM in this app, not `teta-wot`** (ADR-0039): a few lines here, against a `teta-wot` release.
- **The launcher on Linux** compiles, and its unit tests pass. It doesn't yet end its server with it, or hear SIGTERM, or keep its folders in XDG's places: all P02. Its integration tests and its end-to-end test stay Windows-only until then.
- **The visual baselines** stay Windows images (ADR-0034, ADR-0038). The accessibility scan, which opens the same pages, runs on Linux too.
- **Size:** 163 changed lines of hand-written code, within M's 600.

## Tests

- **On Linux x86_64 and ARM64,** in CI:
  - 227 Rust tests pass: Windows' 231, less the launcher's five Windows-only integration tests, plus the SIGTERM test;
  - the web app's checks and its 383 unit tests;
  - on x86_64, 86 end-to-end tests: all but the visual ones and the launcher's.
- **The SIGTERM test** would fail without `stop_signal`: SIGTERM's default action ends the process with no exit code, where the test asks for 0. It ran and passed on both Linux runners.
- **On Windows,** locally and in CI, everything as before.

## Follow-ups and known gaps

- **P02:** the launcher on Linux, the systemd service, and Linux release archives.
- **The nightly workflow** (the soak and the budgets) stays on Windows: its memory readings use Windows' process counters.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
