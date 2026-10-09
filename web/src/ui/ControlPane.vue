<script setup lang="ts">
// The fixed-width column of controls beside a page's main view: narrow
// (Control, Settings) or wide (Slide Scan, Sequence).
withDefaults(defineProps<{ width?: 'narrow' | 'wide' }>(), { width: 'narrow' })
</script>

<template>
  <aside class="control-pane" :class="`control-pane--${width}`">
    <slot />
  </aside>
</template>

<style scoped>
.control-pane {
  flex: none;
  height: 100%;
  padding: var(--space-3);
  overflow: hidden auto;
  border-right: var(--border-width) solid var(--color-border);
  background: var(--color-surface-panel);
  scrollbar-gutter: stable;
}

.control-pane--narrow {
  width: var(--pane-width-narrow);
}

.control-pane--wide {
  width: var(--pane-width-wide);
}
</style>
