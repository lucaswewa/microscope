# P20: Control tab: stage navigation

- Status: In review
- Pull request: (added once opened)
- ADRs: none new
- Spec: [phases.md#p20](../phases.md#p20). P20 was split, at the owner's choice: wheel focus and the navigation preferences are now P20b, which closes Checkpoint A (plan Appendix D).

## Summary

The Control tab moves around the sample the way OpenFlexure's does:

- **A narrow control pane beside the live image.**
- **The Position section:**
  - the live position, observed over SSE;
  - x, y and z fields, which Refresh fills in from the stage;
  - Move, which a Cancel button replaces while the move runs, and Enter in a field moves too;
  - Set Home;
  - Move Home, which asks you to remove your sample first.
- **A d-pad and focus buttons:** a press moves one step, and holding keeps moving until you let go, even with the pointer off the button.
- **Keyboard jog:** the arrow keys and PgUp/PgDn behave the same way, and the `?` dialog lists them.

The image follows every move.

## What was built

| Where | What |
|---|---|
| `web/src/control/jog.ts` | `JogController`: taps, holds, combined directions and repeats, independent of Vue |
| `web/src/control/useJog.ts` | A jog controller for the stage, driven by the keys while the Control tab is open, and the directions |
| `web/src/control/DirectionPad.vue` | The d-pad and focus buttons |
| `web/src/control/PositionSection.vue` | The live position, the fields and the four buttons |
| `web/src/views/ControlPage.vue`, `web/src/router/index.ts` | The Control tab |
| `web/src/api/things/index.ts` | Two fixes found here: an action called without input sends `{}`, and actions with no inputs at all may leave it out |
| `web/tests/unit/control/`, `web/tests/e2e/control.spec.ts` | The tests below; the visual baselines for the Control tab, and the View tab's end-to-end test, updated |

## How to try it

From the repository's root:

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Then open `http://127.0.0.1:5090/#/control`, and try these:

- **Hold a d-pad button**, or an arrow key, to keep moving, and tap one to move a step.
- **PgUp and PgDn** focus.
- **Typed coordinates:** type some, and press Enter or Move. Cancel stops the move where it is.
- **`?`** lists the keys.

## Design notes and deviations from the plan

- **Jogs:**
  - **A tap** moves one step: 200 steps in x and y, and 50 in z, OpenFlexure's defaults; P20b makes them preferences. A released tap isn't stopped, so the step completes.
  - **Holding** for longer than 300 ms makes the jog continuous: every 300 ms, the jog is renewed, aiming five steps ahead, so the stage never catches up and pauses. Releasing stops the stage.

  OpenFlexure's taps move only as far as the stage gets before the key-up's stop. Here a tap is a definite step.
- **Every jog replaces the one in progress,** as the stage's `jog` does (P16, OpenFlexure). So a second tap, made before the first step has finished, cuts it short: ↑ then PgDn a millisecond apart moved y by 1 step, not 200. The end-to-end test taps in turn, as a person does.
- **Directions are as the image shows them:**
  - **x:** → moves to +x, which shows what's to the right.
  - **y:** ↑ moves to −y. In the simulator, the image's y grows downwards, so −y shows what's above. OpenFlexure's ↑ is +y.

  P20b's inversion settles it for other set-ups.
- **Presses combine:** ↑ with → moves diagonally, opposite presses cancel, and the d-pad and a key pressed the same way don't double the speed. Pressing another direction during a held jog re-aims it at once.
- **Safety:**
  - **Losing focus:** a key's auto-repeat is ignored. If the window loses focus or the page is hidden, everything is released: the key-ups would be missed, and the stage would keep jogging.
  - **Pointer capture:** the d-pad's buttons capture the pointer, so releasing anywhere stops the stage.
  - **The keyboard:** Enter or Space on a focused button moves one step.
- **Keys:** they're registered with the P09b shortcut registry, so the `?` dialog lists them under "Stage", and they don't fire while you type in a field. They're active only on the Control tab, as in OpenFlexure.
- **Position:**
  - **The live position** comes from observing `position` and `moving` over SSE (the stage refreshes them every 20 ms while it moves).
  - **The fields** are filled from the stage when the tab opens, after Refresh, and after every move, so the numbers you type aren't overwritten while you type them.
- **Errors** appear as an error toast with the server's details: a move that fails, or a position that can't be read. A cancelled move isn't an error.
- **Layout:** the pane is 180 px wide, as OpenFlexure's is. The fields are stacked, one axis a row, since three side by side would cut off values like −100000.
- **Fixes to P19's facade** (`web/src/api/things/index.ts`), found here:
  - **An action called without input** now sends `{}`. Without a body, the server answered 422, which the first try of Move Home showed.
  - **Actions with no inputs at all** (`set_zero_position`, typed `Record<string, never> | null`) may leave the input out, as intended.
- **The View tab's test** of the stream stopping now leaves for Logging. Control shows the live image too, so the shared stream rightly stays open between View and Control (P18).
- **Visual baselines:** the Control tab's two baselines are new images of the real page. The visual tests turn the stream off and serve a fixed stage position, since other tests move the stage at the same time.
- **Size:** 836 changed lines of hand-written code (about 350 of them tests), within L's budget of 900.

## Tests

- **`jog.spec.ts`** (6 tests):
  - a tap sends one step and no stop;
  - a hold renews the jog every 300 ms, five steps ahead, then stops;
  - presses combine and re-aim;
  - a new press during a hold re-aims at once, without doubling a direction;
  - repeats are ignored, and opposite presses cancel;
  - releasing everything stops once.
- **`ControlPage.spec.ts`** (3 tests, the whole app against a fake server):
  - **Position:** the live position from SSE, and fields filled from the stage. A typed move sends `move_absolute`, and Cancel cancels its invocation.
  - **Home:** Set Home sends `set_zero_position`. Move Home asks first: cancelling sends nothing, and confirming sends `move_to_origin`.
  - **Jogging:**
    - a held ↑ sends a step, two renewals and a stop;
    - a held PgUp stops when the window loses focus;
    - a d-pad tap and an Enter on the button each send one step.
- **`app.spec.ts`:** the Control route shows its page.
- **End to end** (`control.spec.ts`, 3 tests against the real server, one at a time):
  - holding → moves the stage more than 400 steps, and the image changes far more than the sensor's noise;
  - tapping ↑ moves exactly −200 in y, then PgDn −50 in z;
  - typed coordinates move exactly there, and Move Home, confirmed, returns to (0, 0, 0).

**Mutation checks:** I broke each of the following, and a test failed every time:

- the key-up;
- the release on blur;
- the step sizes;
- keyboard clicks;
- the primary-button check;
- the confirmation;
- Cancel;
- observing the position;
- stopping taps;
- clamping combined directions;
- ignoring repeats;
- re-aiming on release, and on a new press.

The last two survived at first, and a new test covers them.

**Checked by hand in Chromium:**

- holding → for a second moved x by about 900 steps;
- a tap of ↑ moved y by exactly −200;
- holding PgUp for a second moved z by 401;
- typed coordinates and Move Home went exactly where they should;
- the confirmation and the `?` dialog look right in both themes.

## Follow-ups and known gaps

- **P20b:** focusing with the mouse wheel, and the navigation preferences (step sizes and inversion, persisted, with a section to set them). It closes Checkpoint A.
- **P21's** autofocus and **P26's** capture join the control pane.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The Control tab's sections, buttons and keys follow OpenFlexure's (plan §6), from reading its source for behaviour only. The jog controller and the layout are this project's own.
