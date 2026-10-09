# ADR-0013: Own components on Reka UI primitives

- Status: Accepted
- Date: 2026-10-09
- Phase: P08

## Context

- Every tab is built from the same few controls: buttons, text and number inputs, drop-down lists, collapsible sections, checkboxes, switches and progress indicators. They must look like the reference's (compact 40 px controls, square inputs, one magenta accent: P05) in both themes, through this project's tokens (ADR-0011).
- Some of them carry a lot of behaviour:
  - a drop-down list is a popup that must escape scrolling panes, with focus management, typeahead, keyboard navigation and screen-reader semantics;
  - an accordion needs heading, button and region semantics, and arrow keys between its titles.

  Writing and testing that by hand is a large job.
- Others are native elements that only need styling. The browser already gives a button, a text input or a checkbox its semantics, its keyboard handling, and its form and label behaviour.
- The plan (D14) chose this project's own components, with a headless library for the accessibility-heavy widgets. Dependencies must allow MIT distribution (ADR-0002), and nothing may be copied from OpenFlexure's UIkit-based components.

## Decision

- **Every UI component is this project's own Vue component in `src/ui/`,** styled only with the design tokens. Pages are built from these components.
- **Native elements wherever the browser provides the behaviour:**
  - Button and IconButton are `<button>`s;
  - TextField and NumberField are `<input type="text">`s;
  - Checkbox and Toggle are `<input type="checkbox">`s, Toggle with `role="switch"`.
- **Reka UI for widgets the browser doesn't provide in a form we can style:** Select and Accordion now, and later widgets such as dialogs and menus as their phases need them. Reka UI is headless: it supplies the behaviour and the ARIA attributes, and our components supply the markup classes and every style.
  - The dependency is `reka-ui` 2.10.5, a runtime dependency installed under ADR-0009's two-week rule.
  - It is MIT-licensed. The packages it brings are MIT, Apache-2.0 (`@internationalized/*` and `@swc/helpers`) and 0BSD (`tslib`). The release packaging (P49) gathers their licences.
- **Only `src/ui/` imports Reka UI.** An ESLint rule (`app/ui-components`) enforces it, so pages never depend on Reka UI's API, and an upgrade or a replacement touches only the wrappers.
- **Popups are moved to `<body>`,** so no scrolling pane clips them. They take the theme of the part of the page their trigger is in, since ADR-0011 lets any element set the theme of its contents.
- **Every input has the same contracts:**
  - each works with `v-model`;
  - a `FormField` gives the control inside it its label, help and error, through Vue's provide and inject;
  - Enter emits `submit`.

  NumberField is the exception to "the model follows the input": its model changes only when an edit is committed, with Enter or by leaving the field. A page can then bind it straight to a setting on the microscope without sending a write for every keystroke.

## Consequences

- The hard accessibility behaviour comes from a maintained, widely used library. Our tests cover our own contracts and check that the ARIA wiring survives our wrapping.
- The look is entirely ours, and the tokens drive both themes.
- One more runtime dependency, with 18 packages beneath it. A page pays for it only when it uses a Reka-based component; the production bundle has none yet.
- Reka UI's bugs reach us. Version 2.10.5 leaves an accordion title's `aria-controls` empty, so `AccordionSection` sets it itself. Its Home and End keys can land on a disabled section, where focus stays put.
- Popup styles can't be scoped: popups are rendered outside their component, so their CSS is global, with component-prefixed class names.

## Alternatives considered

- **A styled component library** (PrimeVue, Vuetify, Element Plus, Naive UI). Ready-made, but its look is its own. Matching the reference would mean working against its theme system, and the bundles are larger.
- **Headless UI for Vue.** It has fewer widgets (no accordion beyond a disclosure), and its Vue package moves more slowly than its React one.
- **Everything hand-written.** The drop-down list alone (positioning, focus, typeahead, scrolling and screen-reader semantics) is a large, fiddly component to write and test. The effort is better spent on the microscope.
- **The native `<select>`.** Its open list can't be styled to match the theme in every engine the app may run in. The Tauri app (ADR-0010) uses each platform's own web view.
