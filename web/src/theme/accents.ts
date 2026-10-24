// Accent colours to try, in development builds only (the dev pages' Accent
// picker). One replaces the accent palette, --palette-accent-50 to -900, on
// the whole page, and is kept in this browser. Its 500, the accent itself,
// is the colour; the lighter and darker steps keep the magenta's proportions
// of lightness (tokens.css), so the colours stay as different as they are.

/**
 * The accents to try: the app's own, then blues, violets and greens. The
 * greens are as Wikipedia's "Shades of green" gives them.
 */
export const ACCENTS = [
  { id: 'magenta', label: 'Magenta, the app’s', hex: '#ce2283' },
  { id: 'midnightblue', label: 'Midnight blue', hex: '#191970' },
  { id: 'darkblue', label: 'Dark blue', hex: '#00008b' },
  { id: 'navyblue', label: 'Navy blue', hex: '#000080' },
  { id: 'mediumblue', label: 'Medium blue', hex: '#0000cd' },
  { id: 'inkblue', label: 'Ink blue', hex: '#2e2787' },
  { id: 'duskblue', label: 'Dusk blue', hex: '#545aa7' },
  { id: 'harbourblue', label: 'Harbour blue', hex: '#205896' },
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

/** Where the choice is kept. */
export const ACCENT_STORAGE_KEY = 'microscope.dev.accent'

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

/** Gives the page accent `id`: magenta is the stylesheet's own. */
export function applyAccent(id: AccentId, root: HTMLElement = document.documentElement) {
  const accent = ACCENTS.find((accent) => accent.id === id)
  const palette = accent && id !== 'magenta' ? accentPalette(accent.hex) : {}
  for (const [step] of STEPS) {
    const token = `--palette-accent-${step}`
    if (palette[token]) root.style.setProperty(token, palette[token])
    else root.style.removeProperty(token)
  }
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
