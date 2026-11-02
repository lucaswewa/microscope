# ADR-0052: The filter slider's drive

- Status: Accepted
- Date: 2026-11-02
- Phase: OPTIMA E07

## Context

The optics controller drives the filter slider's stepper through a TMC2209 on the [carrier board](../misc/optima_carrier_board.md), with a home switch at one end of its travel. The slider moves rarely, and [precision barely matters](../misc/optima_hardware_design.md#64-filter-slider). The firmware's rule is that decisions go in `optics-core`, where a PC can test them, and that the control loop owns the pins.

## Decision

- **The control loop sends the steps.** `optics-core` works out when each step is due, and the loop wakes for it, as it does for every other deadline. There's no PIO program and no second executor.
- **Each step's time comes from its number:** a ramp of 50 mm/s² up to 10 mm/s, a steady run, then a ramp down. So a late loop catches up, with at most 8 steps at once, and the move keeps its length.
- **4 microsteps a step,** which the driver interpolates to 256. At 400 steps per mm, that's 4 kHz at top speed.
- **The firmware sets the driver up over its UART, at each homing,** rather than at start-up, because the driver can't answer without its 24 V. The driver counts the writes it accepts, so the firmware can tell if they all landed. This takes a few milliseconds of the control loop.
- **The firmware sets the current,** from the driver's internal reference, rather than the module's trimmer.
- **SpreadCycle,** as the motion controller uses (E03).
- **The motor switches off 250 ms after each move,** as the objective changer's does. Its lead screw doesn't back-drive, and this keeps the motor's heat out of the head.

## Consequences

- **Everything but the pulses is tested on a PC,** including the ramp's timing, against a simulated clock and a fake driver.
- **The matrix's frame can delay a step** by about half a millisecond, if a client changes the pattern during a move, and a burst then catches up. That's about half a full step, which a stepper at this speed should take. The bench should check it.
- **The control loop runs at up to 4 kHz during a move.** Nothing here has measured how long each pass takes on the chip.
- **A stall isn't detected until the next homing.** Moves don't read the driver's status. Only the home switch can catch a slider that's badly out of place.

## Alternatives considered

- **A PIO state machine for the steps:** it would give steadier pulses, and a loop that wakes less often. It adds a program that can't be tested on a PC, for a slider that doesn't need the precision. It's the first thing to try if the bench shows rough moves.
- **An interrupt executor at a higher priority:** the same benefit, with some `unsafe`.
- **StealthChop:** it's quieter, but it tunes itself on its first moves, and this motor's power-off between moves could upset that. E03 has the same open question for the changer.
- **Leaving the driver in its standalone mode:** that needs no UART, but the current would then rest on a trimmer, and the 1 kΩ link would hold PDN_UART high.
