import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { WotClient } from '@/api/wot/client'
import type { ThingDescription } from '@/api/wot/td'
import { calibratable, needingCalibration } from '@/calibration/tasks'
import { useConnectionStore } from '@/connection/store'

import { fakeServer, json, type Received } from '../api/fakeServer'
import { mountApp } from '../mountApp'

/** A Thing's description, with readable properties and actions. */
function thing(name: string, properties: string[], actions: string[] = []): ThingDescription {
  const forms = (affordance: string, op: string) => [
    { href: `/api/v1/${name}/${affordance}`, op: [op] },
  ]
  return {
    title: name,
    base: 'http://localhost/',
    properties: Object.fromEntries(properties.map((p) => [p, { forms: forms(p, 'readproperty') }])),
    actions: Object.fromEntries(actions.map((a) => [a, { forms: forms(a, 'invokeaction') }])),
  }
}

const descriptions = {
  system: thing('system', ['hostname']),
  stage: thing('stage', ['calibration_required', 'can_calibrate'], ['calibrate_z_direction']),
  camera: thing('camera', ['calibration_required'], ['full_auto_calibrate']),
  camera_stage_mapping: thing(
    'camera_stage_mapping',
    ['calibration_required', 'can_calibrate'],
    ['calibrate_xy'],
  ),
  // A Thing the wizard doesn't know how to calibrate.
  illumination: thing('illumination', ['calibration_required']),
}

let values: Record<string, unknown>
let received: Received[]
let wrapper: VueWrapper | undefined

beforeEach(() => {
  vi.useFakeTimers()
  values = {
    'system/hostname': 'lab-pc',
    'stage/calibration_required': true,
    'stage/can_calibrate': true,
    'camera/calibration_required': false,
    'camera_stage_mapping/calibration_required': true,
    'camera_stage_mapping/can_calibrate': true,
    'illumination/calibration_required': true,
  }
  const server = fakeServer({
    'GET /api/v1/thing_descriptions/': descriptions,
    ...Object.fromEntries(
      Object.keys(values).map((key) => [`GET /api/v1/${key}`, () => json(values[key])]),
    ),
    'GET /api/v1/stage/z_direction_check': [
      { kind: 'text', text: [{ text: 'Check the z direction.' }] },
      {
        kind: 'action_button',
        thing: 'stage',
        action: 'calibrate_z_direction',
        label: 'Confirm the z direction',
        input: { z_inverted: false },
        refresh: true,
      },
    ],
    'POST /api/v1/stage/calibrate_z_direction': () => {
      values['stage/calibration_required'] = false
      const href = 'http://localhost/api/v1/action_invocations/i'
      return json({ id: 'i', status: 'completed', href }, 201)
    },
  })
  received = server.received
  vi.stubGlobal('fetch', server.fetch)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

async function settle() {
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

async function open(path: string) {
  ;({ wrapper } = await mountApp(path))
  await useConnectionStore().connect({ kind: 'local' })
  await settle()
}

const wizard = () => document.querySelector<HTMLElement>('[role="dialog"]')
const title = () => wizard()?.querySelector('h2')?.textContent?.trim()
/** The wizard's Back and Next buttons. */
const footer = () =>
  [...wizard()!.querySelectorAll('footer button')].map((b) => b.textContent!.trim())

/** Presses the button that says `label`, or failing that, is labelled it. */
async function press(label: string, within: HTMLElement | null = wizard()) {
  const buttons = [...within!.querySelectorAll('button')]
  const button =
    buttons.find((b) => b.textContent!.trim() === label) ??
    buttons.find((b) => b.getAttribute('aria-label') === label)
  if (!button) throw new Error(`No ${label} button`)
  button.click()
  await settle()
}

describe('the calibration tasks', () => {
  const client = new WotClient({ baseUrl: 'http://localhost/api/v1/' })

  it("are for the Things the wizard knows, with calibration_required, unless they can't be calibrated", async () => {
    expect(await calibratable(client, descriptions)).toEqual([
      'stage',
      'camera',
      'camera_stage_mapping',
    ])
    values['camera_stage_mapping/can_calibrate'] = false
    const cameraWithout = { ...descriptions, camera: thing('camera', []) }
    expect(await calibratable(client, cameraWithout)).toEqual(['stage'])
  })

  it('are needed when their Things say so', async () => {
    const things = ['stage', 'camera', 'camera_stage_mapping']
    expect(await needingCalibration(client, descriptions, things)).toEqual([
      'stage',
      'camera_stage_mapping',
    ])
  })
})

describe('the calibration wizard', () => {
  it('opens on connecting, and goes through what needs calibrating', async () => {
    await open('/view')
    expect(title()).toBe('Welcome to your microscope')
    expect(footer()).toEqual(['Next'])

    await press('Next')
    expect(title()).toBe('Calibration: Stage')
    expect(wizard()!.textContent).toContain('Check the z direction.')
    expect(footer()).toEqual(['Back', 'Skip'])
    await press('Confirm the z direction')
    const confirmed = received.find((r) => r.url.pathname.endsWith('/calibrate_z_direction'))
    expect(JSON.parse(confirmed!.body!)).toEqual({ z_inverted: false })
    expect(footer()).toEqual(['Back', 'Next'])

    // The camera needs nothing, so the mapping is next.
    await press('Next')
    expect(title()).toBe('Calibration: Camera–stage mapping')
    expect(footer()).toEqual(['Back', 'Next'])
    await press('Next')
    expect(footer()).toEqual(['Back', 'Skip'])
    await press('Skip')
    expect(title()).toBe('Calibration')
    expect(footer()).toEqual(['Back', 'Finish'])
    await press('Back')
    expect(footer()).toEqual(['Back', 'Skip'])
    await press('Skip')
    await press('Finish')
    expect(wizard()).toBeNull()
  })

  it("doesn't open with nothing to calibrate, nor again on reconnecting", async () => {
    values['stage/calibration_required'] = false
    values['camera_stage_mapping/calibration_required'] = false
    await open('/view')
    expect(wizard()).toBeNull()

    values['stage/calibration_required'] = true
    const connection = useConnectionStore()
    connection.state = 'lost'
    await settle()
    connection.state = 'connected'
    await settle()
    expect(wizard()).toBeNull()

    await connection.connect({ kind: 'local' })
    await settle()
    expect(title()).toBe('Welcome to your microscope')
  })

  it('asks before closing early', async () => {
    await open('/view')
    wizard()!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }),
    )
    await settle()
    const question = () => document.querySelector<HTMLElement>('[role="alertdialog"]')
    expect(question()!.textContent).toContain('Close the calibration wizard?')
    await press('Keep calibrating', question())
    expect(title()).toBe('Welcome to your microscope')

    await press('Close')
    await press('Close the wizard', question())
    expect(wizard()).toBeNull()
  })

  it('opens from Settings with every task, without the welcome', async () => {
    values['stage/calibration_required'] = false
    values['camera_stage_mapping/calibration_required'] = false
    await open('/settings')
    expect(wizard()).toBeNull()
    await press('Calibration wizard', wrapper!.element as HTMLElement)
    const titles = [title()]
    while (footer().at(-1) !== 'Finish') {
      await press(footer().at(-1)!)
      titles.push(title())
    }
    expect(titles).toEqual([
      'Calibration: Stage',
      ...Array(4).fill('Calibration: Camera'),
      ...Array(2).fill('Calibration: Camera–stage mapping'),
      'Calibration',
    ])
  })
})
