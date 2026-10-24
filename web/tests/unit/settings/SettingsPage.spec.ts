import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'

import type { ThingDescription } from '@/api/wot/td'
import { useConnectionStore } from '@/connection/store'
import { hostKey } from '@/host'
import { createBrowserHost } from '@/host/browser'
import { STREAM_DISABLED_KEY } from '@/live/preferences'
import { ACCENT_STORAGE_KEY } from '@/theme/accents'
import { THEME_STORAGE_KEY } from '@/theme/store'

import { eventStream, fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

/** A Thing's description, with readable, observable properties and actions. */
function thing(name: string, properties: string[], actions: string[] = []): ThingDescription {
  const href = (affordance: string) => `/api/v1/${name}/${affordance}`
  return {
    title: name,
    base: 'http://localhost/',
    properties: Object.fromEntries(
      properties.map((p) => [
        p,
        {
          observable: true,
          forms: [
            { href: href(p), op: ['readproperty'] },
            { href: href(p), op: ['observeproperty'], subprotocol: 'sse' },
          ],
        },
      ]),
    ),
    actions: Object.fromEntries(
      actions.map((a) => [a, { forms: [{ href: href(a), op: ['invokeaction'] }] }]),
    ),
  }
}

const descriptions = {
  system: thing('system', ['hostname']),
  camera: thing('camera', ['background_detector_name'], ['full_auto_calibrate']),
  stage: thing('stage', ['axis_inverted'], ['invert_axis_direction']),
  camera_stage_mapping: thing(
    'camera_stage_mapping',
    [
      'image_to_stage_displacement_matrix',
      'image_resolution',
      'last_cal_timestamp',
      'last_calibration',
    ],
    ['calibrate_xy'],
  ),
  bg_channel_deviations_luv: thing('bg_channel_deviations_luv', []),
  stage_measure: thing(
    'stage_measure',
    ['calibrated_range'],
    ['perform_rom_test', 'perform_recentre'],
  ),
}

let values: Record<string, unknown>
let received: Received[]
let wrapper: VueWrapper | undefined
let router: Router
let fullscreen: boolean
const saveFile = vi.fn(async () => {})
const setFullscreen = vi.fn(async (on: boolean) => {
  fullscreen = on
})

beforeEach(() => {
  vi.useFakeTimers()
  window.localStorage.clear()
  fullscreen = false
  values = {
    'system/hostname': 'lab-pc',
    'camera/background_detector_name': 'bg_channel_deviations_luv',
    'stage/axis_inverted': { x: false, y: false, z: false },
    'camera_stage_mapping/image_to_stage_displacement_matrix': [
      [1.12, -0.0001],
      [0, 1],
    ],
    'camera_stage_mapping/image_resolution': [820, 616],
    'camera_stage_mapping/last_cal_timestamp': '2026-10-11T07-35-12Z',
    'camera_stage_mapping/last_calibration': { timestamp: '2026-10-11T07-35-12Z', moves: [] },
    'stage_measure/calibrated_range': null,
  }
  const panel = (text: string) => [{ kind: 'text', text: [{ text }] }]
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': descriptions,
    ...Object.fromEntries(
      Object.keys(values).map((key) => [
        `GET /api/v1/${key}`,
        (request: Received) =>
          request.headers.get('Accept') === 'text/event-stream'
            ? eventStream([], request, { open: true })
            : json(values[key]),
      ]),
    ),
    'GET /api/v1/camera/primary_calibration_actions': panel('Primary actions'),
    'GET /api/v1/camera/secondary_calibration_actions': panel('Secondary actions'),
    'GET /api/v1/camera/manual_camera_settings': panel('Manual settings'),
    'GET /api/v1/bg_channel_deviations_luv/settings_ui': panel('Detector settings'),
    'POST /api/v1/stage/invert_axis_direction': () =>
      json(
        { id: 'i', status: 'completed', href: 'http://localhost/api/v1/action_invocations/i' },
        201,
      ),
    'POST /api/v1/stage_measure/perform_recentre': () =>
      json(
        { id: 'r', status: 'completed', href: 'http://localhost/api/v1/action_invocations/r' },
        201,
      ),
  })
  received = server.received
  vi.stubGlobal('fetch', server.fetch)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.clearAllMocks()
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

async function open(path: string) {
  const fullscreenControl = {
    active: () => fullscreen,
    set: setFullscreen,
    watch: () => () => {},
  }
  const host = { ...createBrowserHost(), saveFile, fullscreen: fullscreenControl }
  ;({ wrapper, router } = await mountApp(path, { [hostKey]: host }))
  await useConnectionStore().connect({ kind: 'local' })
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

const heading = () => wrapper!.get('.settings-page__section h2').text()
const button = (label: string) => wrapper!.findAll('button').find((b) => b.text() === label)!

describe('the Settings tab', () => {
  it('lists its sections, and shows the one in the address, or Display', async () => {
    await open('/settings')
    expect(heading()).toBe('Display')
    expect(wrapper!.get('.settings-page__nav [aria-current="page"]').text()).toBe('Display')
    const links = wrapper!.findAll('.settings-page__link').map((link) => link.text())
    expect(links).toEqual([
      'Display',
      'Stage control',
      'Connection',
      'Camera',
      'Stage',
      'Camera–stage mapping',
    ])
    await wrapper!.findAll('.settings-page__link')[4]!.trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.fullPath).toBe('/settings/stage')
    expect(heading()).toBe('Stage')
    expect(wrapper!.get('.settings-page__nav [aria-current="page"]').text()).toBe('Stage')
    await router.push('/settings/nonsense')
    await flushPromises()
    expect(heading()).toBe('Display')
  })

  it('sets the theme, the accent, full screen and the live image in Display', async () => {
    await open('/settings/display')
    const [theme, accent] = wrapper!.findAll('[role="combobox"]')
    const choose = async (label: string) => {
      const options = [...document.querySelectorAll<HTMLElement>('[role="option"]')]
      const option = options.find((option) => option.textContent?.trim() === label)!
      option.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
      await flushPromises()
    }
    await theme!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    await choose('Dark')
    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark')
    // Nine accents in three groups, each with its colour (ADR-0047).
    expect(accent!.text()).toBe('Magenta (default)')
    await accent!.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    const groups = [...document.querySelectorAll('[role="group"]')].map((group) => [
      group.querySelector('.app-select__group')?.textContent?.trim(),
      group.querySelectorAll('[role="option"]').length,
    ])
    expect(groups).toEqual([
      ['Magentas', 3],
      ['Greens', 3],
      ['Blues', 3],
    ])
    expect(document.querySelectorAll('[role="option"] .app-select__swatch')).toHaveLength(9)
    await choose('Mantis')
    expect(window.localStorage.getItem(ACCENT_STORAGE_KEY)).toBe('mantis')
    expect(document.getElementById('accent-tokens')?.textContent).toContain('#74c365')
    const [screen, stream] = wrapper!.findAll('input[role="switch"]')
    await screen!.setValue(true)
    expect(setFullscreen).toHaveBeenCalledWith(true)
    await stream!.setValue(true)
    expect(window.localStorage.getItem(STREAM_DISABLED_KEY)).toBe('true')
  })

  it('offers the Connect screen as Connection, and stays in Settings', async () => {
    await open('/settings/connection')
    expect(wrapper!.text()).toContain('This server')
    expect(wrapper!.text()).toContain('Connected to lab-pc.')
    await button('Connect').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.fullPath).toBe('/settings/connection')
  })

  it('shows the camera’s and its detector’s panels in Camera', async () => {
    await open('/settings/camera')
    for (const text of ['Primary actions', 'Secondary actions', 'Manual settings']) {
      expect(wrapper!.text()).toContain(text)
    }
    expect(wrapper!.text()).toContain('Detector settings')
    expect(wrapper!.find('[aria-label="Live image"]').exists()).toBe(true)
  })

  it('says whether z is inverted, and inverts it, in Stage', async () => {
    await open('/settings/stage')
    expect(wrapper!.text()).toContain("The z axis isn't inverted.")
    await button('Invert z').trigger('click')
    await flushPromises()
    const posted = received.filter((r) => r.url.pathname.endsWith('/invert_axis_direction'))
    expect(posted.map((r) => JSON.parse(r.body ?? 'null'))).toEqual([{ axis: 'z' }])
    expect(wrapper!.text()).toContain("It isn't measured yet.")
    expect(button('Measure the range of motion').exists()).toBe(true)
    expect(button('Recentre')).toBeUndefined()
  })

  it('shows the measured range of motion, and recentres on it, in Stage', async () => {
    values['stage_measure/calibrated_range'] = {
      x: { min: -2500, max: 3500, focus_shift: [185, -197] },
      y: { min: -3000, max: 2000, focus_shift: null },
    }
    await open('/settings/stage')
    const items = wrapper!.findAll('.stage-measure li').map((li) => li.text())
    expect(items).toEqual([
      'x: 6000 steps, from -2500 to 3500; the focus moves 185 and -197 z steps at the ends',
      'y: 5000 steps, from -3000 to 2000',
    ])
    await button('Recentre').trigger('click')
    await flushPromises()
    const question = document.querySelector<HTMLElement>('[role="alertdialog"]')!
    expect(question.textContent).toContain('Move the stage to the middle of its measured range')
    const confirm = [...question.querySelectorAll('button')].find(
      (b) => b.textContent!.trim() === 'Recentre',
    )!
    confirm.click()
    await flushPromises()
    expect(received.some((r) => r.url.pathname.endsWith('/perform_recentre'))).toBe(true)
  })

  it('shows the mapping’s matrix and scale, and saves its data', async () => {
    await open('/settings/camera-stage-mapping')
    const cells = wrapper!.findAll('.mapping-settings__matrix td').map((td) => td.text())
    expect(cells).toEqual(['1.120', '0.000', '0.000', '1.000'])
    expect(wrapper!.text()).toContain('1.058 steps per pixel')
    expect(wrapper!.text()).toContain('918 × 616 steps')
    expect(button('Calibrate').exists()).toBe(true)
    await button('Download the calibration data').trigger('click')
    await flushPromises()
    const [[name, file]] = saveFile.mock.calls as unknown as [string, Blob][]
    expect(name).toBe('camera_stage_mapping_2026-10-11T07-35-12Z.json')
    expect(JSON.parse(await file.text())).toEqual(values['camera_stage_mapping/last_calibration'])
  })

  it('says when the mapping isn’t calibrated', async () => {
    values['camera_stage_mapping/image_to_stage_displacement_matrix'] = null
    await open('/settings/camera-stage-mapping')
    expect(wrapper!.text()).toContain("It isn't calibrated yet.")
    expect(wrapper!.find('.mapping-settings__matrix').exists()).toBe(false)
  })
})
