<script setup lang="ts" generic="T extends string | number">
// A drop-down list of options, on Reka UI's Select (ADR-0013), which handles
// the keyboard, typeahead, focus and ARIA. `v-model` is the chosen option's
// value; until one is chosen, `placeholder` shows. In a FormField it takes
// the field's id, description and error state.
import check from '@material-symbols/svg-400/outlined/check.svg?raw'
import chevron from '@material-symbols/svg-400/outlined/keyboard_arrow_down.svg?raw'
import {
  SelectContent,
  SelectIcon,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPortal,
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectViewport,
} from 'reka-ui'
import { ref } from 'vue'

import AppIcon from './AppIcon.vue'
import { useFieldControl } from './field'

const model = defineModel<T>()
const props = withDefaults(
  defineProps<{
    options: readonly { value: T; label: string; disabled?: boolean }[]
    placeholder?: string
    size?: 'sm' | 'md'
    invalid?: boolean
    disabled?: boolean
  }>(),
  { placeholder: undefined, size: 'md', invalid: false, disabled: false },
)

const field = useFieldControl(() => props.invalid)

// The list is moved to <body> so no scrolling pane clips it. It takes the
// theme of the part of the page its trigger is in (ADR-0011 lets any element
// set the theme of its contents).
const root = ref<HTMLElement>()
const theme = ref<string>()
function onOpen(open: boolean) {
  if (open)
    theme.value = root.value?.closest('[data-theme]')?.getAttribute('data-theme') ?? undefined
}
</script>

<template>
  <span ref="root" class="app-select">
    <SelectRoot v-model="model" :disabled="disabled" @update:open="onOpen">
      <SelectTrigger
        :id="field.id"
        class="ui-input app-select__trigger"
        :class="`ui-input--${size}`"
        :aria-invalid="field.invalid.value || undefined"
        :aria-describedby="field.describedBy.value"
      >
        <SelectValue :placeholder="placeholder" class="app-select__value" />
        <SelectIcon as-child>
          <AppIcon :svg="chevron" size="var(--icon-size-sm)" />
        </SelectIcon>
      </SelectTrigger>
      <SelectPortal>
        <SelectContent
          position="popper"
          :side-offset="2"
          :data-theme="theme"
          class="app-select__list"
        >
          <SelectViewport>
            <SelectItem
              v-for="option in options"
              :key="option.value"
              :value="option.value"
              :disabled="option.disabled"
              class="app-select__option"
            >
              <SelectItemText>{{ option.label }}</SelectItemText>
              <SelectItemIndicator as-child>
                <AppIcon :svg="check" size="var(--icon-size-sm)" />
              </SelectItemIndicator>
            </SelectItem>
          </SelectViewport>
        </SelectContent>
      </SelectPortal>
    </SelectRoot>
  </span>
</template>

<style src="./input.css"></style>

<style>
/* Not scoped: the list is rendered at the end of <body>. */
.app-select {
  display: block;
}

.app-select__trigger {
  gap: var(--space-1);
  justify-content: space-between;
  text-align: left;
  cursor: pointer;
}

.app-select__trigger[data-placeholder] .app-select__value {
  color: var(--color-text-muted);
}

.app-select__value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-select__list {
  z-index: var(--z-dropdown);
  min-width: var(--reka-select-trigger-width);
  max-height: var(--reka-select-content-available-height);
  padding: var(--space-1) 0;
  overflow: auto;
  border: var(--border-width) solid var(--color-border-strong);
  background: var(--color-surface-raised);
  box-shadow: var(--shadow-md);
  color: var(--color-text);
  font-family: var(--font-family-sans);
  font-size: var(--font-size-md);
}

.app-select__option {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  justify-content: space-between;
  min-height: var(--control-height-sm);
  padding: 0 var(--space-2);
  outline: none;
  cursor: pointer;
  user-select: none;
}

.app-select__option[data-highlighted] {
  background: var(--color-accent-subtle);
}

.app-select__option[data-state='checked'] {
  color: var(--color-accent-text);
}

.app-select__option[data-disabled] {
  color: var(--color-text-disabled);
  cursor: not-allowed;
}
</style>
