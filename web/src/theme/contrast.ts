// WCAG 2 contrast between the design tokens' colours: a unit test checks the
// pairs below in both themes, and the tokens page shows them.

/** The relative luminance of a #rrggbb colour. */
export function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [
    number,
    number,
    number,
  ]
  const linear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/** The WCAG 2 contrast ratio between two #rrggbb colours, from 1 to 21. */
export function contrast(a: string, b: string): number {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number]
  return (high + 0.05) / (low + 0.05)
}

// [foreground, background, minimum ratio]: 4.5 for text, 3 for other things
// people must see (WCAG 2.2 AA, 1.4.3 and 1.4.11).
export const CONTRAST_PAIRS: [string, string, number][] = [
  ['--color-text', '--color-bg', 4.5],
  ['--color-text', '--color-surface-panel', 4.5],
  ['--color-text', '--color-surface-raised', 4.5],
  ['--color-text-muted', '--color-bg', 4.5],
  ['--color-text-muted', '--color-surface-panel', 4.5],
  ['--color-text-heading', '--color-bg', 4.5],
  ['--color-text-heading', '--color-surface-panel', 4.5],
  ['--color-accent-text', '--color-bg', 4.5],
  ['--color-accent-text', '--color-surface-panel', 4.5],
  ['--color-accent-text', '--color-surface-rail', 4.5],
  ['--color-accent-text', '--color-surface-raised', 4.5],
  ['--color-text', '--color-accent-subtle', 4.5],
  ['--color-accent-text', '--color-accent-subtle', 4.5],
  ['--color-on-accent', '--color-accent', 4.5],
  ['--color-on-accent', '--color-accent-hover', 4.5],
  ['--color-on-accent', '--color-accent-pressed', 4.5],
  ['--color-control-text', '--color-control-bg', 4.5],
  ['--color-control-border', '--color-control-bg', 3],
  ['--color-control-border', '--color-bg', 3],
  ['--color-control-border', '--color-surface-panel', 3],
  ['--color-rail-text', '--color-surface-rail', 4.5],
  ['--color-rail-text', '--color-rail-hover-bg', 4.5],
  ['--color-rail-active-text', '--color-rail-active-bg', 4.5],
  ['--color-focus', '--color-bg', 3],
  ['--color-focus', '--color-surface-panel', 3],
  ['--color-danger', '--color-bg', 4.5],
  ['--color-warning', '--color-bg', 4.5],
  ['--color-success', '--color-bg', 4.5],
  ['--color-info', '--color-bg', 4.5],
]
