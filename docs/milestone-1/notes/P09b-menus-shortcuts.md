# P09b: UI components III: menus, lists and shortcuts

- Status: Done
- Pull request: [#13](https://github.com/lucaswewa/microscope/pull/13)
- ADRs: none
- Spec: [phases.md#p09b](../phases.md#p09b)

## Summary

The rest of the original P09:

- menus of actions, either on any trigger or behind a labelled button;
- page buttons for long lists;
- a drop-down list allowing several choices;
- the app's keyboard-shortcut registry, with a `?` dialog that lists every shortcut.

P07's Shift+↑/↓ tab switching now runs on the registry. The menus, page buttons and multi-select are built on Reka UI (ADR-0013).

## What was built

| Where | What |
|---|---|
| `web/src/ui/AppMenu.vue` | A menu of actions on Reka UI's DropdownMenu, opening from its default slot (a button). Items are `{ label, icon?, disabled?, danger?, run }`, or `'separator'` between groups. The menu opens in its trigger's theme |
| `web/src/ui/ButtonMenu.vue` | AppMenu behind an AppButton with a label and a chevron, such as Download with a choice of formats |
| `web/src/ui/AppPagination.vue` | Page buttons on Reka UI's Pagination: previous, the first and last pages, the pages around the current one with gaps marked "…", and next. `v-model:page`, `total` items and `perPage`. A `<nav>` named "Pages" |
| `web/src/ui/MultiSelect.vue` | A drop-down list allowing several choices, on Reka UI's Select with `multiple`. `v-model` is the chosen values, kept in the options' order, and the trigger shows their labels. It works in a FormField, like AppSelect |
| `web/src/ui/select.css`, `regionTheme.ts` | Shared by AppSelect, MultiSelect and AppMenu: the select's styles, moved out of AppSelect, and the helper that gives a popup its trigger's theme |
| `web/src/app/shortcuts.ts` | The registry: `provideShortcuts()`, `useShortcut(...)` and `keysOf(event)` |
| `web/src/app/ShortcutHelp.vue` | The `?` dialog: every registered shortcut, by group, with its keys |
| `web/src/app/useTabCycling.ts` | Shift+↑/↓ registered as two shortcuts in a "Navigation" group |
| `web/src/App.vue`, `AppShell.vue` | The registry is created in App.vue, and the shell renders the help dialog |
| `web/src/views/dev/ComponentsView.vue` | A "Menus and lists" section in both panels |

## How to try it

```powershell
cd web
npm install
npm run dev     # then open http://localhost:5173/#/dev/components
```

- Under **Menus and lists**, open **Download** with Enter, move with ↓, and choose with Enter; "Last event" shows the choice.
- Open the **⋮** menu and close it with Escape.
- In **Levels**, pick and unpick several levels; the list stays open.
- Page through with the page buttons.
- Press `?` anywhere outside a field for the shortcuts, and Shift+↓ to change tab.

## Design notes and deviations from the plan

- **Shortcuts:**
  - **Keys** are written as `KeyboardEvent.key` after any of `Ctrl+`, `Alt+`, `Shift+` and `Meta+`, in that order: `c`, `?`, `Shift+ArrowDown`, `Ctrl+s`. A printable character already includes Shift, so `?` is `?`, not `Shift+?`. Other modifiers must match exactly, so Ctrl+Shift+↓ doesn't change tab.
  - **A key press is left alone** if something already handled it (the menus' and lists' own arrow keys), if it's typing in a field, or if it happens in a dialog, which owns the keyboard while open. A shortcut can opt in to fields with `inFields`.
  - **Conflicts:** registering keys that are already taken throws, naming the existing shortcut. A conflict is a programming mistake, and throwing makes tests catch it.
  - **Lifetime:** a component's shortcuts are removed when it unmounts, so pages can register their own (P20's stage keys, P21's `a`, P26's `c`).
  - **The registry is created in App.vue, above the shell.** Vue's `inject` reads only what a component's parents provide, so the shell couldn't both create the registry and register Shift+↑/↓ with it.
  - **The `?` dialog** has a Close button, so the focus starts there rather than on the ✕, where a tooltip would flash.
- **Menus are non-modal** (Reka UI's `modal: false`): the page doesn't stop scrolling or go inert while a menu is open. Keyboard use is unchanged: ↑ and ↓ skip disabled actions, Enter or Space runs one, typing jumps by label, and Escape closes the menu and gives the focus back to its trigger.
- **The multi-select:**
  - **The trigger shows the chosen labels itself.** Reka UI's SelectValue showed the placeholder for a few hundred milliseconds after the list closed, while it found the options again. Building the text from the options and the model avoids that.
  - **`aria-multiselectable`:** Reka UI doesn't mark the list as allowing several choices, so MultiSelect adds the attribute.
- **Pagination** names its arrows "Previous page" and "Next page" in place of Reka UI's "Previous Page" and "Next Page". It shows one page each side of the current one, plus the first and last.
- **Checked by hand in Chromium,** in both themes:
  - menus by keyboard and mouse, their theme, separators and the danger style;
  - the multi-select's choices and label;
  - paging;
  - the `?` dialog and Shift+↓;
  - no console warnings.

  The multi-select's flicker and missing `aria-multiselectable` were found there and fixed.
- **Size:** 1,134 changed lines of hand-written code (797 in `src`, 337 in tests) against L's budget of 900, above the 950 I estimated when P09 was split. About 120 of those lines are AppSelect's styles moving into `select.css`, counted once deleted and once added.

## Tests

204 unit tests, and 14 end-to-end and visual tests, all passing locally. New in this phase:

- **Menus** (4):
  - a menu button opens with the keyboard, in its region's theme, listing actions, separators and a disabled action;
  - choosing runs the action and closes the menu;
  - Escape closes without running anything;
  - any trigger works.
- **AppPagination** (3): a "Pages" navigation region marking the current page, with gaps; page, previous and next buttons; previous and next disabled at the ends.
- **MultiSelect** (3): the chosen labels or the placeholder; a multiple-choice list that stays open and keeps the options' order; FormField wiring.
- **Shortcuts** (7):
  - how key presses are named;
  - a shortcut runs for its keys and claims the key press;
  - typing is left alone unless a shortcut opts in;
  - key presses in dialogs, and ones already handled, are left alone;
  - conflicts are refused;
  - a component's shortcuts go when it does, freeing their keys;
  - the `?` dialog lists every shortcut by group.

  P07's tab-switching tests pass unchanged on the registry.

Mutation checks: removing the typing check, the dialog check, the conflict check or `aria-multiselectable` each fails at least one test.

## Follow-ups and known gaps

- Pages register their own shortcuts as they're built: P20's stage keys, P21's `a` and P26's `c`.
- P32 (Gallery) uses the menus and pagination; P43 (Logging) uses pagination and the level filter.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The components and the registry are this project's own, on its own tokens, with Reka UI (MIT) for behaviour.
