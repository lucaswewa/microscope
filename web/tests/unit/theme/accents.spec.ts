import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'

import {
  ACCENT_STORAGE_KEY,
  ACCENTS,
  accentPalette,
  applyAccent,
  readAccent,
  writeAccent,
} from '@/theme/accents'
import { luminance } from '@/theme/contrast'
import AccentPicker from '@/views/dev/AccentPicker.vue'

const STEPS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900]
const root = () => document.documentElement.style

afterEach(() => {
  window.localStorage.clear()
  applyAccent('magenta')
})

describe('accent colours to try', () => {
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

  it('are given to the page, and magenta is the stylesheet’s own', () => {
    applyAccent('midnightblue')
    expect(root().getPropertyValue('--palette-accent-500')).toBe('#191970')
    expect(root().getPropertyValue('--palette-accent-50')).toMatch(/^#[0-9a-f]{6}$/)
    applyAccent('magenta')
    expect(STEPS.map((step) => root().getPropertyValue(`--palette-accent-${step}`))).toEqual(
      STEPS.map(() => ''),
    )
  })

  it('are kept in this browser, magenta when there’s none or it’s unknown', () => {
    expect(readAccent()).toBe('magenta')
    writeAccent('vividviolet')
    expect(readAccent()).toBe('vividviolet')
    window.localStorage.setItem(ACCENT_STORAGE_KEY, 'teal')
    expect(readAccent()).toBe('magenta')
  })

  it('are chosen on the dev pages, for the whole app', async () => {
    writeAccent('navyblue')
    const picker = mount(AccentPicker)
    const checked = () => picker.find<HTMLInputElement>('input:checked').element.value
    expect(checked()).toBe('navyblue')
    await picker.find('input[value="electricviolet"]').setValue(true)
    expect(checked()).toBe('electricviolet')
    expect(root().getPropertyValue('--palette-accent-500')).toBe('#8f00ff')
    expect(readAccent()).toBe('electricviolet')
    expect(picker.emitted('change')).toEqual([['electricviolet']])
    expect(picker.findAll('label').map((label) => label.text())).toEqual(
      ACCENTS.map((accent) => accent.label),
    )
  })
})
