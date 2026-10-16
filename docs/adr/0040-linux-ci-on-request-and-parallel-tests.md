# ADR-0040: Linux CI on request, and the Rust tests in parallel

- Status: Accepted
- Date: 2026-10-16
- Phase: outside a phase, after P01 (milestone 2)
- Supersedes: [ADR-0038](0038-milestone-2-platforms.md), in part: Linux CI on every pull request

## Context

- **ADR-0038 runs CI on all three platforms** for every pull request, and P01 made it so.
- **The owner wants Linux only when asked:** most pull requests don't touch anything platform-specific, and the Linux jobs double the runner time.
- **The wait isn't Linux's:** the jobs run at once, so a pull request took as long before P01 as after, 12–14 minutes. That's the Rust job, whose Test step took 11.5 minutes on Windows.
- **Most of that is six simulator test files,** run one after another, about 10 minutes together:

  | Tests | Seconds |
  |---|---|
  | `workflows` | 127 |
  | `stage_measure` | 125 |
  | `smart_scan` | 86 |
  | `stack` | 80 |
  | `mapping` | 64 |
  | `autofocus` | 55 |

  Within each, the simulator-heavy tests take turns, as they share the CPU with their cameras.

## Decision

- **Every check for a platform is one reusable workflow,** `platform.yml`:
  - the Rust checks (format, clippy, docs);
  - the Rust tests;
  - the web app's checks;
  - the end-to-end suite.
- **The Rust tests run as three jobs at once,** each on its own runner:
  - **simulator 1:** `workflows`, `mapping` and `autofocus`;
  - **simulator 2:** `stage_measure`, `smart_scan` and `stack`;
  - **the rest:** every other crate whole, and `microscope-things`' unit tests, doc tests and other test files. It finds them from `cargo metadata`, so a new test file runs there without being listed.
- **Windows on every pull request:** `ci.yml` runs `platform.yml` for Windows x86_64, with the contract tests.
- **Linux when asked:** `linux.yml` runs `platform.yml` for Linux x86_64 and ARM64, the end-to-end suite on x86_64 only. It runs:
  - on a pull request labelled **`linux`**, and on each push to it while it is;
  - or by hand, on any branch.
- **When to ask:** a pull request that changes platform-specific code, the launcher, packaging, Linux or the Pi gets the label, as does each checkpoint's.

## Consequences

- **A shorter wait:** the slowest of the three test jobs, not their sum. Expected about 6–7 minutes a pull request; its pull request reports the measured time.
- **More runner time for Windows,** as each test job builds the workspace: their cache is shared, saved by the job that builds everything.
- **`main` isn't checked on Linux unless asked.** A change that breaks only Linux shows up at the next labelled pull request, or a manual run.
- **Job names change:** "Windows x86_64 / Rust tests (simulator 1)" and so on. Nothing requires checks by name (the ruleset on `main` only forbids deleting it and force-pushing).
- **The groups are by hand:** a test file that grows slow should move into a simulator group, to keep the three balanced.

## Alternatives considered

- **cargo-nextest, partitioning the tests across jobs:** automatic balance, but it runs each test in a process of its own. The simulator tests' in-process turn-taking would then be lost, and with it their frame rates and their reliability.
- **Running the test binaries at once in one job:** no extra builds, but the simulator tests would share four cores, which is what made autofocus tests flaky in milestone 1.
- **Linux on a schedule:** nightly, it would catch Linux breakage on `main` within a day, at a cost every day. It can be added if Linux breaks unnoticed.
