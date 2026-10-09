import { expect, test } from '@playwright/test'

test('the built app opens on the View page, with the navigation rail', async ({ page }) => {
  await page.goto('/')

  await expect(page).toHaveTitle('Microscope')
  // The router keeps routes in the URL's hash.
  await expect(page).toHaveURL(/\/#\/view$/)
  await expect(page.getByRole('navigation', { name: 'Main' }).getByRole('link')).toHaveCount(9)
  await expect(page.getByRole('heading', { name: 'View' })).toBeVisible()
})
