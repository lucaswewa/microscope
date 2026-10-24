// Accent colours (ADR-0047): nine to choose from in Settings, Display, and
// more to try on the dev pages, kept in this browser. Magenta is the
// stylesheet's own. Another replaces the accent palette, --palette-accent-50
// to -900, whose 500, the accent itself, is the colour; the lighter and
// darker steps keep the magenta's proportions of lightness (tokens.css), so
// the colours stay as different as they are. Then each theme's accent tokens
// take the steps that keep their contrast, as the token test checks for the
// nine.
import { defineStore } from 'pinia'
import { ref } from 'vue'

import { contrast } from './contrast'
import type { Theme } from './store'

/**
 * The accents: the app's own, then other magentas, blues, violets and
 * greens. The other magentas and the greens are as Wikipedia's "Shades of
 * magenta" and "Shades of green" give them.
 */
export const ACCENTS = [
  { id: 'magenta', label: 'Magenta (default)', hex: '#ce2283' },
  { id: 'magentadye', label: 'Magenta dye', hex: '#ca1f7b' },
  { id: 'subtractivemagenta', label: 'Magenta (subtractive primary)', hex: '#ff0090' },
  { id: 'africanviolet', label: 'African violet', hex: '#b284be' },
  { id: 'amaranthpurple', label: 'Amaranth purple', hex: '#ab274f' },
  { id: 'chineseviolet', label: 'Chinese violet', hex: '#856088' },
  { id: 'magentahaze', label: 'Magenta haze', hex: '#9f4576' },
  { id: 'telemagenta', label: 'Telemagenta', hex: '#cf3476' },
  { id: 'midnightblue', label: 'Midnight blue', hex: '#191970' },
  { id: 'darkblue', label: 'Dark blue', hex: '#00008b' },
  { id: 'navyblue', label: 'Navy blue', hex: '#000080' },
  { id: 'mediumblue', label: 'Medium blue', hex: '#0000cd' },
  { id: 'inkblue', label: 'Ink blue', hex: '#2e2787' },
  { id: 'duskblue', label: 'Dusk blue', hex: '#545aa7' },
  { id: 'harborblue', label: 'Harbor blue', hex: '#205896' },
  { id: 'violet', label: 'Violet', hex: '#ee82ee' },
  { id: 'vividviolet', label: 'Vivid violet', hex: '#9f00ff' },
  { id: 'electricviolet', label: 'Electric violet', hex: '#8f00ff' },
  { id: 'psychologicalgreen', label: 'Psychological primary green', hex: '#009f6b' },
  { id: 'pantonegreen', label: 'Pantone green', hex: '#00ad43' },
  { id: 'crayolagreen', label: 'Crayola green', hex: '#1cac78' },
  { id: 'acidgreen', label: 'Acid green', hex: '#b0bf1a' },
  { id: 'evergreen', label: 'Evergreen', hex: '#05472a' },
  { id: 'ferngreen', label: 'Fern green', hex: '#4f7942' },
  { id: 'junglegreen', label: 'Jungle green', hex: '#29ab87' },
  { id: 'mantis', label: 'Mantis', hex: '#74c365' },
  { id: 'malachite', label: 'Malachite', hex: '#0bda51' },
  { id: 'huntergreen', label: 'Hunter green', hex: '#355e3b' },
] as const

export type AccentId = (typeof ACCENTS)[number]['id']

/** The accents Settings offers, in its groups. */
export const SETTINGS_ACCENTS: { group: string; ids: AccentId[] }[] = [
  { group: 'Magentas', ids: ['magenta', 'amaranthpurple', 'chineseviolet'] },
  { group: 'Greens', ids: ['ferngreen', 'mantis', 'huntergreen'] },
  { group: 'Blues', ids: ['midnightblue', 'duskblue', 'harborblue'] },
]

/** Where the choice is kept. */
export const ACCENT_STORAGE_KEY = 'microscope.accent'

/**
 * The surfaces accent text and focus outlines stand on in each theme, and
 * the text that can go on the accent, white first: tokens.css's colours,
 * which the token test checks.
 */
export const SURFACES: Record<Theme, Record<string, string>> = {
  light: {
    '--color-bg': '#ffffff',
    '--color-surface-panel': '#fafafa',
    '--color-surface-rail': '#f2f2f2',
    '--color-surface-raised': '#ffffff',
  },
  dark: {
    '--color-bg': '#1f1f1f',
    '--color-surface-panel': '#232323',
    '--color-surface-rail': '#2b2b2b',
    '--color-surface-raised': '#2a2a2a',
  },
}
export const ON_ACCENT = { '--palette-neutral-0': '#ffffff', '--palette-neutral-950': '#1a1a1a' }

/** Each step of the accent palette, and the magenta's HSL lightness there. */
const STEPS = [
  [50, 95],
  [100, 88],
  [200, 78],
  [300, 70],
  [400, 60],
  [500, 47],
  [600, 41],
  [700, 34],
  [800, 25],
  [900, 15],
] as const
const MAGENTA_500 = 47

/** A #rrggbb colour's hue (degrees), saturation and lightness (percent). */
function toHsl(hex: string): [number, number, number] {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [
    number,
    number,
    number,
  ]
  const [high, low] = [Math.max(r, g, b), Math.min(r, g, b)]
  const lightness = (high + low) / 2
  const range = high - low
  if (range === 0) return [0, 0, lightness * 100]
  const saturation = range / (1 - Math.abs(2 * lightness - 1))
  const sector =
    high === r ? (g - b) / range + 6 : high === g ? (b - r) / range + 2 : (r - g) / range + 4
  return [(sector * 60) % 360, saturation * 100, lightness * 100]
}

