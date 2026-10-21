# P03b: The SDKs at run time, and listing devices

- Status: In review
- Pull request: [#76](https://github.com/lucaswewa/microscope/pull/76)
- ADRs: [ADR-0041](../../adr/0041-hardware-drivers.md), from P03a
- Spec: [phases.md#p03b](../phases.md#p03b)

## Summary

The camera SDKs load at run time where they're installed, and say how to get them where they aren't. `microscope-server --list-devices` prints what every driver finds, for writing a configuration.

- **`microscope-sdk`,** a new crate, loads pylon's and Vimba X's C APIs. It's the workspace's only crate with unsafe code, as ADR-0041 decided.
- **On the owner's Windows PC,** both load and work: pylon finds no cameras, and Vimba X 3.
- **The server runs without them,** as in CI.

## What was built

| Where | What |
|---|---|
| `crates/microscope-sdk/src/lib.rs` | The loader: `SdkError` (not installed, unusable, or a call that failed), the search, and a lock so one SDK starts at a time |
| `crates/microscope-sdk/src/pylon.rs` | `Pylon`: `load`, `path`, `count_cameras` |
| `crates/microscope-sdk/src/vimba.rs` | `VimbaX`: `load`, `path`, `version`, `count_cameras` |
| `crates/microscope-things/src/drivers/mod.rs` | `list_devices`: what each driver finds, or why it couldn't look |
| `crates/microscope-things/src/drivers/serial.rs` | `ports`: the serial ports |
| `crates/microscope-server/src/cli.rs` | `--list-devices` |
| `release/about.toml` | ISC accepted, for `libloading` |
| `CLAUDE.md` | Unsafe code only in `microscope-sdk` |

### Where each SDK is looked for

| SDK | Windows | Linux |
|---|---|---|
| pylon | `PylonC_v10.dll` in `Runtime\x64`, in `PYLON_DEV_DIR`'s folder or `Program Files\Basler\pylon`; then on `PATH` | `libpylonc.so` in `$PYLON_ROOT/lib`, then `/opt/pylon/lib`; then the system's search |
| Vimba X | `VmbC.dll` in `api\bin`, in `VIMBA_X_HOME` or `Program Files\Allied Vision\Vimba X`; then on `PATH` | `libVmbC.so` in `$VIMBA_X_HOME/api/lib`; then the system's search |

`MICROSCOPE_PYLON_LIBRARY` and `MICROSCOPE_VIMBA_X_LIBRARY` name a library in place of the search.

## How to try it

1. **List the devices:**

   ```sh
   cargo run -p microscope-server -- --list-devices
   ```

   On the owner's PC:

   ```text
   Serial ports, a stage's `port`:
     none
   Basler cameras, through pylon:
     0 cameras found, with pylon at C:\Program Files\Basler\pylon\Runtime\x64\PylonC_v10.dll
   Allied Vision cameras, through Vimba X:
     3 cameras found, with Vimba X 1.3.2 at C:\Program Files\Allied Vision\Vimba X\api\bin\VmbC.dll
   ```

2. **Without an SDK,** say by pointing at a library that isn't there, the line says so:

   ```sh
   MICROSCOPE_PYLON_LIBRARY=C:/nowhere/PylonC_v10.dll cargo run -p microscope-server -- --list-devices
   ```

   ```text
   Basler cameras, through pylon:
     pylon isn't installed: it isn't at C:/nowhere/PylonC_v10.dll. Install it from https://www.baslerweb.com/pylon, or set MICROSCOPE_PYLON_LIBRARY to its library's path
   ```

3. **On the Pi,** with pylon or Vimba X for Linux ARM64 installed, the same command shows whether they load there.

## Hardware

- **The owner's Windows PC,** 2026-10-20:
  - pylon 26.09.2 (its C API 10.1.4) loads, starts, and finds 0 cameras;
  - Vimba X 2026-3 (its C API 1.3.2) loads, gives its version, and finds 3 cameras. Which cameras they are, P08 will list.
- **No serial ports** were found there.
- **Linux isn't tried yet:** CI has no SDKs, and this PC has no Linux. The libraries' Linux names and folders are from the vendors' documentation: `libpylonc.so` in `/opt/pylon/lib`, and `libVmbC.so` in Vimba X's `api/lib`. Both vendors ship their SDKs for Linux x86_64 and ARM64. The owner's Pi, or P07 and P08, will try them.

## Design notes and deviations from the plan

- **Unsafe code in one crate** (ADR-0041):
  - **its lints:** the workspace's, without the ban, and with clippy's `undocumented_unsafe_blocks` and `multiple_unsafe_ops_per_block`. Each unsafe operation is in its own block, with a `SAFETY:` comment;
  - **what it declares:** each function's type comes from the vendor's header. They use `__stdcall` on Windows and C elsewhere, which is Rust's `extern "system"`;
  - **what it keeps:** each SDK's struct holds its library while it has the library's functions.
- **The search:**
  - a path is tried only if the file is there;
  - a bare name is left to the system's search;
  - a file that's there but won't load is an error of its own, naming it, rather than "not installed".
- **On Windows,** a library given by its path is loaded with `LOAD_WITH_ALTERED_SEARCH_PATH`, so the libraries it needs are found beside it. Vimba X's folder isn't on `PATH`.
- **One SDK starts at a time:** Vimba X uses pylon's GenTL producers too. When the two started at the same moment on two threads, Vimba X found none of its 3 cameras. One after the other, it found them all, also with pylon initialised. A lock in `microscope-sdk` keeps their starts apart.
- **pylon has no version function,** only its headers' macros, so its line gives the library's path. Vimba X's gives its version.
- **Counts, not cameras:** listing each camera's serial number needs the SDKs' camera structs, which their drivers declare (P07, P08). For now, the count shows the SDK works, and that it sees the cameras.
- **`pylon` stops, after counting, without shutting down its logging,** so it can start again in the same process.
- **`--list-devices` needs no configuration,** and returns before one would be read.
- **The release's licences** accept ISC, `libloading`'s: as permissive as MIT.
- **Size:** 774 changed lines (772 added, 2 removed), within L's 900.

## Tests

- **The loader** (`microscope-sdk`):
  - a missing library: the error lists where it looked, where to get the SDK, and which variable to set;
  - the variable in place of the search;
  - a file that isn't a library: unusable, naming it;
  - a failed call's code, in decimal and hex;
  - where each SDK is looked for, from its variables.
- **The SDKs, where installed:** pylon counts its cameras; Vimba X gives a version of three numbers and counts its cameras. Both ran on the owner's PC; CI skips them, saying so.
- **`list_devices`:** each driver's heading, and either what it found or "isn't installed".
- **`--list-devices`:** exits 0, with each heading.

## Follow-ups and known gaps

- **Linux:** the SDKs on Linux x86_64 and the Pi, when they're installed there.
- **P06:** the SDK camera frame pipeline (ADR-0043).
- **P07, P08:** each camera driver lists its cameras by serial number, and declares the SDK functions it needs.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
