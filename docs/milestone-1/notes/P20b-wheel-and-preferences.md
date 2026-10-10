# P20b: Control tab: wheel focus and navigation preferences

- Status: In review
- Pull request: (added once opened)
- ADRs: none new
- Spec: [phases.md#p20b](../phases.md#p20b), split from P20 (plan Appendix D)

## Summary

The Control tab now focuses with the mouse wheel, and navigation can be set up the way you like it:

- **The wheel over the live image focuses:** turning it away from you focuses up, a z step a notch.
- **Navigation preferences**, kept in this browser:
  - a step size for each axis, used by the d-pad, the keys and the wheel;
  - inversion of each axis, for set-ups whose camera sees the stage turned or mirrored.

  A collapsed "Navigation" section in the control pane sets them.

## Checkpoint A, "Live microscope"

The checkpoint is reached: the app shows the live simulated image, and the buttons, keys, wheel and typed coordinates all move the stage, with the image following. Each part is tested end to end against the real server:

| Part | Shown by |
|---|---|
| The live simulated image | `view.spec.ts`: frames arrive (P18b) |
| Buttons | `control.spec.ts`: holding the d-pad moves the stage, and the image changes (P20) |
| Keys | `control.spec.ts`: a tap of ↑ moves −200 in y, and PgDn −50 in z (P20) |
| The wheel | `control.spec.ts`: six notches focus up 300 steps, and the image changes as it blurs (here) |
| Typed coordinates | `control.spec.ts`: Move goes exactly there, and Move Home brings it back (P20) |

## What was built

| Where | What |
|---|---|
| `web/src/control/navigation.ts` | `useNavigationPreferences`: steps and inversion, saved in `localStorage` and checked when read |
| `web/src/control/wheel.ts` | `wheelFocus`, and `notches`, which reads a wheel event in pixels, lines or pages |
| `web/src/control/NavigationSection.vue` | The section that sets the preferences, ready for the Settings tab (P28) to reuse |
| `web/src/control/useJog.ts`, `web/src/views/ControlPage.vue` | Jogs use the preferences; the wheel and the section join the Control tab |
| `web/tests/unit/control/`, `web/tests/e2e/control.spec.ts` | The tests below, and the Control tab's visual baselines updated for the new section |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Then open `http://127.0.0.1:5090/#/control`. Turn the wheel over the image to focus. Open "Navigation" to change the steps, or to invert an axis, and reload: the choices stay.

## Design notes and deviations from the plan

- **The wheel's direction:** turning it away from you focuses up (+z). OpenFlexure does the opposite. "Invert z" reverses the wheel, the focus buttons and PgUp/PgDn together.
- **Adding up turns:** every jog replaces the one in progress, so the wheel's turns are added up and sent as one jog every 100 ms. Turning fast moves at the stage's speed, and a turn and its turning back send nothing.
- **Notches:** a notch is 100 pixels of delta, 3 lines, or a page, as browsers report them; touchpads give fractions, which add up.
- **Inversion is applied to the step:** a press towards + moves the stage −, for every source alike.
- **Saving:** the preferences are saved as JSON under `microscope.navigation` in `localStorage`.
  - **Checked when read:** steps must be positive whole numbers, and inversions true or false. Anything else, including unreadable JSON, falls back to the defaults: 200, 200 and 50 steps, nothing inverted.
- **Where the section lives:** in the Control pane, collapsed under "Navigation". OpenFlexure keeps these under Settings, and P28's Settings tab can show the same component there.
- **Size:** 272 changed lines of hand-written code (about 95 of them tests), within S's budget of 300.

## Tests

- **`navigation.spec.ts`** (4 tests):
  - the defaults;
  - saving and reading back steps and inversion;
  - rejecting saved values that make no sense;
  - counting notches in each unit;
  - the wheel: turns away focus up, are added up and sent once per 100 ms, and turning back sends nothing. The page doesn't scroll.
- **`ControlPage.spec.ts`** (1 new): with saved steps (y 30, z 10) and y inverted, ↑ jogs +30 in y, and two notches of the wheel focus up 20. The Navigation section's checkbox saves its choice.
- **End to end** (`control.spec.ts`, 1 new): six notches of the wheel over the image focus up exactly 300 steps, and the image changes as it blurs.

**Mutation checks:** I broke each of the following, and a test failed every time:

- the wheel's direction;
- adding up turns;
- `preventDefault`;
- reading lines;
- inversion;
- checking saved steps;
- saving.

## Follow-ups and known gaps

- **P28's** Settings tab shows `NavigationSection` under "Stage control preferences", alongside "Disable stream".
- **Wheel speed:** fast wheel turns are limited by the stage's speed, as with OpenFlexure's jogs. P21's autofocus is the precise way to focus.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The preferences mirror OpenFlexure's navigation settings (step sizes and inversion), from reading its source for behaviour only.
