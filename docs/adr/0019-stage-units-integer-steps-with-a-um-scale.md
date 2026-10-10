# ADR-0019: Stage units: integer steps with a µm scale

- Status: Accepted
- Date: 2026-10-09
- Phase: P15

## Context

- OpenFlexure's stage works in whole motor steps per axis. Its API, saved positions and camera–stage mapping (pixels to steps) all use them. Mirroring them keeps workflows and data familiar (D7, D13).
- Some milestone-2 stages are µm-native: their controllers move in micrometres, at a fixed resolution such as 0.1 µm (D10).
- People think in micrometres, and image metadata should say how far apart images are.
- The simulator models the world in µm (ADR-0017), and its stage model (P14) moves in steps with a per-axis µm scale.

## Decision

- **Positions are whole steps per axis:** `Position { x, y, z }`, of `i64`, in every interface, action and saved file. Displacements are positions too.
- **A step is whatever the stage's smallest move is:** a motor step for a stepper stage, the controller's resolution for a µm-native one.
- **Each stage declares its scale per axis,** as µm per step: `AxisScale { x, y, z }`, each an `Option<f64>`.
  - A µm-native stage always knows its scale.
  - A stepper stage knows it once it's configured or calibrated, and until then it's `None`.
- **Conversions:**
  - **To µm:** each axis converts with its own scale. An axis whose scale isn't known has no value in µm.
  - **To steps:** a distance in µm needs every axis's scale, and rounds to the nearest whole step.
- **What people see:** the API shows steps. The web app shows µm wherever the scale is known, and steps otherwise. Capture metadata records the position in steps and the scale.

## Consequences

- No floating-point drift in positions. A move by n steps is exact, and returning to a saved position returns to the same step.
- Camera–stage mapping, backlash and step sizes stay in the stage's own units, so they don't depend on a scale that may be unknown.
- Every stage fits:
  - a Sangaboard in motor steps, with its scale unknown until calibrated;
  - a µm-native stage with 0.1 µm steps, its scale always known;
  - the simulator, with a configured scale.

  `i64` at 0.1 µm per step covers about 900 km, far more than any stage travels.
- Showing µm needs the scale, so the UI and metadata handle "unknown" on each axis.
- An axis scale is linear. A stage with a non-linear axis would correct it inside its driver.

## Alternatives considered

- **µm as floats everywhere.** It's natural for people, but a stepper stage then rounds every move, positions drift, and OpenFlexure parity is lost.
- **Integer nanometres everywhere.** It's exact for µm-native stages, but stepper stages would need a scale before they could move at all, and a step rarely is a whole number of nanometres.
- **A unit carried with each position** (steps or µm). Every calculation would have to check it, for little gain over one unit and a scale.
