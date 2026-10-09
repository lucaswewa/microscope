# P09: UI components II: overlays and feedback

- Status: In review
- Pull request: [#10](https://github.com/lucaswewa/microscope/pull/10)
- ADRs: none
- Spec: [phases.md#p09](../phases.md#p09)

## Summary

The components that appear over a page:

- a modal dialog, with Escape and outside-click policies;
- `useConfirm()`, a promise-based confirmation that can hold rich content;
- toasts for success, information and errors, with folded details;
- tooltips;
- ErrorDetails, which shows any error a `teta-wot` server returns.

The dialog, the toasts and the tooltips are built on Reka UI (ADR-0013). The phase was split in two before it started, at the project owner's choice. Menus, pagination, multiple selection and keyboard shortcuts moved to the new P09b.

## What was built

All in `web/src/ui/` unless noted.

| Where | What |
|---|---|
| `AppDialog.vue` | A modal dialog on Reka UI's Dialog. `v-model:open`, a `title` that names it, an optional `description`, and three sizes. The body scrolls, and a `footer` slot holds the buttons. `dismissible` (default true) allows Escape, the close button and clicks outside. `closeOnOutsideClick` turns off clicks outside alone, and `alert` makes it an `alertdialog` |
| `overlays.ts` | `useConfirm()` and `useToast()`, with their option types. Each throws a clear error when there's no OverlayProvider above it |
| `OverlayProvider.vue` | Provides both, and shows them. Confirmations queue, one dialog at a time. Toasts stack in the bottom-right corner on Reka UI's Toast |
| `AppTooltip.vue` | A tooltip on Reka UI's Tooltip, shown after 500 ms of hover or at once on keyboard focus. It brings its own provider, so it works anywhere |
| `errors.ts` | `describeError()`: a heading, a message and a list of invalid inputs, from any error the app meets |
| `ErrorDetails.vue` | Shows what `describeError()` reads |
| `IconButton.vue` | Shows its label in an AppTooltip instead of the native `title`, and passes other attributes and listeners to its button |
| `web/src/App.vue` | One OverlayProvider around the shell |
| `web/src/views/dev/ComponentsView.vue` | The gallery: error details in both panels, and an Overlays section with a dialog, a busy dialog, two confirmations, three toasts and a tooltip |
| `docs/milestone-1/phases.md`, `implementation-plan.md` | The split. P09 keeps dialogs, confirmations, toasts, tooltips and error details. P09b gets menus, pagination, MultiSelect and keyboard shortcuts. P20, P24, P32 and P43 now depend on P09b. Both phases are sized L, and the change is in Appendix D |

## How to try it

```powershell
cd web
npm install
npm run dev     # then open http://localhost:5173/#/dev/components
```

Under **Overlays**:

- **Dialog** opens on its field. Tab stays inside it. Escape, the ✕ or a click outside closes it, and the focus goes back to the button.
- **Busy dialog** ignores Escape and clicks outside, and has no ✕.
- The two **Confirm** buttons start on Cancel and ignore clicks outside. "Last event" shows the answer.
- The **toasts:** success and info go after 5 s, and the error stays until dismissed. Open its **Details**.
- Tab onto any icon button to see its tooltip.

The overlays open in the page's theme, not the panel's.

## Design notes and deviations from the plan

- **The split.** Before starting, I estimated the original P09 at about 1,900 lines with tests, three times its M budget. The project owner chose two pull requests over one or three.
- **The confirmation's API.** `await confirm({ title, message?, content?, confirmLabel?, cancelLabel?, danger? })` resolves to `true` or `false`, and never rejects:
  - confirming gives `true`;
  - Cancel, the ✕ and Escape all give `false`;
  - a click outside does nothing, as is usual for alert dialogs.

  `content` is a render function, so a confirmation can list what it will delete. Questions asked while one is open wait their turn.
- **Where a dialog's focus starts.** Reka UI focuses the first focusable element, which is the dialog's ✕. AppDialog starts on the first field or button in its body, then its footer. That's the field in a form, and Cancel in a confirmation, the least destructive choice. Focus goes back to whatever had it when the dialog closes. Reka UI does that itself, and a test checks it.
- **A workaround for Reka UI's dialog:** without a description, Reka UI still points `aria-describedby` at a description element that doesn't exist, and warns in the console. AppDialog removes the attribute then.
- **Toasts:**
  - **Announcing:** errors are announced at once and the rest politely (Reka UI's foreground and background types).
  - **Timing:** success and info go after 5 s and errors stay; hovering or focusing a toast pauses its timer. F8 moves the focus to the toasts (Reka UI's default).
  - **Details:** they're folded in a native `<details>` and shown with ErrorDetails, so a server error can go straight into a toast.
- **Tooltips:**
  - **Each AppTooltip brings its own provider.** Without one above it, Reka UI's tooltip throws, which would make IconButton unusable without an app-level provider in tests and elsewhere. What's lost is Reka UI's shorter delay when moving straight from one tooltip to the next.
  - **Only real keyboard focus shows them.** Focus moved by code after a click doesn't count, so a dialog opened with the mouse doesn't flash a tooltip.
  - **IconButton now uses AppTooltip** in place of `title`. The rail keeps its native `title`, since its labels are always visible.
- **ErrorDetails** reads the three bodies a `teta-wot` v0.1.0 server sends in its default `tetathing` profile, found in `crates/teta-wot-http/src/render.rs` and `problem.rs`:
  - `{"detail": "…"}`;
  - a 422's `{"detail": [{"type", "loc", "msg", "input"}]}`;
  - a problem, `{"detail", "type", "status", "title", "instance"}`, such as the global lock being busy.

  An issue's location drops its first step (`body`, `path` or `query`) and writes indices as `rows[2].name`. It also reads text and `Error`s, and shows any other shape as JSON. The `wot` profile's `invalid-params` isn't read: this app's server uses the default profile.
- **The production bundle grew.** The app-wide OverlayProvider brings Reka UI's dialog, toast and positioning code into every page. The JavaScript is now 244 KB (87 KB gzipped), up from 103 KB (41 KB). For an app served on the local network or packaged with Tauri that's acceptable. It can be split later if load time matters.
- **The gallery's overlays open in the page's theme.** They're page-wide, so there's no panel theme to follow.
- **Checked by hand in Chromium,** in both themes:
  - focus moving in, staying in under Tab, and coming back;
  - the dismissal policies, the alert dialog, and confirming and cancelling;
  - the toasts and their details, and the tooltip on keyboard focus;
  - no console warnings.

  The first pass found two problems, both fixed: a field's focus ring was cut off by the dialog's scrolling body, and a "Close" tooltip flashed whenever a dialog opened.
- **Size:** 1,104 changed lines of hand-written code (707 in `src`, 397 in tests) against L's budget of 900. That's more than the 950 I estimated when the phase was split.

## Tests

187 unit tests, and 14 end-to-end and visual tests, all passing locally. New in this phase:

- **AppDialog** (9):
  - named by its title, it starts on its first field;
  - it's described only when it has a description;
  - Tab stays inside;
  - Escape, the close button and a click outside close it and give the focus back;
  - clicks outside can be ignored on their own;
  - when not dismissible it ignores Escape and clicks outside and has no close button;
  - `alert` makes it an alert dialog.
- **useConfirm** (8):
  - it resolves `true` when confirmed, and `false` when cancelled, closed or dismissed with Escape;
  - it ignores clicks outside;
  - it shows its message, rich content and labels, with a destructive style, starting on its cancel button;
  - questions are asked one at a time, in order;
  - it needs an OverlayProvider.
- **useToast** (3): each kind in the notifications region; details folded and read as an error; success and info go after 5 s (or their own duration), errors stay, and dismissing closes one.
- **Errors** (8): `describeError` reads a `{detail}`, a validation list (with locations), a problem, text, an `Error`, and anything else as JSON; ErrorDetails shows a validation list, a problem and an unknown shape.
- **IconButton** (2, one replaced): the label names it and shows as a tooltip on keyboard focus, described by `aria-describedby`; attributes and listeners reach the button.

Mutation checks:

- Without the start-focus handling, four tests fail.
- Without the `aria-describedby` fix, the description test fails.
- A focus-return handler I'd first added for dialogs opened from code turned out to be unnecessary: with it removed, the focus still came back, in Chromium and in the tests. I deleted it.

## Follow-ups and known gaps

- P09b: Menu and ButtonMenu, Pagination, MultiSelect, the shortcut registry and the `?` help dialog, which takes over P07's Shift+↑/↓.
- P10's `ApiError` will carry these error bodies, and P11 will show connection errors as toasts with their details.
- Pages adopt AppTooltip as they're built. The rail keeps its native `title`.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The components are this project's own, on its own tokens, with Reka UI (MIT) for behaviour. The error shapes come from `teta-wot`'s source (MIT), the project's own dependency.
