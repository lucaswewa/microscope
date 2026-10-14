import { expect, test } from '@playwright/test'

// The Logging tab shows the server's log: an action's error in its
// invocation's group, and today's log file to download.
test('the Logging tab shows an action’s error, and downloads the log file', async ({
  page,
  request,
}) => {
  // download_zip doesn't hold the microscope, so it runs beside the other tests.
  const name = `missing-${Date.now()}`
  const failed = await request.post('/api/v1/sequence/download_zip', { data: { name } })
  expect(failed.status()).toBe(201)
  await expect
    .poll(async () => (await request.get((await failed.json()).href)).json(), {
      intervals: [200],
    })
    .toMatchObject({ status: 'error' })

  await page.goto('/#/logging')
  await page.locator('.logging-page__level [role="combobox"]').click()
  await page.getByRole('option', { name: 'ERROR' }).click()
  // Its group, found by its records' text, though they're hidden until it opens.
  const group = page.locator('details.log-group', { hasText: name })
  await expect(group.locator('.log-group__title')).toHaveText('sequence.download_zip')
  await expect(group.locator('summary .log-level')).toHaveText('ERROR')
  await group.locator('summary').click()
  await expect(group.getByText(`there's no sequence \`${name}\``)).toBeVisible()

  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: 'Download log file' }).click()
  expect((await download).suggestedFilename()).toMatch(/^microscope\.\d{4}-\d{2}-\d{2}\.log$/)
})
