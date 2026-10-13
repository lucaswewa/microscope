import { expect, test } from '@playwright/test'

test.use({ viewport: { width: 1280, height: 800 } })

test('a capture saved to the gallery is shown as a card, and can be deleted', async ({ page }) => {
  await page.goto('/#/control')
  await expect(page.locator('.live-image')).toHaveAttribute('data-status', 'live')
  await page.getByRole('button', { name: 'Capture' }).click()
  // A capture waits for any other action holding the microscope's lock.
  await expect(page.getByText('Saved to the gallery.').first()).toBeVisible({ timeout: 20_000 })

  await page.getByRole('link', { name: 'Gallery' }).click()
  // The newest first, with its thumbnail from the data folder.
  const card = page.locator('.gallery-card').first()
  const name = (await card.locator('.gallery-card__title').textContent())!
  expect(name).toMatch(/^capture-/)
  const thumbnail = card.locator('.gallery-card__thumbnail img')
  await expect(thumbnail).toHaveJSProperty('complete', true)
  expect(await thumbnail.evaluate((image: HTMLImageElement) => image.naturalWidth)).toBe(320)
  await expect(card.locator('.gallery-card__info dd').first()).toHaveText('1640 × 1232 px')

  await card.getByRole('button', { name: 'Delete', exact: true }).click()
  await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click()
  await expect(page.locator('.gallery-card__title', { hasText: name })).toHaveCount(0)
})
