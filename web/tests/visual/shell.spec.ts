import { expect, test } from '@playwright/test'

// Baselines for the shell in both themes (ADR-0012). Update them with
// `npx playwright test tests/visual --update-snapshots` when the look changes
// on purpose, and review the new images in the pull request.
for (const colorScheme of ['light', 'dark'] as const) {
  test.describe(`the shell in the ${colorScheme} theme`, () => {
    test.use({ colorScheme, viewport: { width: 1280, height: 800 } })

    for (const route of ['control', 'settings/camera']) {
      test(`on ${route}`, async ({ page }) => {
        // A still picture: the stream off, and the stage at a fixed place,
        // whatever other tests do with it meanwhile.
        await page.addInitScript(() => localStorage.setItem('microscope.stream-disabled', 'true'))
        await page.route('**/api/v1/stage/position', (r) =>
          r.fulfill({ json: { x: 0, y: 0, z: 0 } }),
        )
        await page.route('**/api/v1/stage/moving', (r) => r.fulfill({ json: false }))
        await page.goto(`/#/${route}`)
        await expect(page.locator('.connection-indicator')).toHaveAttribute(
          'data-state',
          'connected',
        )
        // The indicator names the machine the tests run on.
        await expect(page).toHaveScreenshot(`${route.replace('/', '-')}-${colorScheme}.png`, {
          mask: [page.locator('.connection-indicator__label')],
        })
      })
    }
  })
}
