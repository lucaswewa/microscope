import { expect, test } from '@playwright/test'

// Checkpoint E: a timelapse with autofocus, set up and started from the
// Sequence tab, can be cancelled, and appears in the gallery with the
// image-sequence viewer. It holds the microscope, so it runs after the scan.
test.use({ viewport: { width: 1280, height: 800 } })

test('a timelapse with autofocus runs, is cancelled, and opens in the gallery', async ({
  page,
}) => {
  test.setTimeout(180_000)
  await page.goto('/#/sequence')
  const pane = page.locator('.control-pane')
  // An hour, a capture every second, each after a short autofocus.
  for (const [field, value] of [
    ['hours', '1'],
    ['minutes', '0'],
    ['seconds', '0'],
    ['Interval (s)', '1'],
    ['Autofocus range (steps)', '400'],
  ]) {
    const input = pane.getByLabel(field!, { exact: true })
    await input.fill(value!)
    await input.press('Enter')
  }
  await pane.getByRole('switch', { name: 'Autofocus before each capture' }).check()

  const name = `e2e-${Date.now()}`
  await pane.getByRole('textbox', { name: 'Name' }).fill(name)
  await pane.getByRole('button', { name: 'Start Sequence' }).click()
  await expect(pane.getByRole('button', { name: 'Cancel' })).toBeVisible()
  await expect(pane).toContainText(/Time remaining\s*0:59:\d\d/)
  await expect(page.locator('.sequence-page__latest')).toBeVisible({ timeout: 60_000 })
  await expect(pane).toContainText(/Images\s*([2-9]|\d{2,}) of/, { timeout: 60_000 })

  await pane.getByRole('button', { name: 'Cancel' }).click()
  await expect(pane.getByText('The sequence was cancelled.')).toBeVisible({ timeout: 60_000 })
  await pane.getByRole('link', { name: 'See it in the gallery' }).click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText(name)
  await expect(dialog).toContainText(/1 of ([2-9]|\d{2,})/)
  await dialog.getByRole('slider', { name: 'Image' }).press('ArrowRight')
  await expect(dialog).toContainText(/2 of/)
})
