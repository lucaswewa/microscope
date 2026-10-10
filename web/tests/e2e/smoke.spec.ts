import { expect, test } from '@playwright/test'

test('the built app opens on the View page, connected, with the navigation rail', async ({
  page,
}) => {
  await page.goto('/')

  // The router keeps routes in the URL's hash.
  await expect(page).toHaveURL(/\/#\/view$/)
  await expect(page.getByRole('heading', { name: 'View' })).toBeAttached()
  await expect(page.getByRole('img', { name: 'Live image' })).toBeVisible()
  // Connected to the test server, which names the window after its host.
  await expect(page.locator('.connection-indicator')).toHaveAttribute('data-state', 'connected')
  await expect(page).toHaveTitle(/^.+ – Microscope$/)
  // The tabs whose Things that server has, and the connection indicator.
  const rail = page.getByRole('navigation', { name: 'Main' })
  await expect(rail.getByRole('link')).toHaveText([
    'View',
    'Control',
    'Settings',
    'Logging',
    'About',
    'Power',
    /\S/,
  ])
})
