# P03a: Device actors and serial ports

- Status: In review
- Pull request: [#75](https://github.com/lucaswewa/microscope/pull/75)
- ADRs: [ADR-0041](../../adr/0041-hardware-drivers.md)
- Spec: [phases.md#p03a](../phases.md#p03a)

## Summary

What every driver shares, ready for the stages: the device-actor pattern, a serial port with a scripted fake for protocol tests, the hardware-test convention, and how a configuration names its devices. ADR-0041 records the whole driver framework, including P03b's half: the camera SDKs, loaded at run time, and listing devices.

- **P03 was split in two,** as the owner chose: written whole, it came to about 1,200 lines, twice what I estimated. P03b's half is written, and follows once this is merged.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/drivers/mod.rs` | The pattern, in its documentation. `device_error`, an action's error for a device's, saying what to do. `hardware_device`, the hardware tests' helper |
| `crates/microscope-things/src/drivers/serial.rs` | `Line`, a conversation in lines; `SerialPort`, a real port as a `Line`; `Script`, a fake device as one |
| `crates/microscope-things/tests/drivers.rs` | A driver in a device actor, over a `Script`: its calls, a failure, a bug, `reconnect`, and its close |
| `configs/README.md` | One configuration per hardware set, and how each names its devices |
| `release/about.toml` | The Linux targets too, which P02 missed |

### The pattern

| | |
|---|---|
| The handle | A `teta-wot` `Device<D>` field on the driver's Thing, made from its configuration: `#[device(init = Driver::open(&config.port))]` |
| Calls | On the device's own thread, one at a time: `self.device.try_call(…)` |
| Errors | `device_error`: "stage isn't connected: reconnect it", "stage stopped working (…): reconnect it", "stage didn't answer within 2s", or the driver's own error |
| A bug | A call that panics faults the device; later calls fail at once |
| Reconnecting | Each driver's Thing has a `reconnect` action, `self.device.reset()`: the driver is closed, then made again |
| Starting | The device opens before the Thing starts. One that can't open stops the server starting, with its error |
| Stopping | The device closes the driver when the server stops |

### Naming devices

- **A stage** by its serial port, `port`: `COM3`, or `/dev/ttyUSB0`.
- **A camera** by its serial number, `serial_number`.
- **One configuration per hardware set,** in `configs/`, named for its devices, such as `basler-zaber.json`.

## How to try it

There's no device to try yet: the first drivers are P04's and P05's.

- **The tests:**

  ```sh
  cargo test -p microscope-things drivers
  cargo test -p microscope-things --test drivers
  ```

- **A hardware test's skip:** a test that calls `hardware_device("MICROSCOPE_ZABER_PORT")` passes at once, printing "skipped: set MICROSCOPE_ZABER_PORT to run this test on its device". P04's are the first.

## Hardware

None: the phase has no hardware. Its serial port is tried in P04, on the Zaber stage.

## Design notes and deviations from the plan

- **The split** (plan's change log): P03a has the stages' half, and ADR-0041. P03b has the SDK loader, the one crate allowed unsafe code, and listing devices. P04, P05 and P12 depend on P03a, and P06 on P03b.
- **`teta-wot`'s `Device<D>` does most of it.** It runs a driver on its own thread, opens it before its Thing starts, faults it on a panic, resets it, and closes it with the server. A `#[device(init = …)]` field sees the Thing's configuration, and runs again on each reset. So the pattern is a way of using it, with `device_error` and the `reconnect` action, not a struct of its own.
- **`reconnect` is each driver's own action:** Things can't be generic, and a shared struct can't add actions (ADR-0018).
- **A line protocol, not bytes:** Zaber's and the Sangaboard's protocols are both text lines, so `Line` sends one with its newline, and receives one, without `\r\n` or `\n`, within a timeout. A protocol that needs bytes can add to it.
- **`serial2`, not `serialport`:** `serialport` is MPL-2.0, a copyleft licence, and `serial2` is BSD-2-Clause or Apache-2.0. It's 0.2.38, from 2026-07-31. Its port names are enough to configure a stage.
- **The fake's script** is the lines a driver must send, each with its answers, or a failure, as an unplugged device gives. Anything else sent is an error naming what was expected, and a test keeps a clone to check what was sent.
- **A device unplugged at start** stops the server starting, as `teta-wot` opens devices with their Things. One lost later fails its calls until it's reconnected. Starting without it, and connecting later, would need a change in `teta-wot`.
- **The SDKs' tests** won't need the environment variable: they need no camera, so they run wherever the SDK is installed (P03b).
- **The release's licences** now list the crates of all three platforms. P02's Linux packages listed only those a Windows build uses, so Linux-only ones, such as `libc`, were missing.
- **Size:** 438 changed lines (436 added, 2 removed), within M's 600.

## Tests

- **The fake's scripted conversations:** a conversation in order; a line it doesn't expect, or after its end; no answer, a timeout; a failure, as an unplugged device gives.
- **A real port's lines:** what it receives is split at `\n`, without `\r`, across reads.
- **`device_error`:** each error's message.
- **`hardware_device`:** an unset variable skips; a set one gives its value.
- **The pattern, end to end** (`tests/drivers.rs`): a Thing's driver answers over a `Script`; a failure fails the call, and `reconnect` makes it work again; a panic faults the device, its later calls say to reconnect, and `reconnect` mends it; the server's stop closes the driver. The script ends where it should, so each close happened.

## Follow-ups and known gaps

- **P03b:** the SDK loader (`microscope-sdk`), listing devices, and `--list-devices`.
- **P04:** the first driver, the Zaber stage, and the first hardware tests.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
