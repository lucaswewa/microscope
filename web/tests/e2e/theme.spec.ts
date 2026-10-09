import { expect, test } from '@playwright/test'

const DARK_BG = 'rgb(31, 31, 31)'
const LIGHT_BG = 'rgb(255, 255, 255)'

const bodyBackground = (page: import('@playwright/test').Page) =>
  page.evaluate(() => getComputedStyle(document.body).backgroundColor)

test.describe('with the system in dark mode', () => {
  test.use({ colorScheme: 'dark' })

  test('the app follows the system theme', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
    await expect(page.getByRole('navigation', { name: 'Main' })).toBeVisible()
    expect(await bodyBackground(page)).toBe(DARK_BG)
  })
})

test.describe('with the system in light mode', () => {
  test.use({ colorScheme: 'light' })

  test('a saved preference wins over the system theme', async ({ page }) => {
    await page.addInitScript(() => window.localStorage.setItem('microscope.theme', 'dark'))
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
    expect(await bodyBackground(page)).toBe(DARK_BG)
  })

  test('the saved theme is applied before the app runs', async ({ page }) => {
    // Block the app's scripts: only index.html's bootstrap can set the theme.
    await page.route('**/assets/*.js', (route) => route.abort())
    await page.addInitScript(() => window.localStorage.setItem('microscope.theme', 'dark'))
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  })

  test('without a saved preference the page is light', async ({ page }) => {
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
    expect(await bodyBackground(page)).toBe(LIGHT_BG)
  })
})
