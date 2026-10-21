# P04: Zaber stages

- Status: In review, until the owner has tried it on the stage
- Pull request: [#77](https://github.com/lucaswewa/microscope/pull/77)
- ADRs: [ADR-0042](../../adr/0042-zaber-stages-through-the-ascii-protocol.md)
- Spec: [phases.md#p04](../phases.md#p04)

## Summary

A Zaber XYZ stage drives the microscope: `ZaberStage` provides `StageApi` over Zaber's ASCII protocol, on a serial port, through a client of our own. It moves, reads its position while moving, stops, homes, and counts from its own zero. It turns stalls into errors and other warnings into log lines. Its speeds and accelerations are in Settings → Stage.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/drivers/zaber.rs` | The protocol: `Zaber` asks an axis a command and finds its reply by address and id; `Reply`; `ZaberError`; what refusals and warning flags mean |
| `crates/microscope-things/src/zaber.rs` | `ZaberConfig`, `ZaberDriver` (in a `Device`), and the `ZaberStage` Thing |
| `crates/microscope-things/src/registry.rs` | `microscope.stage:ZaberStage` |
| `configs/zaber.json` | A Zaber stage with the simulated camera, which keeps a simulated stage of its own |
| `web/src/server-ui/ServerPanel.vue` | `optional`: a panel only some Things serve shows nothing where it isn't served |
| `web/src/settings/StageSettings.vue` | The stage's own panel, where it serves one |
| `crates/microscope-things/tests/zaber.rs` | The stage against a scripted chain, and on the owner's stage |

### `ZaberStage`

| Affordance | What |
|---|---|
| `position`, `moving` | Where the stage is, in steps from its zero, and whether it moves. Read every 50 ms while it moves |
| `move_relative`, `move_absolute`, `move_to_origin` | Wait until every axis is idle. Cancelled, the stage halts |
| `jog`, `halt` | Start a move and return at once; stop every axis |
| `set_zero_position` | Makes where the stage is (0, 0, 0), in the driver: the devices still count from home |
| `home` | Moves every axis to its home sensor |
| `reconnect` | Closes the port and opens it again |
| `invert_axis_direction`, `calibrate_z_direction`, `z_direction_check` | As the simulated stage's |
| `xy_speed`, `z_speed`, `xy_acceleration`, `z_acceleration` | In steps a second, read from the devices and written to them |
| `settings_ui` | The speeds, the accelerations, and homing |
| `um_per_step` | Each axis's `microstep_um` from the configuration, or unknown |

## How to try it

On a PC with the stage, here the owner's Windows PC. The commands are PowerShell's.

1. **Find its port:**

   ```powershell
   cargo run -p microscope-server -- --list-devices
   ```

   Its port, such as `COM3`, is under "Serial ports".
2. **The hardware tests:** each axis moves a little (1000 microsteps in x and y, 100 in z) and back; a jog in x stops on a halt, and returns:

   ```powershell
   $env:MICROSCOPE_ZABER_PORT = 'COM3'
   cargo test -p microscope-things --test zaber on_the_stage -- --nocapture
   ```

   Then homing, once the stage is clear, since every axis, z too, goes to its end:

   ```powershell
   $env:MICROSCOPE_ZABER_HOMING = '1'
   cargo test -p microscope-things --test zaber on_the_stage_homing
   ```

3. **In the app:**
   - Copy `configs/zaber.json`, and set its `port`.
   - If you know them, set each axis's `microstep_um`, from its datasheet.
   - Run it in a folder of its own, so it doesn't touch your usual settings:

     ```powershell
     cargo build -p microscope-server -p microscope-launcher
     target\debug\microscope-launcher.exe -c my-zaber.json --root $env:TEMP\microscope-zaber
     ```

   - **Control:** the jog buttons and keys move the stage, and the position follows it as it moves. The position fields move it there.
   - **Settings → Stage:** the Zaber panel shows the speeds and accelerations the devices have. Changing one changes them. "Home the stage" homes it.
   - **The log** has a line for each warning, such as an axis not homed (`WR`).
   - The camera's image is the simulator's, and doesn't follow the stage.
4. **Record** the stages' models, their firmware (`/1 get version`) and their microstep sizes in [hardware.md](../hardware.md).

## Hardware

- **Not yet tried on the stage:** this PC has no Zaber stage and no serial port. Everything here ran against the scripted chain.
- **The owner's stage:** three integrated stages, daisy-chained as devices 1, 2 and 3, axis 1 each. The models are to record.

## Design notes and deviations from the plan

- **Our own client, not `zproto`** (ADR-0042): `zproto` needs `serialport`, which is MPL-2.0, and can't run against `Script`.
- **Moves don't hold the device thread:** each command is one exchange, then the stage polls until idle. That's how `position` is read while the stage moves, for autofocus's monitor, and how `halt` gets through at once.
- **Ids find the replies,** since an alert, or another device's reply, can come first on a shared port.
- **The zero is kept in the driver,** as a setting, so the devices' limits still count from home.
- **Faults end the move,** and are cleared, so the next move starts clean. A warning is logged once, when it appears.
- **Microstep sizes from the configuration:** the protocol doesn't give them. Without them, the stage works in steps, as an uncalibrated stage does (ADR-0019).
- **x and y share a speed and an acceleration** in the panel. The devices can still differ, set with Zaber's own tools.
- **Homing is never automatic,** and its hardware test runs only where `MICROSCOPE_ZABER_HOMING` is set.
- **The configuration with a real camera** waits for the camera drivers (P07, P08), as `basler-zaber.json` and `allied-vision-zaber.json`. The simulated camera can't follow a real stage.
- **The warning flags explained** are the common ones. Others show their two letters.
- **The z direction's check** asks the user to jog z and watch, then confirm. There's no gear to see on a Zaber stage, but the direction depends on how it's mounted.
- **`ServerPanel`'s `optional`** shows nothing while it loads, too, so the simulated stage's Settings page, and its visual baseline, are unchanged.
- **Size:** 1,284 changed lines (1,278 added, 6 removed), over L's 900. I'd estimated about 1,000 when the owner chose one pull request. The Thing is 632 lines, with its documentation, and the tests 297.

## Tests

- **The protocol:** replies with and without ids and checksums, and lines that aren't replies; a reply found among an alert and another device's; a refusal, and what it means; no reply; warnings' meanings.
- **The stage, against a script:**
  - a move goes to the axes that move, and ends where the stage stops;
  - positions count from the zero, in each axis's direction;
  - a jog returns at once, and the stage follows it until idle;
  - a stall ends the move with an error, and clears the warning;
  - a refused move says why;
  - a stage that doesn't reply stops the server starting, and says what to check;
  - speeds and accelerations are in steps a second, each way;
  - the settings panel has the speeds and homing.

  Each script ends where it should, so the stage said exactly what it was meant to, and stopped each axis on closing.
- **The app:** an optional panel a Thing doesn't serve shows nothing.
- **On the stage,** with `MICROSCOPE_ZABER_PORT`: moves land where asked, and come back; a halt stops a long move; and, with `MICROSCOPE_ZABER_HOMING`, homing ends at the devices' zero. Not yet run.

## Follow-ups and known gaps

- **The owner's run** of "How to try it", which makes the phase Done, and the stage's models in hardware.md.
- **Listing Zaber devices:** `--list-devices` could ask each port's chain for its devices.
- **P05:** the Sangaboard.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Protocols are written from their documentation, in our own words.
