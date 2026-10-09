# ADR-0012: Icon set

- Status: Accepted
- Date: 2026-10-09
- Phase: P07

## Context

- The navigation rail shows an icon above each label, as OpenFlexure's does, and later phases need icons for controls: the d-pad, capture, cancel, and so on.
- The icons must work offline (ADR-0010: no CDN), follow the theme's colours, render the same on every machine (P07's visual baselines), and carry a licence compatible with MIT distribution (ADR-0002).
- OpenFlexure uses Google's Material Symbols as a web font. P05 measured its rail icons at 24 px.

## Decision

- **The icon set is Material Symbols Outlined:** weight 400, unfilled, grade 0. They're Google's icons under the Apache License 2.0.
- **Each icon comes from the `@material-symbols/svg-400` package as its own SVG file,** imported as raw text (`…/outlined/settings.svg?raw`). Only the icons the app imports are bundled. The package is a development dependency, used at build time.
- **`src/ui/AppIcon.vue` draws an icon** from that text: it reads the viewBox and paths and renders an inline `<svg>` with `fill="currentColor"`, hidden from assistive technology, at `--icon-size-md` (24 px) unless sized otherwise. It renders no HTML from strings (`v-html`).
- **Icons are chosen for meaning,** not copied from OpenFlexure's choices:
  - `visibility` for View;
  - `control_camera` for Control;
  - `grid_on` for Slide Scan;
  - `timelapse` for Sequence;
  - `photo_library` for Gallery;
  - `settings` for Settings;
  - `receipt_long` for Logging;
  - `info` for About;
  - `power_settings_new` for Power.
- **`NOTICE`** at the repository root gives the attribution the licence asks for. The release packaging (P49) gathers it with the other third-party licences.

## Consequences

- No font to download or subset, so no flash of missing icons, and screenshots are stable.
- Adding an icon is one import. The package holds every Material Symbol, so there's no build step.
- Icons are vector paths, so they scale cleanly, and they follow the text colour in both themes and in the rail's active item.
- The package is 13 MB in `node_modules`, though the bundle carries only what the app uses.

## Alternatives considered

- **The Material Symbols web font**, as OpenFlexure uses it. It's a multi-megabyte font, or needs a subsetting pipeline, with a flash of text before it loads, and ligature names in the markup.
- **Lucide** (ISC). A good set, but a thinner, different look, further from the reference's proportions.
- **unplugin-icons or Iconify.** They'd add build plugins for something one small component does.
- **Drawing our own icons.** Effort better spent elsewhere, for no gain.
