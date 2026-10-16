# Working on this repository

A microscope service: a Rust backend on [`teta-wot`](https://github.com/lucaswewa/teta-wot) (v0.1.0) and a Vue 3 web app (`web/`) inspired by OpenFlexure. Milestone 1 built it against a simulated camera and stage. Milestone 2 adds real cameras and stages, Linux and the Raspberry Pi 5, correlation stitching, authentication, and a Tauri desktop app. Start from [`docs/README.md`](docs/README.md); the current plan is [`docs/milestone-2/implementation-plan.md`](docs/milestone-2/implementation-plan.md), every phase's spec is in [`phases.md`](docs/milestone-2/phases.md), and the devices are in [`hardware.md`](docs/milestone-2/hardware.md).

## How work is delivered

- **Phase by phase, in the plan's order.** The plan's §7 table shows each phase's status (milestone 1's was its §8).
- **One branch and one pull request per phase.** Name the branch `m2/pNN-short-name` and the pull request `PNN: <title>`.
- **The project owner approves every pull request before it is merged.** Never merge without that approval. Start the next phase only after the merge, from an up-to-date `main`.
- **Each pull request contains:**
  - the code and its tests;
  - the phase's notes, `docs/milestone-2/notes/PNN-short-name.md`, from the template, with its Hardware section;
  - any new ADRs, with the status *Accepted*, and their rows in the ADR index;
  - the phase marked *In review* in plan §7 and in the notes index.

  The next phase's pull request marks the previous phase *Done* in plan §7, in the notes index, and on its notes' Status line.
- **A driver phase is Done only after the owner has run its "How to try it" on the device.** CI can't reach hardware.
- **The size budget** (S ≤ 300, M ≤ 600, L ≤ 900) counts every hand-written changed line: source, CSS, tests, comments and blank lines. Lock files, generated bindings and docs don't count.
  - Estimate a phase's size before building it. If it will be well over budget, ask the owner whether to split it or keep one pull request.
  - Report the measured size honestly in the notes and the pull request.
  - A split updates `phases.md`, the plan's phase overview and its change log.
- **Changes outside a phase,** like this file, still go through a branch and an approved pull request.

## Rules

- **Independent implementation (ADR-0002).** OpenFlexure is GPL-3.0 and this repository is MIT. Copy none of its code, styles, assets or text. Its reference screenshots stay out of git.
- **Dependencies.**
  - Their licences must allow MIT distribution.
  - Install only versions at least two weeks old: `npm install --before=<date two weeks ago>`.
  - Check new packages' licences, and `npm audit` against `main`.
- **The web app** keeps the rules in ADR-0010, ADR-0011 and ADR-0013, which ESLint partly enforces:
  - hash routing, and no direct platform APIs outside `src/host/`;
  - only design tokens for styles;
  - only `src/ui/` imports `reka-ui`.
- **Platforms:** Windows x86_64, Linux x86_64, and Linux ARM64 on a Raspberry Pi 5 (ADR-0038). CI builds and tests on all three.
  - Keep platform-specific code in small `cfg` blocks, each with an equivalent elsewhere or a clear message where there's none.
  - Line endings are LF. Scripts that write files on Windows must not turn them into CRLF.
  - Playwright's visual baselines are Windows images (`*-chromium-win32.png`), so they don't match on Linux, and are checked on Windows only.
- **Hardware** ([`docs/milestone-2/hardware.md`](docs/milestone-2/hardware.md)):
  - A test that needs a device runs only when its environment variable names the device (such as `MICROSCOPE_ZABER_PORT`), and is skipped otherwise.
  - Every driver also has a fake at its protocol or SDK boundary, for CI.
  - Never redistribute an SDK (pylon, Vimba X); the user installs it.
  - Write protocols, such as the Sangaboard's, from their documentation, in our own words (ADR-0002).
- **Manual checks** use their own ports: 5090–5097 for `microscope-server`, 5190–5199 for Vite (`--strictPort`). The end-to-end tests use 5098 and 5099 for their servers. The owner runs the server on 5000 and Vite on 5173. Stop only processes you started, by their process ID; never stop "whatever listens on a port".

## Checks before opening a pull request

These match CI (`.github/workflows/ci.yml`):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked

cd web
npm run format:check && npm run lint && npm run typecheck && npm run test:unit && npm run build
npm run api:types -- --check   # the generated API types match openapi.json
# When a Thing's affordances change, regenerate them (ADR-0021):
npm run api:openapi && npm run api:types
npm run test:e2e    # end-to-end and visual tests, against the app as a server built from the checkout serves it
# with a server running (see README):
MICROSCOPE_API_URL=http://127.0.0.1:5090/api/v1/ npm run test:contract
```

For a UI change, also look at it in a browser in both themes. The development-only galleries are at `/#/dev/components` and `/#/dev/tokens`.
