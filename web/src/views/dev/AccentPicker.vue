<script setup lang="ts">
// The dev pages' Accent picker: another accent colour for the whole app, to
// try it (src/theme/accents.ts). It's kept in this browser, in development
// builds only.
import { ref } from 'vue'

import { ACCENTS, applyAccent, readAccent, writeAccent, type AccentId } from '@/theme/accents'

const emit = defineEmits<{ change: [id: AccentId] }>()
const chosen = ref(readAccent())

function choose(id: AccentId) {
  chosen.value = id
  writeAccent(id)
  applyAccent(id)
  emit('change', id)
}
</script>

<template>
  <fieldset class="accent-picker">
    <legend>Accent</legend>
    <label v-for="accent in ACCENTS" :key="accent.id" class="accent-picker__choice">
      <input
        type="radio"
        name="accent"
        :value="accent.id"
        :checked="chosen === accent.id"
        @change="choose(accent.id)"
      />
      <span class="accent-picker__swatch" :style="{ background: accent.hex }" />
      {{ accent.label }}
    </label>
  </fieldset>
</template>

<style scoped>
.accent-picker {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-4);
  align-items: center;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.accent-picker__choice {
  display: inline-flex;
  gap: var(--space-2);
  align-items: center;
}

.accent-picker__swatch {
  width: var(--space-4);
  height: var(--space-4);
  border: var(--border-width) solid var(--color-border-strong);
  border-radius: var(--radius-sm);
}
</style>
