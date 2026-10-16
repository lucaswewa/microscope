# ADR-0039: `teta-wot` on Linux

- Status: Accepted
- Date: 2026-10-16
- Phase: P01 (milestone 2)

## Context

- **ADR-0038** adds Linux x86_64 and Linux ARM64 to milestone 2's platforms.
- **ADR-0004** depends on `teta-wot` v0.1.0 without changing it. `teta-wot` is built and tested on Windows only, and the plan asked whether Linux might need changes in it ([plan §10](../milestone-2/implementation-plan.md#10-points-for-your-review), point 2).
- **What's platform-specific in `teta-wot` v0.1.0:**
  - **`shutdown_signal`:** on Windows it completes on Ctrl-C, Ctrl-Break, the console closing, log-off and shutdown; elsewhere on Ctrl-C (SIGINT) alone.
  - **Running as a Windows service,** behind a feature this app doesn't use.
- **Nothing else** is Windows-specific: its dependencies (tokio, axum and the rest) are portable, and it needs no system libraries.
- **A service manager stops a program with SIGTERM:** systemd does, and so does `kill` by default. `teta-wot`'s `shutdown_signal` doesn't listen for it.

## Decision

- **`teta-wot` stays as it is,** at v0.1.0, on every platform: ADR-0004 stands.
- **This app adds SIGTERM:** on Unix, the server stops on `teta-wot`'s `shutdown_signal` or on SIGTERM, whichever comes first, and stops as cleanly either way.
- **CI proves it:**
  - this app's tests, which run `teta-wot`'s runtime, server and HTTP layer, pass on Linux x86_64 and ARM64;
  - a Unix test stops a server with SIGTERM and checks it exits 0.
- **If `teta-wot` ever needs a change** for Linux, that's a new ADR, superseding ADR-0004 for it.

## Consequences

- **No upstream work,** and no fork: the same `teta-wot` tag on every platform.
- **SIGTERM is handled in one place,** the server's `stop_signal`, so the launcher (P02) and a systemd service stop it alike.
- **`teta-wot`'s own tests** still run only in its own CI, on Windows. This app's tests cover the parts it uses, on all three platforms.

## Alternatives considered

- **Adding SIGTERM to `teta-wot`'s `shutdown_signal`:** the right place in the long run, but it needs a `teta-wot` release, and this app can do it in a few lines.
- **Running the server under a wrapper that turns SIGTERM into SIGINT:** another moving part on every Linux install, for what the server can do itself.
