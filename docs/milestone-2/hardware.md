# Milestone 2 hardware

The devices milestone 2's drivers are tried on, the SDKs they use, and how the hardware tests find them. Each driver phase's notes say which of these it was run on.

## The owner's devices

| Device | Model | Interface | Firmware | Used from |
|---|---|---|---|---|
| Basler camera | *to record* | *USB 3 or GigE* | *to record* | P07 |
| Allied Vision camera | *to record* | *USB 3 or GigE* | *to record* | P08 |
| Zaber XYZ stage | Three integrated stages, daisy-chained as devices 1, 2 and 3; *models and microstep sizes, to record* | *USB serial* | *to record* | P04 |
| Raspberry Pi 5 | *memory, to record* | — | *Raspberry Pi OS version, to record* | P02, P09, P11 |
| Raspberry Pi camera | *module, to record* | CSI | — | P09 |
| Sangaboard | *version, to record* | *USB serial* | *to record* | P05 |

## The SDKs

The user installs them; the project never redistributes them (plan §4).

| SDK | For | Found on the owner's Windows PC, 2026-10-16 |
|---|---|---|
| Basler pylon | Basler cameras (P07) | pylon 26.09.2, with its C API (`Runtime\x64\PylonC_v10.dll`) and its camera emulator (`PylonCamEmu_v12_TL.dll`) |
| Allied Vision Vimba X | Allied Vision cameras (P08) | Vimba X 2026-3, with its C API (`api\bin\VmbC.dll`) |
| Zaber Motion Library | Not needed: Zaber stages are driven through their ASCII protocol (D3) | 10.2.0 |
| libcamera | The Pi camera (P09) | On the Pi, with Raspberry Pi OS |

pylon's camera emulator can stand in for a Basler camera on a computer with pylon but no camera, which the hardware tests may use.

## Hardware tests

Tests that need a device run only when an environment variable names it, and are skipped otherwise, so CI never needs hardware. P03a sets the convention; the variables are:

| Variable | Names | Phase |
|---|---|---|
| `MICROSCOPE_ZABER_PORT` | The Zaber stage's serial port, such as `COM3` or `/dev/ttyUSB0` | P04 |
| `MICROSCOPE_ZABER_HOMING` | Set to let a test home the Zaber stage, once it's clear | P04 |
| `MICROSCOPE_SANGABOARD_PORT` | The Sangaboard's serial port | P05 |
| `MICROSCOPE_BASLER_SERIAL` | The Basler camera's serial number | P07 |
| `MICROSCOPE_ALLIED_VISION_ID` | The Allied Vision camera's ID | P08 |
| `MICROSCOPE_PI_CAMERA` | Set on a Pi with a camera | P09 |

The names may change as each driver phase writes its tests; this table follows them.

The helper is `microscope_things::drivers::hardware_device("MICROSCOPE_ZABER_PORT")`: it gives the device's name, or `None`, and the test then returns at once, saying it was skipped (ADR-0041).

- **The SDKs' tests** in `microscope-sdk` need no camera: they run wherever the SDK is installed, and are skipped where it isn't, as in CI.
- **The SDKs' libraries** are found where their installers put them. `MICROSCOPE_PYLON_LIBRARY` and `MICROSCOPE_VIMBA_X_LIBRARY` name them elsewhere.
- **`microscope-server --list-devices`** prints the serial ports and the cameras each SDK finds. On the owner's Windows PC on 2026-10-20: no serial ports, no pylon cameras, and 3 cameras through Vimba X.
