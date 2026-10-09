# ADR-0003: Milestone-1 platform: Windows x86_64

- Status: Accepted
- Date: 2026-10-08
- Phase: P00

## Context

- `teta-wot` supports **Windows x86_64** only. It is built and tested there, and its shutdown signals and Windows-service support are Windows-specific.
- Development happens on Windows.
- Milestone 2 targets three kinds of hardware: OpenFlexure hardware, which normally runs on a Raspberry Pi (Linux, ARM64); industrial camera SDKs, which generally ship for Windows and Linux; and µm-native motorised stages ([plan §3, D10](../milestone-1/implementation-plan.md#3-decisions-made-while-planning)).
- A Tauri desktop app follows later and will need its own platform decision.

## Decision

Milestone 1 builds, tests and supports **Windows x86_64** (MSVC toolchain) only:

- CI runs on `windows-latest`, and visual-regression baselines are Windows baselines.
- The code stays portable where that costs little. It uses standard-library paths and APIs rather than Windows ones, and confines platform-specific code (for example the launcher's Job Object, P45) to small, clearly marked `cfg(windows)` modules. On other platforms those modules either have a stand-in or fail to compile with a clear message.
- The Linux and Raspberry Pi question is revisited at the start of milestone 2, when the hardware targets are fixed.

## Consequences

- One platform to build, test and support keeps milestone 1 focused, and matches `teta-wot`.
- Supporting a Raspberry Pi later may need work in `teta-wot` and cross-compilation in CI. Keeping the code portable reduces, but doesn't remove, that work.
- Contributors on macOS or Linux can't run the full stack in milestone 1.

## Alternatives considered

- **Windows and Linux x86_64 from the start.** Broader reach, but `teta-wot` is unverified on Linux, and CI and debugging effort would double before any hardware needs it.
- **Windows and Raspberry Pi.** It would match OpenFlexure's hardware, but cross-compiling and testing on ARM64 costs effort now, for hardware that milestone 1 doesn't use.
