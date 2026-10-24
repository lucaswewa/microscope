# ADR-0047: Accent colours a person can choose

- Status: Accepted
- Date: 2026-10-24
- Phase: M2, outside a phase
- Supersedes: [ADR-0011](0011-design-tokens-and-theming.md), in part: magenta as the only accent

## Context

- **ADR-0011 gives the app one accent,** magenta. The accent palette, `--palette-accent-50` to `-900`, is hue 326° at fixed lightnesses. Each theme's accent tokens take steps of it:
  - fills at 500, darker when hovered and pressed, with white on them;
  - accent text, headings and focus outlines at 600 in the light theme, and at 300 in the dark.
- **A unit test checks 29 contrast pairs** in both themes, to WCAG 2.2 AA: 4.5:1 for text, 3:1 for what people must see.
- **The dev pages' Accent picker,** added outside a phase, tries other colours: their palettes keep the colour at 500, and magenta's proportions of lightness for the other steps.
- **The owner wants nine accents** people can choose in Settings: magenta, amaranth purple and Chinese violet; fern green, mantis and hunter green; midnight blue, dusk blue and harbor blue.
- **Seven pass every pair on magenta's steps. Two don't:**
  - mantis is too light for white text, 2.2:1, and for its accent text on white;
  - midnight blue's dark-theme accent text and focus outline are too dark on the dark surfaces.
- **The theme is chosen per browser,** in `localStorage`, under Settings, Display.

## Decision

- **Settings, Display, has an Accent colour choice** of the nine, in three groups, each with its swatch. Magenta is the default.
- **It's kept per browser,** in `localStorage` (`microscope.accent`), as the theme is. The app gives the page its accent before it shows.
- **Magenta is the stylesheet's own,** `tokens.css`, unchanged.
- **Another accent's tokens** are a stylesheet of their own, added to the page's `<head>`, with the stylesheet's blocks:
  - the palette, from the colour, as the dev pages make it;
  - each theme's accent tokens. The light theme's block leaves the root of a dark page alone.
- **Each theme's accent tokens keep their contrast:**
  - the fills are the colour itself, at 500;
  - on them, white if it reaches 4.5:1, or else the darkest text, `--palette-neutral-950`;
  - hovered and pressed, the fills darken, 600 and 700, under white, and lighten, 400 and 300, under dark text;
  - accent text and headings take the stylesheet's step, or the nearest beyond it, towards the theme's contrast, that reaches 4.5:1 on every surface and on the accent's subtle background;
  - focus outlines take the same steps, to 3:1 on every surface.
- **The token test checks every accent Settings offers,** with the same 29 pairs in both themes. It also checks that the surfaces and text colours the accents are measured against are `tokens.css`'s.
- **The dev pages offer every accent,** the nine and 19 others to try. They give the page the same tokens, and keep the same choice.
- **`AppSelect` can group its options and show swatches,** for this and for any other list.

## Consequences

- **Every accent Settings offers meets WCAG AA** in both themes, and a new one can't be offered without passing the test.
- **An adjusted accent isn't its colour everywhere.** Mantis buttons have dark text. Its text on white is a darker green, and midnight blue's text in the dark theme is a lighter blue.
- **The accent is per browser,** not per microscope. Two people at one microscope can see different accents. The desktop app (P17) will keep it as the browser does.
- **A colour in the middle of the range of lightness** could fail with both white and the darkest text, at about 4.2:1 at worst. None of the nine does, and the test would catch one that did.
- **The surfaces are written twice,** in `tokens.css` and in `accents.ts`, which the test keeps in step.

## Alternatives considered

- **Each accent hand-written in `tokens.css`,** a block for each theme: 18 blocks to keep, with the same steps chosen by hand.
- **Mantis and midnight blue as they are,** failing contrast in places.
- **Dropping mantis and midnight blue:** the owner chose to keep them, adjusted.
- **Any colour, from a colour picker:** a colour too close to the middle of the range of lightness can't be made to pass, and every colour would need checking at run time.
- **The accent as a setting of the microscope,** on the server: one choice for everyone at a microscope, but it belongs with the theme, which is the browser's.
