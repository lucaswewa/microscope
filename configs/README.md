# Configurations

Each file here is a server configuration for one set of hardware (ADR-0041):

| File | Hardware | Phase |
|---|---|---|
| `simulation.json` | The simulated camera, stage and light | Milestone 1 |

Each driver phase adds its set's, named for its devices: `basler-zaber.json`, `allied-vision-zaber.json`, `pi-sangaboard.json`.

## Naming the devices

- **A stage** by its serial port, `port`: `COM3` on Windows, `/dev/ttyUSB0` on Linux. On Linux, its `/dev/serial/by-id/…` name stays the same when it's plugged in again.
- **A camera** by its serial number, `serial_number`, as its SDK reports it.
