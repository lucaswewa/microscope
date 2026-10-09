import { expect, test } from '@playwright/test'

test('the built app loads and shows its placeholder page', async ({ page }) => {
  await page.goto('/')

  await expect(page).toHaveTitle('Microscope')
  await expect(page.getByRole('heading', { name: 'Microscope' })).toBeVisible()
  // The router keeps routes in the URL's hash.
  await expect(page).toHaveURL(/\/#\/$/)
})
