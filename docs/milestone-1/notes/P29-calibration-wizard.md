# P29: Calibration wizard

- Status: In review
- Pull request: (added once opened)
- ADRs: none new
- Spec: [phases.md#p29](../phases.md#p29)

## Summary

The calibration wizard guides the user through every calibration the microscope needs.

- **Its structure:** it's a dialog of tasks, each made of steps:
  - a welcome;
  - the stage's z direction;
  - the camera, with steps for the illumination and for finding an empty view;
  - the camera–stage mapping, with a focusing step;
  - an end.
- **When it opens:**
  - **on connecting,** when anything needs calibrating, with only those tasks;
  - **from Settings,** with every task and no welcome.
- **Closing it early** with Escape or × asks first.

## What was built

| Where | What |
|---|---|
| `web/src/calibration/tasks.ts` | The tasks and their steps (our own text), and finding which Things can be calibrated and which need it |
| `web/src/calibration/wizard.ts` | A store: whether the wizard is open, with which tasks; it opens it once for each connection |
| `web/src/calibration/CalibrationWizard.vue` | The dialog: Back, Next, Skip and Finish, and asking before closing |
| `web/src/calibration/GuideStep.vue`, `CalibrateStep.vue` | The two kinds of step: one says what to do, with the live image and the d-pad if they help; the other calibrates a Thing |
| `web/src/app/AppShell.vue`, `views/SettingsPage.vue` | The wizard is in the shell; the Settings button opens it |
| `web/src/ui/AppDialog.vue` | `beforeClose`: Escape, × and a click outside close a dialog only if it answers true |
| `web/src/server-ui/ServerPanel.vue` | Passes its actions' `refresh` on |
| `crates/microscope-things/src/stage.rs` | `z_direction_check`, a panel (ADR-0024): the simulated stage's z check |
| `crates/microscope-things/src/camera.rs` | Full auto-calibrate asks for a refresh |
| `web/tests/e2e/calibrate.setup.ts`, `web/playwright.config.ts` | A setup project that calibrates the test server first |

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

Open `http://127.0.0.1:5090/`:

- **If anything needs calibrating,** the wizard opens. With your settings, that's probably just the stage, since its z direction hasn't been confirmed yet. Go through it with Next, pressing each step's button: Confirm the z direction, Full auto-calibrate and Calibrate. Until a calibration is done, Next says Skip.
- **Press Escape:** it asks before closing.
- **Settings → Calibration wizard** opens it with every task.

To see a first run, point `settings_folder` at an empty folder in a copy of `configs/simulation.json`.

## Design notes and deviations from the plan

- **Tasks are discovered as OpenFlexure's are:**
  - **Which Things:** the wizard knows how to calibrate the stage, the camera and the camera–stage mapping. It offers a task when the server has that Thing, the Thing has `calibration_required`, and it doesn't say `can_calibrate` is false (no `can_calibrate`, as on the camera, counts as yes).
  - **On connecting:** only the tasks whose `calibration_required` is true are offered, after a welcome.
  - **From Settings:** every task is offered, without the welcome.
- **It opens once for each connection.**
  - **Not after reconnecting:** reconnecting to the same server doesn't open it again, so it doesn't come back when the connection drops.
  - **Connecting anew:** connecting to a server, or to the same one again from the Connect screen, does open it.
- **The stage describes its own z check (ADR-0024).**
  - **OpenFlexure's check:** OpenFlexure asks which way the z gear turns. A simulated stage has no gear, so that question can't be answered.
  - **Our check:** the stage serves `z_direction_check`, a panel that explains this and offers "Confirm the z direction". That runs `calibrate_z_direction` with the inversion as it is.
  - **A real stage in milestone 2** can describe the gear check instead, without the wizard changing.
- **The camera's steps are hardware procedures in our words.** They cover centring and focusing the illumination, then finding an empty view. In the simulator, Full auto-calibrate takes the sample out by itself.
- **Skip:** a step that calibrates a Thing reads its `calibration_required` when the step opens, and again after its action succeeds (the action asks for a refresh). Until the Thing is calibrated, Next says Skip, or "Skip and finish" on the last step.
- **Calibrations show their progress in a dialog over the wizard,** with the stream. So Back and Next can't be pressed while one runs, which is what OpenFlexure's wizard does by disabling them.
- **Steps are one list.**
  - **OpenFlexure's structure:** it nests task components, each with its own steps.
  - **Ours:** the wizard flattens the tasks' steps into one list and shows the task in the title, such as "Calibration: Camera".
- **The d-pad's keys don't work in the wizard:** the app's shortcuts don't run inside a dialog, which owns the keyboard. Its buttons do. OpenFlexure's focusing steps mention PgUp and PgDn; ours don't.
- **`AppDialog` closes through `beforeClose`:** Escape, × and a click outside now all go through one path, which asks `beforeClose` when there is one. Other dialogs behave as before.
- **The end-to-end tests:**
  - **Calibrated first:** CI's server starts uncalibrated, so the wizard would open over every test. A setup project calibrates it first, through the API; it confirms the z direction as it is. The mapping test now relies on that setup, rather than calibrating again.
  - **Fresh second servers:** each second server, on 5099, now gets its own fresh settings folder. They had shared the main server's.
  - **The connection test** closes the wizard with Escape, then checks that reconnecting doesn't open it again.
- **Checked in a browser, in both themes:** a first run on a fresh settings folder through every step, including the calibrations and asking on Escape; then a reload, which doesn't open it again; and the Settings button.
- **The confirm button is "Close the wizard",** not "Close", since the confirmation's own × is also called Close.
- **Size:** 856 changed lines of hand-written code, against L's budget of 900. About 370 of them are tests: 221 unit, 125 end-to-end and 23 Rust.

## Tests

- **`tests/unit/calibration/wizard.spec.ts`** (6 tests), the whole app against a fake server:
  - **the tasks:**
    - only for the Things the wizard knows that have `calibration_required`, and not when `can_calibrate` is false;
    - without `can_calibrate`, a Thing counts as calibratable;
    - those whose `calibration_required` is true are the ones needed;
  - **on connecting:**
    - the welcome;
    - the stage's panel, with Skip until Confirm runs, then Next;
    - the camera left out, as it needs nothing;
    - the mapping, with Skip;
    - the end with Finish, and Back;
    - Finish closes it;
  - **not opening:** it doesn't open with nothing to calibrate, nor on reconnecting. Connecting anew opens it;
  - **closing early:** Escape asks, "Keep calibrating" keeps it, and × asks too, then closes;
  - **from Settings:** every task's steps in order, without the welcome.
- **The end-to-end tests:**
  - **`connection.spec.ts`:**
    - "the calibration wizard calibrates a fresh microscope" goes through every step against a fresh server, and checks that nothing needs calibrating afterwards;
    - the connection test closes the wizard, and checks that it stays closed after reconnecting.
  - **`calibrate.setup.ts`** calibrates the main test server first.
- **Rust:**
  - **`the_z_check_confirms_the_direction_as_it_is`:** the panel's button keeps an inverted z inverted, and asks for a refresh;
  - **the camera's calibration panel** asks for a refresh.

**Mutation checks:** I broke each of 21 behaviours, and a test failed every time:

- **the tasks:**
  - offering a Thing without `calibration_required`;
  - ignoring `can_calibrate`;
  - counting a missing `can_calibrate` as no;
  - offering every task as needed;
- **the store:**
  - no welcome;
  - opening with nothing to do;
  - opening again on reconnecting;
  - Settings offering only what's needed;
- **the dialog:**
  - no Skip;
  - Finish not closing;
  - Skip carrying over to the next step;
  - closing without asking;
  - the task missing from the title;
- **the calibrate step:** not checking again after a panel's action;
- **`AppDialog`:**
  - closing whatever `beforeClose` answers;
  - × doing nothing;
- **`ServerPanel`:** not passing `refresh` on;
- **Settings:** the button doing nothing;
- **Rust:**
  - the z check resetting the inversion;
  - the z check without a refresh;
  - Full auto-calibrate without a refresh.

## Follow-ups and known gaps

- **A real stage's z check:** for the hardware's gear check (milestone 2), the stage's panel will need a hold-to-turn button, a new element kind.
- **Exposure in the illumination step:** OpenFlexure's illumination step has buttons for a darker or brighter exposure; ours leaves exposure to the camera's settings.
- **Resetting the camera's calibration:** OpenFlexure offers to reset the camera's calibration before re-aligning the light; our simulated camera has nothing to reset.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its behaviour,** read from its source:
    - which tasks there are, and how they're found;
    - when the wizard opens;
    - Skip;
    - asking on Escape.
  - **Our own:** the structure, the text and the code.
