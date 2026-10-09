import add from '@material-symbols/svg-400/outlined/add.svg?raw'
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import AppButton from '@/ui/AppButton.vue'
import IconButton from '@/ui/IconButton.vue'

describe('AppButton', () => {
  it('is a plain button unless it is told to submit', () => {
    expect(mount(AppButton).get('button').attributes('type')).toBe('button')
    const submit = mount(AppButton, { attrs: { type: 'submit' } })
    expect(submit.get('button').attributes('type')).toBe('submit')
  })

  it('shows its variant, size, label and icon', () => {
    const wrapper = mount(AppButton, {
      props: { variant: 'danger', size: 'sm', icon: add },
      slots: { default: 'Delete' },
    })
    expect(wrapper.classes()).toEqual(
      expect.arrayContaining(['app-button--danger', 'app-button--sm']),
    )
    expect(wrapper.text()).toBe('Delete')
    expect(wrapper.get('svg').attributes('aria-hidden')).toBe('true')
  })

  it('is clicked unless disabled', async () => {
    const wrapper = mount(AppButton)
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toHaveLength(1)
    const disabled = mount(AppButton, { attrs: { disabled: true } })
    expect(disabled.get('button').element.disabled).toBe(true)
  })
})

describe('IconButton', () => {
  it('is named by its label, which is also its tooltip', () => {
    const button = mount(IconButton, { props: { icon: add, label: 'Add' } }).get('button')
    expect(button.attributes('aria-label')).toBe('Add')
    expect(button.attributes('title')).toBe('Add')
    expect(button.text()).toBe('')
  })
})
