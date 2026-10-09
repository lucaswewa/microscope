import { expect, test } from '@playwright/test'

// Baselines for the shell in both themes (ADR-0012). Update them with
// `npx playwright test tests/visual --update-snapshots` when the look changes
// on purpose, and review the new images in the pull request.
for (const colorScheme of ['light', 'dark'] as const) {
  test.describe(`the shell in the ${colorScheme} theme`, () => {
    test.use({ colorScheme, viewport: { width: 1280, height: 800 } })

    for (const route of ['control', 'settings/camera']) {
      test(`on ${route}`, async ({ page }) => {
        await page.goto(`/#/${route}`)
        await expect(page.getByRole('navigation', { name: 'Main' })).toBeVisible()
        await expect(page).toHaveScreenshot(`${route.replace('/', '-')}-${colorScheme}.png`)
      })
    }
  })
}
