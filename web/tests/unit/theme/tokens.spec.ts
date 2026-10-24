import { readFileSync } from 'node:fs'
import { dirname, resolve as resolvePath } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import { CONTRAST_PAIRS, contrast } from '@/theme/contrast'
import { THEMED_GROUPS, TOKEN_GROUPS } from '@/theme/tokens'

// Read from disk, by path: Vitest doesn't load stylesheets, even with ?raw,
// and in the happy-dom environment URL is happy-dom's, which Node's fs refuses.
const here = dirname(fileURLToPath(import.meta.url))
const css = readFileSync(resolvePath(here, '../../../src/theme/tokens.css'), 'utf8')

/** The custom properties declared in each top-level block of tokens.css. */
function blocks(source: string): Map<string, Map<string, string>> {
  const text = source.replace(/\/\*[\s\S]*?\*\//g, '')
  const result = new Map<string, Map<string, string>>()
  let depth = 0
  let start = 0
  let selector = ''
  for (let i = 0; i < text.length; i++) {
    if (text[i] === '{') {
      if (depth === 0) {
        selector = text.slice(start, i).trim().replace(/\s+/g, ' ')
        start = i + 1
      }
      depth++
    } else if (text[i] === '}') {
      depth--
      if (depth === 0) {
        const declarations = new Map<string, string>()
        for (const [, name, value] of text.slice(start, i).matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
          declarations.set(name!, value!.trim().replace(/\s+/g, ' '))
        }
        result.set(selector, declarations)
        start = i + 1
      }
    }
  }
  return result
}

const parsed = blocks(css)
const root = parsed.get(':root')!
const themes = {
  light: parsed.get(":root, [data-theme='light']")!,
  dark: parsed.get("[data-theme='dark']")!,
}

/** A token's value in a theme, following var() references to a colour. */
function resolve(theme: keyof typeof themes, token: string): string {
  const value = themes[theme].get(token) ?? root.get(token)
  if (value === undefined) throw new Error(`${token} isn't defined`)
  const reference = /^var\((--[\w-]+)\)$/.exec(value)
  return reference ? resolve(theme, reference[1]!) : value
}

describe('the design tokens', () => {
  it('lists exactly the tokens that tokens.css defines', () => {
    const defined = new Set([...root.keys(), ...themes.light.keys(), ...themes.dark.keys()])
    const listed = new Set(Object.values(TOKEN_GROUPS).flat())
    expect([...defined].sort()).toEqual([...listed].sort())
  })

  it('defines every themed token in both themes, and only those', () => {
    const themed = THEMED_GROUPS.flatMap((group) => TOKEN_GROUPS[group]).sort()
    expect([...themes.light.keys()].sort()).toEqual(themed)
    expect([...themes.dark.keys()].sort()).toEqual(themed)
  })

  it('turns motion off for people who ask for reduced motion', () => {
    const reduced = blocks(
      /@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?\})\s*\}/.exec(css)![1]!,
    )
    expect(reduced.get(':root')?.get('--duration-fast')).toBe('0ms')
    expect(reduced.get(':root')?.get('--duration-normal')).toBe('0ms')
  })

  for (const theme of ['light', 'dark'] as const) {
    describe(`in the ${theme} theme`, () => {
      for (const [foreground, background, minimum] of CONTRAST_PAIRS) {
        it(`${foreground} on ${background} has a contrast of at least ${minimum}:1`, () => {
          const ratio = contrast(resolve(theme, foreground), resolve(theme, background))
          expect(ratio).toBeGreaterThanOrEqual(minimum)
        })
      }
    })
  }
})
