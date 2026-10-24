import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  ACCENT_STORAGE_KEY,
  ACCENTS,
  accentPalette,
  accentTokens,
  applyAccent,
  readAccent,
  useAccentStore,
  writeAccent,
} from '@/theme/accents'
import { luminance } from '@/theme/contrast'
import AccentPicker from '@/views/dev/AccentPicker.vue'

const STEPS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900]
const style = () => document.getElementById('accent-tokens')
const hexOf = (id: string) => ACCENTS.find((accent) => accent.id === id)!.hex

beforeEach(() => setActivePinia(createPinia()))

afterEach(() => {
  window.localStorage.clear()
  applyAccent('magenta')
})

describe('accent colours', () => {
  it('make a palette whose 500 is the colour, lightest at 50 and darkest at 900', () => {
    for (const { hex } of ACCENTS) {
      const palette = accentPalette(hex)
      expect(Object.keys(palette)).toEqual(STEPS.map((step) => `--palette-accent-${step}`))
      expect(palette['--palette-accent-500']).toBe(hex)
      const shades = Object.values(palette).map(luminance)
      for (let i = 1; i < shades.length; i++) expect(shades[i]).toBeLessThan(shades[i - 1]!)
    }
    // Blues alike in hue stay as different as they are.
    const blue = (hex: string) => accentPalette(hex)['--palette-accent-300']
    expect(blue('#00008b')).not.toBe(blue('#0000cd'))
  })

  it('keep their tokens readable: dark text on a light accent, lighter text on a dark one', () => {
    // Harbor blue keeps the stylesheet's steps, with white on it.
    const harbor = accentTokens(hexOf('harborblue'))
    expect(harbor.light['--color-on-accent']).toBe('var(--palette-neutral-0)')
    expect(harbor.light['--color-accent-hover']).toBe('var(--palette-accent-600)')
    expect(harbor.light['--color-accent-text']).toBe('var(--palette-accent-600)')
    expect(harbor.dark['--color-accent-text']).toBe('var(--palette-accent-300)')
    // Mantis is too light for white, so its fills lighten under dark text,
    // and its text on white is darker.
    const mantis = accentTokens(hexOf('mantis'))
    expect(mantis.light['--color-on-accent']).toBe('var(--palette-neutral-950)')
    expect(mantis.light['--color-rail-active-text']).toBe('var(--palette-neutral-950)')
    expect(mantis.light['--color-accent-hover']).toBe('var(--palette-accent-400)')
    expect(mantis.light['--color-accent-text']).not.toBe('var(--palette-accent-600)')
    expect(mantis.light['--color-text-heading']).toBe(mantis.light['--color-accent-text'])
    // Midnight blue's text on the dark theme is lighter than its 300.
    const midnight = accentTokens(hexOf('midnightblue'))
    expect(['var(--palette-accent-200)', 'var(--palette-accent-100)']).toContain(
      midnight.dark['--color-accent-text'],
    )
    expect(midnight.dark['--color-text-heading']).toBeUndefined()
  })

  it('are given to the page in a stylesheet of their own; magenta is the stylesheet’s own', () => {
    applyAccent('midnightblue')
    const text = style()!.textContent!
    expect(text).toContain('--palette-accent-500: #191970;')
    expect(text).toMatch(/\n\[data-theme='light'\] \{[^}]*--color-accent-text/)
    expect(text).toMatch(/\n\[data-theme='dark'\] \{[^}]*--color-accent-text/)
    // A dark page doesn't take the light theme's accent heading.
    document.documentElement.dataset.theme = 'dark'
    const root = getComputedStyle(document.documentElement)
    expect(root.getPropertyValue('--color-text-heading')).toBe('')
    document.documentElement.dataset.theme = 'light'
    expect(root.getPropertyValue('--color-text-heading')).not.toBe('')
    delete document.documentElement.dataset.theme
    applyAccent('telemagenta')
    expect(document.querySelectorAll('#accent-tokens')).toHaveLength(1)
    applyAccent('magenta')
    expect(style()).toBeNull()
  })

  it('are kept in this browser, magenta when there’s none or it’s unknown', () => {
    expect(readAccent()).toBe('magenta')
    writeAccent('vividviolet')
    expect(readAccent()).toBe('vividviolet')
    window.localStorage.setItem(ACCENT_STORAGE_KEY, 'teal')
    expect(readAccent()).toBe('magenta')
  })

  it('are given to the page by the store, as it starts and when one is chosen', () => {
    writeAccent('duskblue')
    const accent = useAccentStore()
    expect(accent.accent).toBe('duskblue')
    expect(style()!.textContent).toContain('#545aa7')
    accent.choose('magenta')
    expect([readAccent(), style()]).toEqual(['magenta', null])
  })

  it('are all on the dev pages, for the whole app', async () => {
    writeAccent('navyblue')
    const picker = mount(AccentPicker)
    const checked = () => picker.find<HTMLInputElement>('input:checked').element.value
    expect(checked()).toBe('navyblue')
    await picker.find('input[value="electricviolet"]').setValue(true)
    expect(checked()).toBe('electricviolet')
    expect(style()!.textContent).toContain('#8f00ff')
    expect(readAccent()).toBe('electricviolet')
    expect(picker.emitted('change')).toEqual([['electricviolet']])
    expect(picker.findAll('label').map((label) => label.text())).toEqual(
      ACCENTS.map((accent) => accent.label),
    )
  })
})
