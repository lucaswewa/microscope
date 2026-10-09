<script setup lang="ts">
// A square button showing only an icon, such as the stage's arrow pad. Its
// `label` names it for assistive technology and is its tooltip.
import AppButton from './AppButton.vue'

withDefaults(
  defineProps<{
    /** The icon, as raw SVG text (see AppIcon). */
    icon: string
    label: string
    variant?: 'default' | 'primary' | 'danger' | 'ghost'
    size?: 'sm' | 'md'
  }>(),
  { variant: 'default', size: 'md' },
)
</script>

<template>
  <AppButton
    class="icon-button"
    :class="`icon-button--${size}`"
    :variant="variant"
    :size="size"
    :icon="icon"
    :aria-label="label"
    :title="label"
  />
</template>

<style scoped>
.icon-button {
  width: var(--control-height-md);
  padding: 0;
}

.icon-button--sm {
  width: var(--control-height-sm);
}
</style>