/** The #rrggbb colour of a hue, saturation and lightness. */
function toHex(hue: number, saturation: number, lightness: number): string {
  const [s, l] = [saturation / 100, lightness / 100]
  const channel = (n: number) => {
    const k = (n + hue / 30) % 12
    const value = l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1))
    return Math.round(value * 255)
      .toString(16)
      .padStart(2, '0')
  }
  return `#${channel(0)}${channel(8)}${channel(4)}`
}

/**
 * The accent palette for `hex`: each step's token and colour. Steps lighter
 * than the magenta's 500 share out the lightness between `hex`'s and white
 * as the magenta's do; darker steps, between `hex`'s and black.
 */
export function accentPalette(hex: string): Record<string, string> {
  const [hue, saturation, lightness] = toHsl(hex)
  return Object.fromEntries(
    STEPS.map(([step, magenta]) => {
      const shade =
        magenta >= MAGENTA_500
          ? 100 - ((100 - magenta) * (100 - lightness)) / (100 - MAGENTA_500)
          : (magenta * lightness) / MAGENTA_500
      return [`--palette-accent-${step}`, step === 500 ? hex : toHex(hue, saturation, shade)]
    }),
  )
}

/** An accent's tokens: its palette, and each theme's accent tokens. */
export interface AccentTokens {
  palette: Record<string, string>
  light: Record<string, string>
  dark: Record<string, string>
}

/**
 * The tokens of accent `hex`. The accent fills are the colour, with white on
 * them if it's readable there, or else the darkest text, and they darken
 * under white and lighten under dark text when hovered and pressed. Accent
 * text, headings and focus outlines take the stylesheet's step, or the
 * nearest one beyond it that stands out from every surface.
 */
export function accentTokens(hex: string): AccentTokens {
  const palette = accentPalette(hex)
  const colour = (step: number) => palette[`--palette-accent-${step}`]!
  const token = (step: number) => `var(--palette-accent-${step})`
  const onWhite = contrast(ON_ACCENT['--palette-neutral-0'], hex) >= 4.5
  const on = onWhite ? 'var(--palette-neutral-0)' : 'var(--palette-neutral-950)'
  const [hover, pressed] = onWhite ? [600, 700] : [400, 300]
  const fills = {
    '--color-accent': token(500),
    '--color-accent-hover': token(hover),
    '--color-accent-pressed': token(pressed),
    '--color-on-accent': on,
    '--color-rail-active-bg': token(500),
    '--color-rail-active-text': on,
  }
  const first = (steps: number[], surfaces: string[], minimum: number) =>
    steps.find((step) => surfaces.every((surface) => contrast(colour(step), surface) >= minimum)) ??
    steps.at(-1)!
  const theme = (name: Theme, steps: number[], subtle: number) => {
    const surfaces = Object.values(SURFACES[name])
    const text = token(first(steps, [...surfaces, colour(subtle)], 4.5))
    const tokens: Record<string, string> = {
      ...fills,
      '--color-accent-text': text,
      '--color-accent-subtle': token(subtle),
      '--color-focus': token(first(steps, surfaces, 3)),
    }
    if (name === 'light') tokens['--color-text-heading'] = text
    return tokens
  }
  return {
    palette,
    light: theme('light', [600, 700, 800, 900], 50),
    dark: theme('dark', [300, 200, 100, 50], 900),
  }
}

/** The id of the stylesheet that gives the page an accent other than magenta. */
const STYLE_ID = 'accent-tokens'

/** Gives the page accent `id`: magenta is the stylesheet's own. */
export function applyAccent(id: AccentId, page: Document = document) {
  page.getElementById(STYLE_ID)?.remove()
  const accent = ACCENTS.find((accent) => accent.id === id)
  if (!accent || id === 'magenta') return
  const { palette, light, dark } = accentTokens(accent.hex)
  const block = (selector: string, tokens: Record<string, string>) =>
    `${selector} {\n${Object.entries(tokens)
      .map(([token, value]) => `  ${token}: ${value};\n`)
      .join('')}}\n`
  const style = page.createElement('style')
  style.id = STYLE_ID
  // As tokens.css's blocks, but the light one leaves a dark page's root
  // alone: the dark one doesn't set the light one's heading back.
  style.textContent =
    block(':root', palette) +
    block(":root:not([data-theme='dark']),\n[data-theme='light']", light) +
    block("[data-theme='dark']", dark)
  page.head.append(style)
}

/** The accent chosen in this browser: magenta if none, or storage fails. */
export function readAccent(): AccentId {
  try {
    const saved = window.localStorage.getItem(ACCENT_STORAGE_KEY)
    return ACCENTS.find((accent) => accent.id === saved)?.id ?? 'magenta'
  } catch {
    return 'magenta'
  }
}

/** Keeps the choice in this browser, if it can. */
export function writeAccent(id: AccentId) {
  try {
    window.localStorage.setItem(ACCENT_STORAGE_KEY, id)
  } catch {
    // Without storage, the accent lasts until the page reloads.
  }
}

/** The accent: the choice kept in this browser, given to the page. */
export const useAccentStore = defineStore('accent', () => {
  const accent = ref<AccentId>(readAccent())
  applyAccent(accent.value)

  /** Chooses accent `id`, keeps it, and gives it to the page. */
  function choose(id: AccentId) {
    accent.value = id
    writeAccent(id)
    applyAccent(id)
  }

  return { accent, choose }
})
