# ADR-0041: Hardware drivers: SDKs, device actors and hardware tests

- Status: Accepted
- Date: 2026-10-20
- Phase: M2 P03a, and P03b for the SDKs
- Amends: [ADR-0005](0005-repository-layout-and-crate-boundaries.md), with a crate allowed unsafe code

## Context

- **Milestone 2's drivers** (plan §5.2) are two serial stages (Zaber, the Sangaboard) and three cameras: two through vendor SDKs (Basler's pylon, Allied Vision's Vimba X) and the Pi's. This phase sets what they share before the first is written.
- **One server binary should run with or without the SDKs** (plan §10.1, approved): CI has no SDK, and a user may have one camera's SDK and not the other's. So the SDKs are loaded at run time, through their C APIs.
- **Calling C needs unsafe code,** even to load a library, since loading runs its initialisers. The workspace forbids unsafe code (ADR-0005), and ADR-0033 went out of its way to keep it so. The owner chose one crate allowed it.
- **`teta-wot`'s `Device<D>`** already runs a driver on a thread of its own: it opens it before its Thing starts, queues calls one at a time, faults the device when a call panics, resets it on request, and closes it when the server stops. A `#[device(init = …)]` field builds the driver from the Thing's configuration.
- **The SDKs' C APIs,** checked against their headers and run on the owner's Windows PC:
  - **pylon** 26.09 has PylonC (`PylonC_v10.dll` on Windows, `libpylonc.so` in `/opt/pylon/lib` on Linux x86_64 and ARM64). It has no version function: the version is the headers' only.
  - **Vimba X** 2026-3 has VmbC 1.3 (`VmbC.dll`, `libVmbC.so`, in its `api` folder), for Windows and Linux x86_64 and ARM64. `VmbVersionQuery` gives its version.
  - **Both** use `__stdcall` on Windows and C elsewhere: Rust's `extern "system"`.
  - **Vimba X uses pylon's GenTL producers too,** where both are installed. Started at the same moment on two threads, Vimba X found none of its 3 cameras; one after the other, it found them all, also with pylon initialised.

## Decision

- **`microscope-sdk`,** a new crate, holds the SDKs' C APIs behind safe functions. It's the workspace's only crate allowed unsafe code:
  - **Its own lints:** the workspace's, without the ban, and with clippy's `undocumented_unsafe_blocks` and `multiple_unsafe_ops_per_block`. Each unsafe block has a `SAFETY:` comment.
  - **Its dependencies:** external crates only. It loads libraries with `libloading` (ISC).
  - **`microscope-things` may depend on it** (ADR-0005's table gains the row). The rest of the workspace still forbids unsafe code.
- **The loader** looks for each SDK's library where its installer puts it, then by name on the system's search path. `MICROSCOPE_PYLON_LIBRARY` or `MICROSCOPE_VIMBA_X_LIBRARY` names it elsewhere. A missing SDK is an `SdkError` that says where it looked, where to get it, and which variable to set. A library that won't load, or lacks a function, is a different error, naming it.
- **One SDK starts at a time:** a lock in `microscope-sdk` is held while either starts or stops.
- **Device actors:** each driver's handle lives in a `teta-wot` `Device<D>` on its Thing, made from the configuration with `#[device(init = …)]`.
  - **Errors:** `drivers::device_error` turns a device's error into an action's, saying what to do ("stage isn't connected: reconnect it").
  - **Reconnecting:** each driver's Thing has a `reconnect` action, which resets its device: the driver is closed, then made again.
  - **Starting:** a device that can't open stops the server from starting, with its error, as a Thing that can't start does.
  - **Shutdown:** the device closes the driver when the server stops.
- **Serial ports** are `drivers::serial::Line`: send a line, receive a line within a timeout. It's what Zaber's and the Sangaboard's text protocols need.
  - **The real port** is `serial2`'s (BSD-2-Clause or Apache-2.0).
  - **The fake** is a `Script`: the lines a driver must send, each with the device's answers, or a failure, as an unplugged device gives. A test keeps a clone to check what was sent.
- **Hardware tests** run only when an environment variable names their device (`MICROSCOPE_ZABER_PORT=COM3`), through `drivers::hardware_device`; otherwise they pass, saying they were skipped. The variables are in [hardware.md](../milestone-2/hardware.md). The SDKs' own tests run wherever the SDK is installed, since they need no camera.
- **Configurations** name a stage by its serial port, `port`, and a camera by its serial number, `serial_number`. They go in `configs/`, one per hardware set, named for its devices: `basler-zaber.json`, `pi-sangaboard.json`. Each driver phase adds its set's.
- **Listing devices:** `microscope-server --list-devices` prints what each driver finds: the serial ports, and each SDK's cameras, or why it can't look. The camera drivers (P07, P08) list their cameras by serial number.

## Consequences

- **One binary for everyone,** and CI builds and tests every driver, against fakes.
- **Unsafe code is in one place,** small and reviewed: the functions each driver calls, declared from the vendors' headers. A wrong declaration there is undefined behaviour, which no test can rule out, so each is checked against the header when it's added.
- **A user without an SDK** sees which and how to get it, from `--list-devices` or from the server's start, never a crash.
- **A device that's unplugged at start** stops the server starting. One lost while running fails its calls until it's reconnected.
- **The release's licences** accept ISC, for `libloading`, and cargo-about lists the crates of all three platforms.
- **The SDKs' library names** may change with their versions (`PylonC_v10`); the loader's names change with them.

## Alternatives considered

- **Building against the SDKs** (the `pylon-cxx` crate, Vimba X's C headers) behind cargo features: no unsafe code of ours. But each SDK must be installed wherever the server is built, CI can't build the camera drivers, and a release needs a build per set of SDKs.
- **Unsafe code in each driver's module** of `microscope-things`: the exception would spread through the crate with the most code.
- **A separate process per SDK,** in C or C++, talking to the server: no unsafe Rust, but a second toolchain, a protocol, and frames copied between processes.
- **`serialport` for serial ports:** it also gives ports' USB IDs, but it's MPL-2.0, a copyleft licence the release would have to carry. `serial2` is permissive, and its port names are enough to write a configuration.
- **Hardware tests marked `#[ignore]`,** run with `--ignored`: the device's name would still need passing, so a variable does both.
