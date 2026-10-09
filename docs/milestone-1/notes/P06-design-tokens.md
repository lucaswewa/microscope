# P06: Design tokens and theme switching

- Status: Done
- Pull request: [#7](https://github.com/lucaswewa/microscope/pull/7)
- ADRs: [ADR-0011](../../adr/0011-design-tokens-and-theming.md)
- Spec: [phases.md#p06](../phases.md#p06)

## Summary

The app has its own design tokens now, for colour, type, spacing, radii, controls, layout, layers, elevation and motion. They come in a light and a dark theme, and every colour pairing that matters is tested for WCAG AA contrast. A theme store keeps the user's choice of Light, Dark or Follow System and follows the system as it changes. An inline script applies the saved theme before the first paint, and a test checks the script agrees with the store. A development-only page, `/#/dev/tokens`, shows every token in both themes.

## What was built

| Where | What |
|---|---|
| `web/src/theme/tokens.css` | Palette tokens; semantic tokens; the light and dark themes on `[data-theme]`; reduced motion |
| `web/src/theme/base.css` | Page defaults: background and text from the tokens, the system font, light-weight headings, the focus ring, reduced motion |
| `web/src/theme/tokens.ts` | Every token, grouped, for the tokens page and the tests |
| `web/src/theme/store.ts` | `useThemeStore`: the preference (`light`, `dark` or `system`) in `localStorage` under `microscope.theme` with guarded access, the system's dark mode followed through `matchMedia`, and the resolved theme applied as `data-theme` on `<html>`. Also `resolveTheme`, `readPreference`, `writePreference` and `applyTheme` |
| `web/index.html` | The inline bootstrap script |
| `web/src/main.ts` | Imports the stylesheets and starts the theme store |
| `web/src/router/index.ts`, `src/views/dev/TokensView.vue` | `/#/dev/tokens`, only in development builds |
| `web/src/views/PlaceholderView.vue` | Uses tokens instead of its own font and colour |
| `web/tests/unit/theme/` | Store, bootstrap and token tests, and a fake `matchMedia` |
| `web/tests/e2e/theme.spec.ts` | The theme in the production build |

## How to try it

```powershell
cd web
npm run dev     # then open http://localhost:5173/#/dev/tokens
```

Use the switcher: the page's theme changes at once. Reload, and the choice is kept. Under "Follow system", change Windows' light or dark app mode and the page follows. Each panel shows one theme whatever the page's.

## Design notes and deviations from the plan

- **Values.** ADR-0011's table maps each P05 measurement to its token.
  - **Accent at two strengths.** Fills (the active rail item, primary buttons) use `#CE2283` with white text at 5.0:1. Accent text and borders use `#B41D73` in light (6.2:1) and `#EA7BBA` in dark (6.3:1). One colour couldn't do both in light: `#CE2283` on the light rail is 4.49:1, just short of 4.5.
  - **Control borders reach 3:1,** where the reference's light-theme `#D5D5D5` on white is 1.5:1.
- **Contrast tests,** beyond the spec: 28 foreground/background pairs, checked in each theme. As a check that the tests can fail, I broke two things on purpose and both were caught. Treating an unknown saved value as light failed one bootstrap case. Lightening the muted grey failed four contrast checks.
- **An end-to-end test of the production build,** beyond the spec, covers four things:
  - following the system;
  - a saved choice beating the system;
  - the bootstrap working with the app's scripts blocked, so it really runs first;
  - the light default.
- **`base.css`** wasn't named in the spec. Applying the tokens to the page needs it: background, text, font, headings and the focus ring.
- **Test files read from disk by path,** for two reasons. Vitest doesn't load stylesheets, even with `?raw`. And in the happy-dom environment `URL` is happy-dom's, which Node's `fs` refuses.
- **Review against the references.** I captured the tokens page in both themes and compared its roles with OpenFlexure's captures:
  - **Light:** a white page with a light-grey rail, a solid magenta active fill, and light-weight magenta headings, slightly deeper than OpenFlexure's.
  - **Dark:** charcoal surfaces, the same magenta fill, near-white headings, and pink accent text.

  The rail itself is built in P07, which compares proportions.

## Tests

82 unit tests and 5 end-to-end tests, all passing locally. New in this phase:

- **The theme store** (5):
  - it follows the system when nothing is saved, and as the system changes;
  - a chosen theme applies at once, is saved, and is restored by a new store, as after a reload;
  - an invalid saved value means `system`;
  - storage that throws still gives `system`, and switching still works.
- **The bootstrap** (13): for each saved value (none, `light`, `dark`, `system`, invalid, storage failing) and each system mode, it sets the same theme as the store. One more test checks the script is there.
- **The tokens** (59): the list matches `tokens.css`; both themes define exactly the themed tokens; reduced motion zeroes the durations; and 28 contrast pairs in each theme.
- **End to end** (4, Playwright): the theme in the production build, as above.

## Follow-ups and known gaps

- The Settings → Display theme select is P28. Until then, the tokens page's switcher (in development) is the only way to choose.
- The Tauri app's Content Security Policy will need the bootstrap script's hash, which Tauri can add at build time.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The tokens are this project's own, derived from measured proportions and colour roles, and differ in value.
