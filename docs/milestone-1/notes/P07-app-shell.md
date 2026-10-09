# P07: App shell and navigation rail

- Status: Done
- Pull request: [#8](https://github.com/lucaswewa/microscope/pull/8)
- ADRs: [ADR-0012](../../adr/0012-icon-set.md)
- Spec: [phases.md#p07](../phases.md#p07)

## Summary

The app now has OpenFlexure's frame, built from this project's own components. A vertical rail holds View, Control, Slide Scan, Sequence and Gallery at the top, and Settings, Logging, About and Power pinned to the bottom. Each item is a Material Symbols icon above its label, and the active item is a solid accent block. Every destination has a route and a placeholder page in the layout it will have. Shift+↑/↓ cycle through the tabs, a notice appears in narrow or portrait windows, and tabs whose Things are missing can be hidden. The rail's proportions are checked against the P05 reference in a test, and visual baselines cover the shell in both themes.

## What was built

| Where | What |
|---|---|
| `src/app/navigation.ts` | The destinations: id (route name and path), label, icon, group, required Things, and the phase that builds each page |
| `src/app/NavRail.vue` | The rail: `<nav aria-label="Main">` with the two groups, router links (`aria-current="page"` on the active one), hover, focus and active styles from the tokens |
| `src/app/AppShell.vue` | The rail, the narrow-window notice and the routed page |
| `src/app/availability.ts` | `thingAvailabilityKey` and `useAvailableDestinations()`. P11 provides real availability; until then everything is available |
| `src/app/useTabCycling.ts` | Shift+↑/↓, wrapping, skipping hidden tabs, and ignored while typing |
| `src/ui/AppIcon.vue` | Draws a Material Symbol from its raw SVG text |
| `src/ui/ControlPane.vue`, `MainView.vue` | The layout primitives: a narrow or wide control pane, and the main view, optionally with the live-image backdrop |
| `src/views/PlaceholderPage.vue`, `src/router/index.ts` | A placeholder per destination, in its future layout. `/` and unknown paths go to View, and Settings takes a section (`/#/settings/camera`) |
| `NOTICE` | Material Symbols' Apache-2.0 attribution |
| `tests/unit/…` | The rail, routes and placeholders, tab cycling, availability (including reacting to changes), and `AppIcon` |
| `tests/e2e/shell.spec.ts` | Clicking, keyboard switching, the narrow-window notice, and the rail's proportions against the reference |
| `tests/visual/shell.spec.ts` (and `-snapshots/`) | Baselines: Control and Settings → Camera, light and dark, at 1280×800 |
| `playwright.config.ts` | Runs `tests/e2e` and `tests/visual`, with a 1% pixel tolerance for screenshots |

## How to try it

```powershell
cd web
npm run dev     # http://localhost:5173/
```

Click through the rail, or press Shift+↓ and Shift+↑. Narrow the window below 900 px for the notice. `npm run test:e2e` runs the end-to-end and visual tests.

## Design notes and deviations from the plan

- **Tolerances.** The spec said "within the tolerances the P05 notes set", but P05 set none. They're set here and asserted in `tests/e2e/shell.spec.ts`:

  | Measure | Reference | This app | Tolerance |
  |---|---|---|---|
  | Rail width, with its border | 85 px | 84 px | ±2 px |
  | Item height | 67 px | 64 px | ±4 px |
  | Icon size | 24 px | 24 px | exact |
  | Label size | 14 px | 13 px | ±1 px |

- **Measured in the browser** at 1280×800: rail 84 px (its 1 px border inside, since `base.css` sizes boxes with their borders), items 64 px, icons 24 px, labels 13 px, the narrow control pane 180 px.
- **Labels are 13 px,** one pixel smaller than the reference's, so "Slide Scan" fits the 84 px rail with room to spare.
- **Icons were chosen for meaning** (ADR-0012), and several differ from OpenFlexure's: Control, Slide Scan, Sequence and Logging.
- **Availability is a function that may read reactive state,** not a ref or getter. The first version used Vue's `toValue`, which treats any function as a getter and called the availability function without its argument. The tests caught it, and a test now checks the rail reacts when availability changes.
- **Tooltips are the native `title`** for now. P09's Tooltip component can replace them.
- **Visual baselines** are Windows/Chromium images (`*-chromium-win32.png`), compared with a 1% pixel tolerance. CI will run them with the e2e job (P12).
- **The old placeholder page is gone.** The version it showed will appear on About (P46).
- **Side-by-side review:** this app's 64 captures, against OpenFlexure's:
  - the rail matches in structure and proportions: icon-over-label items, a solid accent active item, a divider under the workflows, and the bottom group pinned;
  - the 180 px control pane matches OpenFlexure's Control pane (about 187 px with its scrollbar gutter);
  - at 1920×1080 the rail keeps its proportions;
  - the pages themselves are placeholders until their phases.

## Tests

105 unit tests, and 14 end-to-end and visual tests, all passing locally. New in this phase:

- **Unit:**
  - the app opens on View, unknown addresses go to View, every destination has its placeholder, and Settings takes a section;
  - the rail lists the groups in order, marks the current destination (including Settings on any section), hides destinations whose Things are missing, and updates when that changes;
  - Shift+↑/↓ go forward and back, wrap, skip hidden tabs, and do nothing without Shift, with other modifiers, or while typing;
  - `AppIcon` reads the viewBox and paths, uses `currentColor`, is hidden from assistive technology, and takes its size from a token.
- **End to end:** clicking a destination, keyboard switching, the rail's proportions against the reference, and the notice in a narrow window but not a wide one.
- **Visual:** four baselines.

## Follow-ups and known gaps

- P09's shortcut registry takes over Shift+↑/↓ and adds the `?` help.
- P11 provides real Thing availability.
- The e2e CI job, including the visual baselines, comes with P12.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The rail is this project's own component on its own tokens. The icons are Google's Material Symbols under Apache-2.0, attributed in `NOTICE`.
