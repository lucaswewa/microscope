# ADR-0042: Zaber stages through the ASCII protocol

- Status: Accepted
- Date: 2026-10-21
- Phase: M2 P04

## Context

- **The owner's Zaber XYZ stage** is three integrated X-series stages, daisy-chained on one serial port: devices 1, 2 and 3, each with axis 1.
- **The plan chose Zaber's ASCII protocol in Rust** (D3), over the C++ Zaber Motion Library: nothing to install, and the same on every platform. It left open whether to use the MIT `zproto` crate or a client of our own.
- **`zproto` 0.4.2** (MIT, 2025-03-18) requires `serialport`, which is MPL-2.0, a copyleft licence P03a chose not to bring in. It has its own port type, not P03a's `Line`, so it can't run against P03a's `Script` fake.
- **The protocol,** from Zaber's documentation:
  - **a command** is `/device axis id command`;
  - **a reply** is `@device axis id flag status warning data`, with `OK` or `RJ`, `IDLE` or `BUSY`, and the axis's most important warning flag;
  - **other lines** are alerts (`!`) and info (`#`).
- **The protocol doesn't give a microstep's size** in µm. It's in each stage's datasheet.
- **Speeds and accelerations** are in the devices' own units: a speed's data is microsteps a second times 1.6384, and an acceleration's is microsteps a second squared times 1.6384 / 10 000.

## Decision

- **Our own client,** `drivers::zaber`, over `Line`, written from the protocol's documentation in our own words:
  - **each command carries an id,** 0 to 99, and its reply is the line with the same address and id. Alerts, info and other devices' replies are passed over;
  - **a reply is due within 500 ms.** Without one, the error asks whether the stage is on, and on this port;
  - **a refusal** gives the device's reason, with its meaning: `BADDATA` is "a value it can't take, such as a position beyond its travel".
- **`ZaberStage`** provides `StageApi`, with the affordances the app uses from the simulated stage: `position`, `moving`, the moves, `jog`, `halt`, `set_zero_position`, and the z direction's check. It adds `home` and `reconnect`.
- **Axes** are configured as a device and an axis number each. By default they're devices 1, 2 and 3, axis 1, the owner's chain.
- **Units:** a step is a microstep. Each axis's `microstep_um`, from its datasheet, makes the stage µm-native (ADR-0019). Without it, that axis's scale is unknown, and the app shows steps.
- **Frames in the driver, not the devices:** `set_zero_position` keeps the devices' position as the zero, in a setting, and `axis_inverted` flips an axis. The devices still count from home, so their limits keep their meaning.
- **Moves send one short command per axis,** then poll every 50 ms until every axis is idle. So the device thread is never held by a move, `position` can be read while the stage moves, and `halt` gets through at once. Cancelling a move halts the stage.
- **Warnings:**
  - **faults** (flags starting with `F`, such as a stall) end the move with an error, and are cleared;
  - **other warnings,** such as not being homed (`WR`), go to the log, once each.
- **Homing is an action,** never automatic: it moves every axis to its end, z too, so the user clears the stage first.
- **No backlash compensation:** the stages' lead screws don't need it. The moves take the parameter, for the app's sake, and ignore it.
- **The settings panel** (ADR-0024) has the x and y axes' top speed and acceleration, and z's, in steps a second, read from the devices and written to them, and a button to home. The app shows it in Settings → Stage, where the stage serves it.
- **Hardware tests** run where `MICROSCOPE_ZABER_PORT` names the port. Homing's runs only where `MICROSCOPE_ZABER_HOMING` also says the stage is clear.

## Consequences

- **No new crates,** and the protocol is tested against `Script`, command by command.
- **Microstep sizes come from the configuration.** A wrong one scales distances in µm, but not moves in steps.
- **A stage that isn't homed** still moves, and warns. Its positions count from wherever it was when it was switched on.
- **x and y share a speed and an acceleration** in the panel, as do the moves of a microscope's stage. The devices can differ, set by Zaber's own tools.
- **The simulated camera can't follow a real stage,** since it draws where its simulated stage is. `configs/zaber.json` gives it one of its own, for driving the stage. A real camera's configuration comes with its driver (P07, P08).

## Alternatives considered

- **`zproto`:** a fuller client, but MPL-2.0 code through `serialport`, and no way to test it against our fake.
- **The Zaber Motion Library:** units and device databases built in, but a C++ library to install and load, on every platform.
- **Blocking moves on the device thread** (`move` then waiting there until idle): simpler, but the position couldn't be read during a move, and `halt` would wait behind it.
- **`set pos` on the devices for the zero:** it would move their limits with the zero.
- **Homing when the stage starts:** a z axis homing into a sample would be worse than a stage without a reference.
