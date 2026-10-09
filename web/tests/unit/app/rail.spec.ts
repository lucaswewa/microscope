import { flushPromises, type VueWrapper } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'
import { ref } from 'vue'

import { thingAvailabilityKey } from '@/app/availability'

import { mountApp } from '../mountApp'

const labels = (wrapper: VueWrapper, group: 'top' | 'bottom') =>
  wrapper.findAll(`.rail__group--${group} .rail__label`).map((label) => label.text())

describe('the navigation rail', () => {
  let wrapper: VueWrapper | undefined

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
  })

  it('lists the workflows at the top and the rest at the bottom, in order', async () => {
    wrapper = (await mountApp('/view')).wrapper
    expect(labels(wrapper, 'top')).toEqual(['View', 'Control', 'Slide Scan', 'Sequence', 'Gallery'])
    expect(labels(wrapper, 'bottom')).toEqual(['Settings', 'Logging', 'About', 'Power'])
    expect(wrapper.get('nav').attributes('aria-label')).toBe('Main')
    expect(wrapper.findAll('.rail__item svg[aria-hidden="true"]')).toHaveLength(9)
  })

  it('marks the current destination', async () => {
    wrapper = (await mountApp('/control')).wrapper
    const active = wrapper.findAll('.rail__item--active')
    expect(active.map((item) => item.text())).toEqual(['Control'])
    expect(active[0]?.attributes('aria-current')).toBe('page')
  })

  it('marks Settings on any of its sections', async () => {
    wrapper = (await mountApp('/settings/camera')).wrapper
    expect(wrapper.findAll('.rail__item--active').map((item) => item.text())).toEqual(['Settings'])
  })

  it('hides destinations whose Things the microscope lacks', async () => {
    const offered = new Set(['camera', 'stage', 'sequence'])
    wrapper = (
      await mountApp('/view', { [thingAvailabilityKey]: (thing: string) => offered.has(thing) })
    ).wrapper
    expect(labels(wrapper, 'top')).toEqual(['View', 'Control', 'Sequence'])
    expect(labels(wrapper, 'bottom')).toEqual(['Settings', 'Logging', 'About', 'Power'])
  })
  it('updates as the offered Things change', async () => {
    const offered = ref(new Set(['camera']))
    wrapper = (
      await mountApp('/view', {
        [thingAvailabilityKey]: (thing: string) => offered.value.has(thing),
      })
    ).wrapper
    expect(labels(wrapper, 'top')).toEqual(['View', 'Control'])

    offered.value = new Set(['camera', 'gallery', 'smart_scan'])
    await flushPromises()
    expect(labels(wrapper, 'top')).toEqual(['View', 'Control', 'Slide Scan', 'Gallery'])
  })
})
