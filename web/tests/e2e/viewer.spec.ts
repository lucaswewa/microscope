import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { expect, type Page, test } from '@playwright/test'

test.use({ viewport: { width: 1280, height: 800 } })

// The gallery lists a deep-zoom image and a sequence of images, served from
// the DZI fixture, until scans (P38) and sequences (P41) make real ones.
const fixtures = fileURLToPath(new URL('../fixtures/dzi/', import.meta.url))
const entry = (name: string, viewer: object) => ({
  name,
  path: `fixtures/${name}`,
  card_type: 'Capture',
  created: '2026-01-01T00:00:00.000Z',
  thumbnail: null,
  viewer,
  info: [],
  download_buttons: [],
  card_actions: [],
  card_messages: [],
  delete_endpoint: `gallery/captures/${name}`,
})
const level = (n: number, tile: string) => `fixtures/sample_files/${n}/${tile}.jpeg`

/** Opens `name`'s viewer, recording the fixture's files it asks for. */
async function view(page: Page, name: string) {
  const asked: string[] = []
  await page.route('**/api/v1/gallery/list_data', (route) =>
    route.fulfill({
      json: [
        entry('mosaic', { type: 'dzi', path: 'fixtures/sample.dzi' }),
        entry('sequence', {
          type: 'directory',
          paths: [level(9, '0_0'), level(9, '1_0'), level(10, '0_0')],
          start: 0,
        }),
      ],
    }),
  )
  await page.route('**/api/v1/data/fixtures/**', (route) => {
    const file = new URL(route.request().url()).pathname.split('/data/fixtures/')[1]!
    asked.push(file)
    return route.fulfill({ path: path.join(fixtures, file) })
  })
  await page.goto('/#/gallery')
  const card = page.locator('.gallery-card', { hasText: name })
  await card.getByRole('button', { name: 'View', exact: true }).click()
  const dialog = page.getByRole('dialog')
  await expect(dialog.locator('.deep-zoom')).toHaveAttribute('data-status', 'shown')
  return { dialog, asked }
}

test('the viewer zooms into a deep-zoom image, through filters', async ({ page }) => {
  const { dialog, asked } = await view(page, 'mosaic')
  expect(asked).toContain('sample.dzi')
  await expect.poll(() => asked.some((file) => file.startsWith('sample_files/'))).toBe(true)
  // Zooming in fetches the most detailed level's tiles.
  await dialog.getByRole('button', { name: 'Zoom in' }).click()
  await dialog.getByRole('button', { name: 'Zoom in' }).click()
  await expect.poll(() => asked.some((file) => file.startsWith('sample_files/10/'))).toBe(true)

  await dialog.getByRole('slider', { name: 'Brightness' }).press('End')
  await expect(dialog.locator('.deep-zoom__view')).toHaveCSS(
    'filter',
    'brightness(2) contrast(1) saturate(1)',
  )
  await dialog.getByRole('button', { name: 'Reset' }).click()
  await expect(dialog.locator('.deep-zoom__view')).toHaveCSS(
    'filter',
    'brightness(1) contrast(1) saturate(1)',
  )
})

test('the viewer steps through a sequence with its slider', async ({ page }) => {
  const { dialog, asked } = await view(page, 'sequence')
  await expect(dialog).toContainText('1 of 3')
  await dialog.getByRole('slider', { name: 'Image' }).press('ArrowRight')
  await expect(dialog).toContainText('2 of 3')
  await expect.poll(() => asked).toContain('sample_files/9/1_0.jpeg')
})
