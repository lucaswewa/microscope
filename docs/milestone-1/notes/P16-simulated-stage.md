# P16: Simulated stage Thing

- Status: Done
- Pull request: [#21](https://github.com/lucaswewa/microscope/pull/21)
- ADRs: none new; it follows [ADR-0018](../../adr/0018-hardware-abstraction-through-teta-wot-interfaces.md) and [ADR-0019](../../adr/0019-stage-units-integer-steps-with-a-um-scale.md)
- Spec: [phases.md#p16](../phases.md#p16)

## Summary

The `stage` Thing, `SimulatedStage`, on the P14 stage model. It mirrors OpenFlexure's stage and provides `StageApi`:

- **Moves** that wait until the stage stops, can be cancelled, end with an error at the end of travel, and can compensate for backlash;
- **Jogs** that return at once, where the latest wins;
- **Axis inversion** and the z-direction check;
- **Settings** for backlash, inversion and the simulation itself (speed, backlash, travel limits), which persist;
- **The `arrived` event,** and a `position` that stays current while the stage moves.

The shipped configuration now runs it as `stage`.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/stage.rs` | `SimulatedStage`, its settings types (`PerAxis`, `Travel`, `StageSimulation`) and its `StageApi` implementation |
| `crates/microscope-things/src/hardware/backlash.rs` | `BacklashCompensation`, the shared struct ADR-0018 promised, for every stage driver |
| `crates/microscope-things/src/registry.rs` | Registers `microscope.stage:SimulatedStage` |
| `configs/simulation.json` | Adds the `stage` Thing |
| `crates/microscope-things/tests/stage.rs` | The harness tests below |

The Thing's interface:

| Kind | Names |
|---|---|
| Properties | `position` (observable), `moving`, `axis_names`, `um_per_step`, `calibration_required`, `can_calibrate` |
| Settings | `backlash_steps`, `axis_inverted` (read-only), `z_direction_checked` (read-only), `simulation` |
| Actions | `move_relative`, `move_absolute`, `move_to_origin`, `set_zero_position`, `jog`, `halt`, `invert_axis_direction`, `calibrate_z_direction` |
| Event | `arrived` |

## How to try it

```powershell
cargo test -p microscope-things --test stage
cargo run -p microscope-server -- -c configs/simulation.json --port 5090
```

Then, in another terminal:

```powershell
$stage = 'http://127.0.0.1:5090/api/v1/stage'
Invoke-RestMethod -Method Post "$stage/move_relative" -ContentType application/json -Body '{"x": 2000, "backlash_compensation": true}'
Invoke-RestMethod "$stage/position"          # moves as the stage travels, for 2 s
Invoke-RestMethod -Method Post "$stage/jog" -ContentType application/json -Body '{"y": 50000}'
Invoke-RestMethod -Method Post "$stage/halt" -ContentType application/json -Body '{}'
```

`http://127.0.0.1:5090/docs` lists every affordance.

## Design notes and deviations from the plan

- **The spec's `stop` action is `halt`.** `teta-wot` generates calls to each action by its method's name. Every Thing already has a lifecycle method called `stop`, so an action named `stop` doesn't compile, and the macro can't name an action differently from its method. OpenFlexure has no such action; its UI stops a jog with `jog(stop=true)`, which works here too. `StageApi::stop` is unaffected. The plan's table of Things now says `halt`.
- **`calibrate_z_direction` is an addition.** The spec has `calibration_required` and `can_calibrate` for the z-direction check, but nothing to record its result. This action sets whether z is inverted and marks it checked. The check itself, watching the image as z moves, comes with the camera.
- **Frames.**
  - **Positions are in the program's frame.** An inverted axis counts the other way from the hardware, here the model, as in OpenFlexure.
  - **`backlash_steps` is in the hardware's frame,** because play is mechanical. Inverting an axis doesn't change which way its moves end.
- **Backlash compensation** (`BacklashCompensation`) tracks, per axis, how much of the play is taken up in the preferred direction: 0 to 1, from the moves made, including interrupted ones.
  - **A compensated move** that wouldn't end with the play taken up becomes two: one that stops short by `backlash_steps`, then one in the preferred direction.
  - **When the play is already taken up,** a move in the preferred direction goes straight.

  The default is 200 steps on each axis, more than the simulated 80. A real stage driver will compose the same struct.
- **Waiting for moves.** A move polls the model every 20 ms until it stops, so cancellation is noticed within 20 ms, and it stops the stage where it is. While anything moves, a task started in `on_start` refreshes `position` and `moving` and emits `arrived`, so jogs are tracked too. The task sleeps between moves.
- **Interruptions.** Jogs and `halt` take no global lock, so they can interrupt a move that holds it. The interrupted move then ends normally, returning where the stage stopped. A limit hit by an interrupting jog ends the move with the limit's error.
- **Simulation settings** apply once the stage is at rest, so changing them never cuts a move short.
- **For P17,** `true_position_um()` gives where the sample really is: the model's carriage, including backlash, in µm in the hardware's frame.
- **Defaults:**
  - **Steps:** 0.1 µm in x and y, and 0.05 µm in z.
  - **Speeds:** 1,000 steps a second in x and y, and 500 in z.
  - **Backlash** is off; when on, it's 80 steps.
  - **Limits** are off; when on, ±10 mm in x and y, and ±1 mm in z.
- **Size:** 1,050 changed lines of hand-written code (411 of them tests) against M's budget of 600, well above my estimate of about 700. Asked whether to split, the owner chose one pull request.

## Tests

12 tests:

- **In `tests/stage.rs`** (11), through HTTP unless noted:
  - **Moves wait** and return where the stage stopped: relative, absolute (one axis), set zero, to the origin. Zeroing moves the zero, not the sample.
  - **Properties:** the axis names, the scale, `can_calibrate` and `calibration_required`. Checking z clears it and sets the inversion. The read-only settings can't be written.
  - **Inversion:** an inverted axis's position changes sign, and moves and jogs go the other way in the hardware. An unknown axis is rejected.
  - **Cancelling a move** (DELETE on its invocation) stops the stage short, and it stays put.
  - **Jogs** return at once while the stage moves. The next jog replaces the last, and `stop: true` halts it.
  - **A jog or `halt` interrupts a move** that holds the global lock, and the move returns where the stage stopped.
  - **The end of travel** ends a move with "the x axis reached the end of its travel", at the limit. Moving back in works.
  - **Backlash compensation:** with 80 steps of play, approaching 0 from either side leaves the sample 8 µm apart. With compensation, it lands in the same place.
  - **Settings survive a restart:** backlash steps, inversion, the z check and the simulation.
  - **`arrived`** arrives over SSE when a jog ends, with nothing else polling.
  - **`StageApi`,** called in process as other Things will.
- **In `hardware/backlash.rs`** (1): the plan for each case. Unknown play corrects; taken-up play doesn't for moves the preferred way, but does for reversals; and a partial reversal leaves it partly taken up.

Mutation checks: I broke each of these, and a test failed every time:

- stopping on cancel;
- emitting `arrived`;
- waking the tracking task;
- the correction;
- the "already taken up" test;
- the clamp;
- applying the simulation;
- the limit error;
- setting zero;
- inversion in moves and in jogs.

The tests use real time but take 0.4 s in all. Fifteen runs in a row all passed.

## Follow-ups and known gaps

- **P17** draws the camera's image from `true_position_um()`, and adds the z-direction check's image side.
- **P20** builds the Control tab on `jog`, `halt` and the moves.
- **`teta-wot`:** an action can't be named `start` or `stop`, the lifecycle methods' names. Worth fixing upstream, by calling actions through the type, or by letting an action's name differ from its method's.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The names mirror OpenFlexure's stage (ADR-0007). I read its source for behaviour only: jogs, axis inversion, and backlash compensation that ends moves in one direction. The compensation's design is this project's own.
