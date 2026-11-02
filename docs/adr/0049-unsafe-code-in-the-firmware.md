# ADR-0049: Unsafe code in the firmware

- Status: Accepted
- Date: 2026-11-02
- Phase: OPTIMA E06
- Amends: [ADR-0041](0041-hardware-drivers.md), which allowed unsafe code only in `microscope-sdk`

## Context

Firmware sits on the hardware: startup code, linker sections and registers no driver covers. Embassy keeps `unsafe` out of most of it, but not always all of it. The repository's rule allowed `unsafe` only in `microscope-sdk`. The [electronics plan's](../misc/optima_electronics_plan.md#7-decisions-for-the-owner) decision 5 asked the owner, who decided on 2026-11-02 to allow it in `firmware/optics/`.

## Decision

- **`optics-core` forbids it** (`#![forbid(unsafe_code)]`). Nothing there has a reason to need it.
- **`optics-fw` denies it by default** (`#![deny(unsafe_code)]`). Each use needs its own `#[allow(unsafe_code)]`, with a `SAFETY:` comment saying why it holds.
- **The server's workspace is unchanged:** only `microscope-sdk` may use it there.

So far there's one use, which isn't a block: the `#[unsafe(link_section = ".bi_entries")]` attribute on the data `picotool info` reads. Embassy's own macros and crates have their own `unsafe`, which is outside this rule.

## Consequences

- **The compiler keeps the list:** a new use fails to build until someone allows it and explains it, so a search for `allow(unsafe_code)` finds them all.
- **The logic stays safe,** because decisions live in `optics-core`.
- **E07 may add uses,** such as registers for sequences' timing, under the same rule.

## Alternatives considered

- **No `unsafe` at all:** that would give up the program's description for `picotool`, and could block E07's timing, with nothing gained over reviewing a few marked places.
- **Allowing it across the firmware's workspace:** this would let it into the logic that the PC's tests rely on, where it isn't needed.
