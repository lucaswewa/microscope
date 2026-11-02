# ADR-0048: The optics firmware's workspace

- Status: Accepted
- Date: 2026-11-02
- Phase: OPTIMA E06

## Context

The optics controller is a Pico 2 (RP2350) on the carrier board. Its firmware is `no_std` Rust for an Arm target, while the server is `std` Rust for the PC. The [electronics plan](../misc/optima_electronics_plan.md) proposes Rust with Embassy, in its own Cargo workspace, with its logic tested on the PC. The firmware's bugs can light an LED, so as much of it as possible should be testable without a chip.

## Decision

- **Its own workspace:** `firmware/optics/`, with its own `Cargo.lock` and `rust-toolchain.toml`. The toolchain is the server's version, with the RP2350's target. The server's workspace doesn't change.
- **Two crates:**
  - `optics-core` holds every decision: the protocol, the interlock's rules, the channels, the triggers. It's `no_std`, forbids `unsafe` and has no dependencies, so it tests on any PC. It's the workspace's default member, so a plain `cargo test` takes it alone.
  - `optics-fw` builds only for `thumbv8m.main-none-eabihf`. It reads pins and USB, passes on what they say, and sets pins.
- **Embassy** for the hardware layer: `embassy-rp`, `embassy-executor`, `embassy-usb` and their relatives, under MIT or Apache 2.0.
- **One CI job,** on every pull request: formatting, Clippy, tests, the build, and the `.uf2` file as an artifact. It runs on Linux, since nothing in it depends on the host, and `.cargo/config.toml` holds aliases, `cargo fw` and `cargo fw-clippy`, for the target's commands.
- **Our own UF2 maker,** `make_uf2.py`, so that loading a Pico 2 needs no tool beyond Python. `picotool` can read what it writes.

## Consequences

- **The logic is tested without a chip:** the interlock's rules, every command and all the parsing. The pins, USB and timing are tested only on the bench, so `optics-fw` has to stay too simple to get wrong.
- **Two sets of dependencies** to keep at least two weeks old and licence-checked. The firmware's are all MIT-compatible (mostly MIT or Apache 2.0; `rp-pac` is BSD-3-Clause).
- **The `.uf2` file isn't committed,** unlike the CAD's built files. It's a build artifact, since a binary's bytes aren't a useful thing to review.
- **Release packaging isn't affected.** If the firmware is ever distributed, its binary needs its dependencies' notices (ADR-0035).

## Alternatives considered

- **Members of the server's workspace:** `cargo test --workspace` and Clippy would try to build the firmware for the PC, and the server's checks would need a target and a filter everywhere.
- **The Pico SDK, in C:** it's the maker's own, but the rest of this project is Rust, and it has no equivalent of testing the logic this simply. It would also bring a second language to review.
- **RTIC, or plain `rp235x-hal`:** both work. Embassy's USB and async pins take less code here, and E07's slider and sequences suit its tasks. Decisions in `optics-core` don't depend on the choice.
- **`picotool` or `elf2uf2-rs` for the UF2 file:** both are more to install on CI and on a bench PC, for a format simple enough to write.
