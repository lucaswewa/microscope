<script setup lang="ts">
// The dev pages' Accent picker: every accent colour, to try it across the
// app (ADR-0047). It's the choice Settings, Display makes from nine of them.
import { ACCENTS, useAccentStore, type AccentId } from '@/theme/accents'

const emit = defineEmits<{ change: [id: AccentId] }>()
const accent = useAccentStore()

function choose(id: AccentId) {
  accent.choose(id)
  emit('change', id)
}
</script>

<template>
  <fieldset class="accent-picker">
    <legend>Accent</legend>
    <label v-for="choice in ACCENTS" :key="choice.id" class="accent-picker__choice">
      <input
        type="radio"
        name="accent"
        :value="choice.id"
        :checked="accent.accent === choice.id"
        @change="choose(choice.id)"
      />
      <span class="accent-picker__swatch" :style="{ background: choice.hex }" />
      {{ choice.label }}
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
