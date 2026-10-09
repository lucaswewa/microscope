# ADR-0011: Design tokens and theming

- Status: Accepted
- Date: 2026-10-09
- Phase: P06

## Context

- The [goals](../milestone-1/goals.md) ask for:
  - compact controls and a prominent live image;
  - a magenta accent, light neutral surfaces and dark charcoal surfaces, all through original components and theme tokens;
  - Dark, Light and Follow System themes that remember the user's choice.
- P05 measured OpenFlexure's interface ([P05 notes](../milestone-1/notes/P05-reference-screenshots.md#measurements)). Its reference values are an 85 px rail, 40 px controls (30 px small), 14 px text, weight-300 headings, and a `#C32280` accent on white or `#222`.
- The app runs in browsers and, later, in Tauri's WebView2 (Chromium), with no network access guaranteed (ADR-0010). OpenFlexure's ProximaNova font is commercial.
- The interface should meet WCAG 2.2 AA: 4.5:1 contrast for text, and 3:1 for focus indicators and control boundaries.

## Decision

**Two layers of CSS custom properties** in `web/src/theme/tokens.css`:

- **Palette tokens** (`--palette-*`) are raw colours, used only to define semantic tokens:
  - an accent scale at hue 326° and 72% saturation, from 50 to 800;
  - neutral greys;
  - a dark and a light shade each of red, amber, green and blue.
- **Semantic tokens** are what components use:
  - `--color-*`: surfaces, text, lines, controls, accent, rail, focus and status;
  - `--font-*` and `--line-height-*`;
  - `--space-*` (a 4 px grid) and `--radius-*`;
  - `--control-*`, `--icon-*` and `--border-width`;
  - layout: `--rail-*` and `--pane-*`;
  - `--z-*`, `--shadow-*`, `--duration-*` and `--easing-*`.

`tokens.ts` lists them all for the tokens page, and a unit test keeps that list and the stylesheet in step.

**Themes:**

- Only colours and shadows differ between themes. They're defined for `[data-theme='light']` (and `:root`) and for `[data-theme='dark']`, each with its `color-scheme`, so native controls and scrollbars match.
- The theme is applied as `data-theme` on `<html>`. Any element can set the attribute to show its contents in one theme.

**Values,** derived from the P05 measurements but this project's own:

| Role | Reference (OpenFlexure) | This app |
|---|---|---|
| Rail | 85 px wide; 67 px items | `--rail-width` 84 px; `--rail-item-height` 64 px; 24 px icons |
| Control panes | about 180 px (Control) and about 400 px (Slide Scan) | `--pane-width-narrow` 180 px; `--pane-width-wide` 400 px |
| Controls | 40 px (small 30 px); 3 px button radius; square inputs | `--control-height-md` 40 px; `--control-height-sm` 30 px; `--radius-sm` 3 px for buttons and inputs |
| Type | 14 px / 21 px; headings 30 and 24 px at weight 300 | `--font-size-md` 14 px, line height 1.5; headings 30, 24 and 20 px at weight 300 |
| Accent fills | `#C32280` with white text | `--color-accent` `#CE2283` with white text (5.0:1) |
| Accent text and borders | `#C32280` light; `#E151A5` dark | `--color-accent-text` `#B41D73` light (6.2:1); `#EA7BBA` dark (6.3:1) |
| Light surfaces | white page; rail a 10% grey wash | `#FFFFFF` page; `#F2F2F2` rail; `#FAFAFA` panels |
| Dark surfaces | `#222` page | `#1F1F1F` page; `#2B2B2B` rail; `#232323` panels; `#2A2A2A` raised |
| Control borders | `#D5D5D5` on white (1.5:1) | `#8C8C8C` light (3.4:1); `#7A7A7A` dark (3.3:1 on the control) |

**Accessibility, enforced by `tokens.spec.ts`:**

- Every text pairing reaches 4.5:1 in both themes: text, muted text and headings on every surface; accent text on every surface; text on the accent; status colours.
- The focus colour and control borders reach 3:1.
- Where the reference falls short (control borders, and light-theme accent text on the rail), the tokens depart from it.

**Fonts:** system fonts only:

- Segoe UI Variable Text, or Segoe UI, for text;
- Segoe UI Variable Display for headings;
- Cascadia Mono or Consolas for code;
- standard fallbacks on other platforms.

Nothing is downloaded.

**Motion:** the duration tokens are 0 under `prefers-reduced-motion`, and `base.css` cuts any remaining animation short.

**The preference:**

- It is `light`, `dark` or `system`, kept in `localStorage` under `microscope.theme`. Every read and write is guarded: an unavailable store or an invalid value means `system`.
- The Pinia theme store (`src/theme/store.ts`) resolves the preference, follows the system's dark mode through `matchMedia` as it changes, and applies the result.
- An inline script in `index.html` applies the saved theme before anything renders, so the page never shows the wrong theme first. A unit test runs that script for every combination of saved value and system mode, and checks it agrees with the store.

**Dev page:** `/#/dev/tokens` shows every token in both themes, side by side. It exists only in development builds.

## Consequences

- The look is changed in one place, and both themes are complete by construction: the tests fail if a themed token is missing from either theme, or if a pairing loses contrast.
- Components must use semantic tokens, not palette tokens or literal colours. Review enforces this, and a lint rule can follow if needed.
- The inline bootstrap script will need a hash in the Tauri app's Content Security Policy. Tauri can add hashes for inline scripts at build time.
- The app differs slightly from the reference where accessibility requires it: control borders are darker, and the light-theme accent text is a little deeper.

## Alternatives considered

- **A token build pipeline** (for example Style Dictionary, generating CSS and TypeScript). It's more machinery than one stylesheet and one list need at this size.
- **A CSS framework with its own theming,** as OpenFlexure uses UIkit with LESS. It would be less original, heavier, and its look would show through.
- **Theming only through `prefers-color-scheme`.** It offers no user choice and nothing to persist.
- **A `.dark` class instead of `data-theme`.** Equivalent, but an attribute with a value says "this theme" more clearly, and covers more themes if they ever come.
- **pinia-plugin-persistedstate for the preference.** It's another dependency, and the bootstrap script must read the stored value directly anyway.
- **A web font close to ProximaNova.** It's a commercial font, and downloading fonts conflicts with working offline.
