# P14: Simulated stage motion model

- Status: Done
- Pull request: [#18](https://github.com/lucaswewa/microscope/pull/18)
- ADRs: none
- Spec: [phases.md#p14](../phases.md#p14)

## Summary

A model of an XYZ stage in `microscope-sim`, with timed moves at a set speed and imperfections that can be switched on and off: backlash, a dead band per axis; and travel limits, which stop the stage and report an error. It keeps the true position (where the sample really is) apart from the reported one (what the stage says, after a zero offset). It never reads a clock: every call is given the time, so tests control time exactly. P16 wraps it in the stage Thing.

## What was built

| Where | What |
|---|---|
| `crates/microscope-sim/src/stage.rs` | `StageModel`, `StageConfig` (speed, backlash and limits per axis) and `MoveError` |
| `crates/microscope-sim/src/clock.rs` | The `Clock` trait, and `ManualClock` for tests and examples |
| `crates/microscope-sim/examples/trajectory.rs` | A move out, back and into a limit, printed over time |
| `crates/microscope-sim/tests/stage.rs` | The tests below |

The model's operations:

- **Moving:** `move_to(now, target)` and `move_by(now, delta)`.
- **Stopping:** `stop(now)`.
- **State:** `is_moving`, `arrival` and `outcome`.
- **Positions:** `reported(now)` and `true_position(now)`.
- **Zero:** `set_zero(now)`.
- **Configuration:** `config` and `set_config(now, config)`.

## How to try it

```powershell
cargo run -p microscope-sim --example trajectory
```

It prints the reported and true x position every 250 ms as the stage moves out to 2,000 steps, back to 0, then towards 5,000, where it stops at its limit of 3,000 and prints why. With 40 steps of backlash:

- going out, the carriage trails the motors by 20 steps;
- going back, it waits while the motors take up the 40 steps of play.

## Design notes and deviations from the plan

- **Three positions per axis, in steps:**
  - **The motors'** position moves in a straight line and stops on whole steps.
  - **The carriage's,** the true position, is kept within half the backlash of the motors. A motor moving one way pushes the carriage along, trailing by half the play; when it turns, the carriage stays put until the motor has crossed the dead band. Within a move each axis moves one way only, so the carriage's position at any moment is just the motors' position clamped into that band, which is exact.
  - **The reported position** is the motors' position, rounded, minus the zero offset.
- **Moves:**
  - **The path:** the axes move together in a straight line. The move takes as long as the slowest axis needs, so the others travel more slowly.
  - **Speed:** the defaults are 1,000 steps a second in x and y and 500 in z. An axis whose speed isn't positive moves at once.
  - **Latest wins:** a new `move_to` or `move_by` interrupts the move in progress, starting from wherever the motors are at that moment. `move_by` is relative to there, not to the old target. This is what makes held-down jog keys work: the newest command takes over.
  - **Stopping** rounds to whole steps, as a stepper motor stops.
- **Limits:**
  - **Where they apply:** to the motors' position, as each axis's lowest and highest step.
  - **Hitting one:** a move whose target lies beyond a limit runs until the first axis reaches it. The whole move then stops there, and `outcome` reports `LimitReached` with the axis, as a controller does when a limit switch trips.
  - **Already outside:** a stage outside its travel, after the limits change, stops at once on the next move that goes further out.
- **Switching at run time:** `set_config` stops any move in progress first, so a move never mixes two configurations. Turning backlash off brings the carriage to the motors.
- **Time:** the model takes `now` as a `Duration` since some fixed start. `Clock` and `ManualClock` are for whatever drives it. The real clock belongs to the Thing (P16), since `microscope-sim` mustn't read the system clock (ADR-0005).
- **The property tests** draw random moves from the crate's own seeded sequence rather than a property-testing crate: 50 seeds of 40 moves, some interrupted part-way. That avoids a dependency, and every run tests the same cases.
- **Size:** 617 changed lines of hand-written code (237 of them tests, 48 the example) against M's budget of 600.

## Tests

9 tests (`tests/stage.rs`):

- **Timing,** against `ManualClock`: a move takes its distance over its speed, the axes arrive together on a straight line, and a move limited by z keeps x on the line.
- **Latest wins:**
  - a new move starts from where the stage is;
  - `move_by` is relative to there.
- **Stopping:** stopping lands on the nearest whole step, and the stage is no longer moving.
- **The zero offset:** it changes the reported position, not the true one.
- **Backlash properties,** over random moves with random backlash:
  - the carriage is never more than half the backlash from the motors;
  - approaching the same target from below and from above leaves true positions exactly the backlash apart;
  - turning back by less than the play doesn't move the carriage;
  - without backlash, the carriage is where the motors are.
- **Limit properties,** over random moves:
  - the stage never leaves its travel;
  - a move ends with an error exactly when its target is outside the travel;
  - after an error the axis is at a limit.
- **Switching at run time:** it stops a move in progress, turning backlash off brings the carriage to the motors, and removing the limits lets later moves go further.

Mutation checks: each of these, removed, fails tests:

- the backlash;
- the limits;
- interrupting from the current position;
- moving the axes together.

The 11 simulator tests from P13 still pass.

## Follow-ups and known gaps

- P15 defines the stage interface and its units (ADR-0019), and P16 wraps the model in the simulated stage Thing, with a real clock and backlash compensation.
- Moves have no acceleration: each axis goes straight to its speed. That's enough for the workflows; it can be added if timing needs to be more realistic.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The model is written from the plan's description of a stage.
