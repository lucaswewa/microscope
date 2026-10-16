# ADR-0038: Milestone-2 platforms: Windows, Linux and the Raspberry Pi 5

- Status: Accepted
- Date: 2026-10-16
- Phase: P00 (milestone 2)
- Supersedes: [ADR-0003](0003-milestone-1-platform-windows-x86-64.md)

## Context

- **ADR-0003** built, tested and supported milestone 1 on Windows x86_64 only, and promised to revisit Linux and the Raspberry Pi once milestone 2's hardware was fixed.
- **It now is** ([plan §3](../milestone-2/implementation-plan.md#3-decisions-made-while-planning)):
  - Basler and Allied Vision cameras and Zaber stages, on PCs;
  - OpenFlexure hardware, the Pi camera and the Sangaboard, on a Raspberry Pi 5;
  - and the industrial devices on the Pi too.
- **The SDKs support Linux:** pylon and Vimba X both ship for Windows x86_64, Linux x86_64 and Linux ARM64, though neither lists the Pi 5 among the boards it tests.
- **The code stayed portable** (ADR-0003): only the launcher has Windows-only parts, behind `cfg(windows)`. `teta-wot` is built and tested on Windows only.
- **GitHub offers ARM64 Linux runners** for public repositories, which this one is.

## Decision

- **Three targets,** built, tested and released:

  | Target | What runs there |
  |---|---|
  | Windows x86_64 | The server, the launcher, and the desktop app (P17) |
  | Linux x86_64 | The server, the launcher, a systemd service (P02), and the desktop app |
  | Linux ARM64, on a Raspberry Pi 5 with the 64-bit Raspberry Pi OS | The server, headless, as a systemd service |

- **CI builds and tests on all three,** on GitHub's hosted runners, natively rather than cross-compiled: Windows, Ubuntu x86_64 and Ubuntu ARM64.
  - **The Rust tests and the web app's unit tests** run on all three.
  - **The end-to-end suite** runs on Windows and Linux x86_64.
  - **The visual baselines stay Windows images** (ADR-0034), checked on Windows only.
- **Platform-specific code** stays in small `cfg` blocks, as ADR-0003 asked, each with an equivalent on the other platforms or a clear message where there's none.
- **Each release** has a Windows zip (and later installer), Linux x86_64 and ARM64 archives, and a `.deb` with the service (P02).
- **The desktop app is for Windows and Linux PCs,** not the Pi. A Pi with a screen can still use a browser.

## Consequences

- **More to keep green:** three CI jobs where there was one, and the ARM64 job is slower.
- **Linux needs its own code paths:**
  - for the launcher, its folders, the process lifetime and signals (P02);
  - for `teta-wot`, whatever P01 finds (ADR-0039).
- **The vendors don't test on a Pi:** pylon and Vimba X are tried on the Pi 5 early (P07, P08). If either fails there, it stays a PC camera, and the notes say so.
- **One set of visual baselines:** Linux renders fonts differently, so its screenshots aren't compared. The Linux end-to-end run checks behaviour, not pixels.

## Alternatives considered

- **Cross-compiling ARM64 from x86_64 runners:** no ARM machine needed, but tests couldn't run on the target, and the SDKs and libcamera would need sysroots. Native ARM64 runners run the tests where the code runs.
- **Linux x86_64 only, the Pi later:** less to support, but it would drop OpenFlexure hardware, which the owner wants on the Pi in milestone 2.
- **The Pi only as a client:** the server on a PC, driving the Pi's camera remotely. That's more parts, and the Pi camera and the Sangaboard are made to be driven from the Pi.
