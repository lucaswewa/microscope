import { expect, test } from '@playwright/test'

import { open, SHOTS, STEADY } from './pages'

// Baselines for every tab and Settings section, and a few key states, in
// both themes, and the main layouts at 1920 × 1080 too (ADR-0034). Update
// them with `npx playwright test tests/visual --update-snapshots` when the
// look changes on purpose, and review the new images in the pull request.
test.use(STEADY)

for (const colorScheme of ['light', 'dark'] as const) {
  for (const [width, height] of [
    [1280, 800],
    [1920, 1080],
  ] as const) {
    test.describe(`${colorScheme}, ${width} × ${height}`, () => {
      test.use({ colorScheme, viewport: { width, height } })
      const size = width === 1280 ? '' : `-${width}`
      for (const shot of SHOTS.filter((shot) => width === 1280 || shot.large)) {
        test(shot.name, async ({ page }) => {
          await open(page, shot)
          // Off whatever was clicked, so no hover shows.
          await page.mouse.move(width - 1, height - 1)
          await expect(page).toHaveScreenshot(`${shot.name}-${colorScheme}${size}.png`)
        })
      }
    })
  }
}
