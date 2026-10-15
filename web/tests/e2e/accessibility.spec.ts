import { AxeBuilder } from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

import { open, SHOTS, STEADY } from '../visual/pages'

// Accessibility (P47, ADR-0034): axe's WCAG 2.1 A and AA rules on every tab,
// Settings section and key state, in both themes; and, with the keyboard
// alone, every stop on each page in turn, each showing its focus ring.
test.use({ ...STEADY, viewport: { width: 1280, height: 800 } })

const WCAG = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']

for (const colorScheme of ['light', 'dark'] as const) {
  test.describe(colorScheme, () => {
    test.use({ colorScheme })
    for (const shot of SHOTS) {
      test(`axe finds nothing wrong on ${shot.name}`, async ({ page }) => {
        await open(page, shot)
        const { violations } = await new AxeBuilder({ page }).withTags(WCAG).analyze()
        const found = violations.map(
          (violation) =>
            `${violation.id}: ${violation.nodes.map((node) => node.target).join(', ')}`,
        )
        expect(found).toEqual([])
      })
    }
  })
}

for (const shot of SHOTS.filter((shot) => !shot.then)) {
  test(`Tab reaches every control on ${shot.name}, each with its focus ring`, async ({ page }) => {
    await open(page, shot)
    const stops: string[] = []
    // Until focus comes back round, or leaves the page.
    for (let tab = 0; tab < 200; tab++) {
      await page.keyboard.press('Tab')
      const stop = await page.evaluate(() => {
        const element = document.activeElement as HTMLElement | null
        if (!element || element === document.body || element.dataset.visited) return null
        element.dataset.visited = 'yes'
        const { outlineStyle, outlineWidth } = getComputedStyle(element)
        const name = element.getAttribute('aria-label') ?? element.textContent ?? ''
        const ring = outlineStyle !== 'none' && parseFloat(outlineWidth) > 0
        return `${element.tagName} “${name.trim().slice(0, 40)}”${ring ? '' : ' without a ring'}`
      })
      if (!stop) break
      stops.push(stop)
    }
    // The rail's ten stops, and the page's own.
    expect(stops.length).toBeGreaterThan(10)
    expect(stops.filter((stop) => stop.endsWith('without a ring'))).toEqual([])
  })
}
