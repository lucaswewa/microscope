# P08: UI components I: controls

- Status: In review
- Pull request: #NN
- ADRs: [ADR-0013](../../adr/0013-own-components-on-reka-ui-primitives.md)
- Spec: [phases.md#p08](../phases.md#p08)

## Summary

The controls every tab will be built from, as this project's own components on its own tokens. These are buttons, text and number inputs with labels, help and errors, a drop-down list, collapsible sections, checkboxes, switches, progress bars, spinners, cards and section headings. The drop-down list and the collapsible sections are built on Reka UI, which supplies their keyboard and ARIA behaviour (ADR-0013). Everything else is a native element with this project's styles. A development-only gallery at `/#/dev/components` shows each component in both themes.

## What was built

All in `web/src/ui/` unless noted.

| Component | What it is |
|---|---|
| `AppButton.vue` | A `<button type="button">` in four variants and two sizes. `default` is an accent outline (the reference's usual button), `primary` an accent fill, `danger` a red outline and `ghost` text only. Sizes are 40 px and 30 px, with an optional icon before the label |
| `IconButton.vue` | A square AppButton showing only an icon. Its `label` is its accessible name and its tooltip |
| `TextField.vue` | A one-line text input. `v-model` follows every keystroke, and Enter emits `submit` |
| `NumberField.vue` | A number input without spinner buttons, as a `spinbutton`. See [the number field](#the-number-field) |
| `FormField.vue`, `field.ts` | A label above one control, with optional help and error texts below. The control takes the field's id, is described by the texts, and is marked invalid while there's an error |
| `AppSelect.vue` | A drop-down list on Reka UI's Select. `v-model` is the chosen value, a placeholder shows until one is chosen, and options can be disabled. The list opens below the trigger, at least as wide |
| `AppAccordion.vue`, `AccordionSection.vue` | Collapsible sections on Reka UI's Accordion, any number open at once. `v-model` lists the open ones. Each title is a button in an `<h3>`, and each content a region named by its title |
| `AppCheckbox.vue`, `AppToggle.vue` | A native checkbox, and a native checkbox with `role="switch"`, each with its label |
| `ProgressBar.vue` | `value` out of `max`, kept within range, or indeterminate without a value |
| `AppSpinner.vue` | A turning ring in the text colour. With a `label` it's announced as busy, and without one it's decoration |
| `AppCard.vue`, `SectionHeading.vue` | A raised panel; a section title in the reference's style (24 px, light, heading colour) at any outline level, with an `actions` slot at its right |
| `input.css` | The box TextField, NumberField and the Select's trigger share: 40 px (or 30 px) tall, square corners, and hover, invalid and disabled states |
| `web/src/views/dev/ComponentsView.vue` | The gallery, in development builds only: every component in a light and a dark panel, sharing state, with the current values and the last `submit` printed below |
| `web/src/router/index.ts` | The `/dev/components` route, beside `/dev/tokens` |
| `web/eslint.config.js` | `app/ui-components`: only `src/ui/` may import `reka-ui` |
| `web/src/theme/tokens.css`, `tokens.ts` | `--icon-size-xs` (16 px) and `--palette-accent-900`, which is now the dark theme's `--color-accent-subtle`. See [token changes](#token-changes) |
| `web/src/theme/base.css` | Under reduced motion, animations also run only once |
| `web/package.json` | `reka-ui` ^2.10.5 |

## How to try it

```powershell
cd web
npm install
npm run dev     # then open http://localhost:5173/#/dev/components
```

In either panel:

- Tab through the controls; each shows the focus ring.
- In **Exposure**, type text or a number outside 1–100 000. The field turns red, and Enter does nothing. Type a valid number and press Enter: "Last submit" and the state below change. Use ↑ and ↓ to step, and Escape to abandon an edit.
- Set **Step size** above 500 for the error text.
- Open **Resolution** with the keyboard (Enter, Space or ↓). Choose with the arrows and Enter, or type the start of an option. The list opens in its panel's theme.
- Move between the accordion's titles with ↑ and ↓.

## Design notes and deviations from the plan

### The number field

The spec asked for no spinner buttons, Enter to submit, `min`, `max` and `step`, and an invalid state. The details:

- **The model changes only when an edit is committed.** Enter commits and emits `submit`, even if the value hasn't changed, because Enter means "do it". Leaving the field also commits, without `submit`, so a "Move" button clicked straight after typing sees the new value. A page can bind the field directly to a setting on the microscope without a write for each keystroke.
- **Text that isn't acceptable is never committed.** That's text that isn't a decimal number (hexadecimal and `Infinity` aren't), is outside `min`–`max`, or is off the `step` grid counted from `min`. While it's shown, the field is marked invalid and has no `aria-valuenow`. Leaving the field abandons it and shows the model again.
- **↑ and ↓ step** to the next value on the grid, by 1 without a `step`, and stop at `min` and `max`. Like typing, stepping is committed with Enter or by leaving. Escape abandons the edit.
- **While no edit is pending, the field follows the model,** so a stage position updated by the server shows at once. A pending edit isn't overwritten.
- It's an `<input type="text" role="spinbutton" inputmode="decimal">`, not `type="number"`, which would bring spinners and change the value on the mouse wheel. Only `.` is accepted as the decimal separator.

### Built on Reka UI

- **Reka UI 2.10.5,** not 2.11.0: 2.11.0 was released on 5 October, inside ADR-0009's two-week window.
- **The drop-down list is moved to `<body>`,** so the control pane's scrolling doesn't clip it. It takes the theme of the region its trigger is in, which is why the dark panel's list is dark on a light page. Its styles are global, with `app-select__` class names, since scoped styles don't reach a moved element.
- **A workaround for Reka UI's accordion:** 2.10.5 renders each title's `aria-controls` before the content has its id and never updates it, so it stays empty. `AccordionSection` points it at the content once both exist, and a test checks the link for open and closed sections.

### Token changes

- **`--palette-accent-900` (#420B2A, lightness 15%)** is the dark theme's `--color-accent-subtle`, in place of `-800`. The default button's hover puts accent text on the subtle accent, which with `-800` was 4.49:1 in the dark theme, just under 4.5. It is now 6.1:1. The contrast test checks the pair in both themes. The light theme is unchanged, at 5.4:1.
- **`--icon-size-xs`,** 16 px, for the checkbox's mark and the error icon.

### Other details

- **Reduced motion:** `base.css` shortened animations to 0.01 ms but left infinite ones looping, which would make the spinner flicker. Animations now also run only once. The indeterminate bar becomes a full, faint bar, and the spinner a still ring.
- **Names:** single-word components are prefixed `App` (`AppButton`, `AppSelect`, …), as `AppIcon` was. Vue's style guide asks for multi-word names.
- **Checkbox and Toggle** pass other attributes, such as `disabled` and `name`, to their `<input>`, not their `<label>`.
- **The gallery isn't in production builds,** so the end-to-end tests, which run against a production build, don't cover it. Keyboard and ARIA behaviour is unit-tested. The gallery was checked by hand in Chromium in both themes: focus rings, hover, the error and invalid states, the list's theme, choosing by keyboard, and reduced motion.
- **The production bundle** doesn't include Reka UI yet, because no production page uses a component built on it.
- **Supply chain:** the 19 new packages are MIT, Apache-2.0 or 0BSD. `npm audit` lists five high-severity findings (`braces` and `source-map-js`, in the linting and build tools). They are also on `main`, and none is in the new packages. npm didn't run vue-demi's install script. It isn't needed, since the package ships its Vue 3 build by default.
- **Size:** 1,868 changed lines of hand-written code (1,285 in `src`, 563 in tests, 20 in configuration), against L's budget of 900. About 450 of the `src` lines are CSS and 240 are comments or blank lines. The project owner chose to keep it as one pull request.

## Tests

158 unit tests, and 14 end-to-end and visual tests, all passing locally. New in this phase:

- **Fields** (20):
  - TextField's `v-model` follows keystrokes, Enter submits, and attributes reach the input.
  - NumberField:
    - is a spin button with its range;
    - commits on Enter (always submitting) or when it loses focus (not submitting), but not while typing;
    - never commits empty, non-numeric, hexadecimal, out-of-range or off-grid text, and reverts it on leaving;
    - accepts signs, decimals and exponents;
    - steps on and onto the grid within the range;
    - abandons on Escape;
    - follows the model unless an edit is pending.
  - FormField labels its control, describes it with its help and error, marks it invalid, and gives each field its own id.
- **Choices** (16):
  - Checkbox and Toggle are native checkboxes (Toggle with the `switch` role), named by their label; their `v-model` works both ways; clicking the label toggles them; and attributes reach the input.
  - Select:
    - is a closed combobox showing its placeholder, or the chosen option;
    - opens with Enter, Space or ↓, listing the options with the selected and disabled ones marked;
    - updates its model and closes when an option is chosen;
    - opens its list in its region's theme;
    - takes its label, description and error from a FormField.
- **Accordion** (4):
  - each title is a button in a heading that controls its region, which the title names;
  - sections open and close independently, in the model;
  - a disabled section doesn't open;
  - ↑, ↓, Home and End move between titles, skipping disabled ones.
- **Buttons** (4): the type defaults to `button`; variant, size, label and icon; clicks; IconButton's name and tooltip.
- **Feedback** (6): ProgressBar's value, range and clamping, and the indeterminate state; the spinner's two roles; SectionHeading's level and actions; Card.
- **Elsewhere:** the gallery route shows both themes, and the new contrast pair is checked in both themes.

Mutation check: without the `aria-controls` workaround, the accordion's ARIA test fails.

## Follow-ups and known gaps

- **Reka UI:**
  - Remove the `aria-controls` workaround once a Reka UI release fixes it, after checking under ADR-0009's two-week rule.
  - Reka's Home and End go to the first or last section even when it's disabled, and then focus stays put. Keep disabled sections away from the ends, or work around it if a page needs one there.
- P09 adds the overlays: Dialog, confirmations, toasts, Tooltip (which can replace the native `title` tooltips) and menus.
- P12's end-to-end CI job runs against production builds, so it won't cover the gallery. Pages built from these components will be covered there.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The components are this project's own, on its own tokens. The reference contributed only measured proportions (P05). Reka UI (MIT) supplies behaviour, not styles, and the icons are Material Symbols (Apache-2.0, in `NOTICE`).
