import { expect, test } from '@playwright/test'

// The About tab describes the microscope the tests run against: a simulated
// one, which no launcher manages.
test('the About tab says what the microscope is', async ({ page }) => {
  await page.goto('/#/about')
  const fact = (label: string) => page.locator('dt', { hasText: label }).locator('+ dd')
  await expect(fact('Camera')).toHaveText('SimulatedCamera')
  await expect(fact('Stage')).toHaveText('SimulatedStage')
  await expect(fact('Illumination')).toHaveText('SimulatedIllumination')
  await expect(fact('Server version')).toHaveText(/^\d+\.\d+\.\d+ \(.+\)$/)
  await expect(fact('Managed by the launcher')).toHaveText('No')
  await expect(page.getByRole('button', { name: 'Flash illumination' })).toBeVisible()
})
