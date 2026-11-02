# ADR-0051: The optics controller's sequences

- Status: Accepted
- Date: 2026-11-02
- Phase: OPTIMA E07

## Context

The [hardware design](../misc/optima_hardware_design.md#54-timing-with-a-rolling-shutter) wants the optics controller to step through preloaded light states, one per frame, with no round trip to the PC. The cameras have rolling shutters, so the light must stay constant from the first row's start to the last row's end. ToupTek publishes no pinout, so nobody yet knows exactly what the exposure signal covers. [ADR-0050](0050-the-optics-serial-protocol.md) left open whether a sequence should survive losing USB.

## Decision

[The protocol](../misc/optima_optics_protocol.md#sequences) is the specification. Its main choices:

- **A step is a whole light state:** the four epi LEDs and the matrix's three masks, up to 64 steps, which is enough for one LED at a time across the matrix.
- **An exposure's end, plus a delay the client sets, moves to the next step.** The delay covers whatever readout is left, without the firmware having to know the camera's timing.
- **The firmware can trigger the camera too,** 2 ms after each step's light comes on. A rolling shutter running freely overlaps its frames, so a clean sequence usually needs the next frame to wait for the light.
- **It starts only between frames,** and owns the light path while it runs: every command that would change the light gets `err busy`.
- **`led all off` and `seq stop` always work,** and however a sequence ends, its light goes out.
- **Epi steps obey the door.** Opening it stops such a sequence, since the channels it needs are off. A sequence that uses only the matrix doesn't depend on the door, which doesn't interlock the matrix.
- **Losing USB stops it.** Nothing could stop it or read its events after that.

## Consequences

- **Frames need no PC,** and a client knows which light lit a frame from `! seq step`.
- **The client must know its camera's readout time,** to set the delay. The bench should measure this.
- **A stalled camera stalls the sequence.** There's no timeout, because an exposure can be long. A client can stop it.
- **Only light steps.** The slider and the camera port are refused while one runs, since they take far longer than the gap between frames.

## Alternatives considered

- **Stepping on an exposure's start:** the light would change as the frame begins, which is what the design forbids.
- **Timed steps with no camera signal:** they'd drift from the camera, and a dropped frame would shift every channel.
- **Keeping a sequence running across a USB loss:** a long acquisition could survive a cable fault, but its light would stay on with no one able to stop it.
- **Separate commands for matrix steps and LED steps:** that's more commands, and some steps need both.
