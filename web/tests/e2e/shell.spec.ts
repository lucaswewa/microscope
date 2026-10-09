import { expect, test } from '@playwright/test'

/**
 * OpenFlexure's rail at 1280×800, as measured in P05 (docs/milestone-1/notes/
 * P05-reference-screenshots.md), and how far this app may differ from it.
 */
const REFERENCE = {
  railWidth: { value: 85, tolerance: 2 }, // including the right border
  itemHeight: { value: 67, tolerance: 4 },
  iconSize: { value: 24, tolerance: 0 },
  labelSize: { value: 14, tolerance: 1 },
}

test.use({ viewport: { width: 1280, height: 800 } })

test('clicking a destination goes there and marks it', async ({ page }) => {
  await page.goto('/#/view')
  const rail = page.getByRole('navigation', { name: 'Main' })
  await rail.getByRole('link', { name: 'Gallery' }).click()
  await expect(page).toHaveURL(/#\/gallery$/)
  await expect(rail.getByRole('link', { name: 'Gallery' })).toHaveAttribute('aria-current', 'page')
  await expect(page.getByRole('heading', { name: 'Gallery' })).toBeVisible()
})

test('Shift+↓ and Shift+↑ switch tabs', async ({ page }) => {
  await page.goto('/#/view')
  await page.keyboard.press('Shift+ArrowDown')
  await expect(page).toHaveURL(/#\/control$/)
  await page.keyboard.press('Shift+ArrowUp')
  await page.keyboard.press('Shift+ArrowUp')
  await expect(page).toHaveURL(/#\/power$/)
})

test("the rail's proportions match the reference's", async ({ page }) => {
  await page.goto('/#/control')
  const measured = await page.evaluate(() => {
    const rail = document.querySelector('nav')!.getBoundingClientRect()
    const item = document.querySelector('.rail__item')!.getBoundingClientRect()
    const icon = document.querySelector('.rail__item svg')!.getBoundingClientRect()
    const label = getComputedStyle(document.querySelector('.rail__label')!)
    return {
      railWidth: rail.width,
      itemHeight: item.height,
      iconSize: icon.width,
      labelSize: parseFloat(label.fontSize),
    }
  })
  for (const [name, { value, tolerance }] of Object.entries(REFERENCE)) {
    const actual = measured[name as keyof typeof measured]
    expect(Math.abs(actual - value), `${name}: ${actual} against ${value}`).toBeLessThanOrEqual(
      tolerance,
    )
  }
})

test.describe('in a narrow window', () => {
  test.use({ viewport: { width: 800, height: 600 } })

  test('a notice says the interface needs a wider window', async ({ page }) => {
    await page.goto('/#/view')
    await expect(page.getByRole('note')).toContainText('wide, landscape window')
  })
})

test('in a wide window there is no notice', async ({ page }) => {
  await page.goto('/#/view')
  await expect(page.getByRole('note')).toBeHidden()
})
