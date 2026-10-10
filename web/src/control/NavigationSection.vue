<script setup lang="ts">
// The navigation preferences: how far a step of the d-pad, the keys or the
// wheel goes, and which axes move the other way. Kept in this browser.
import AppCheckbox from '@/ui/AppCheckbox.vue'
import FormField from '@/ui/FormField.vue'
import NumberField from '@/ui/NumberField.vue'

import { useNavigationPreferences } from './navigation'

const AXES = ['x', 'y', 'z'] as const

const preferences = useNavigationPreferences()
</script>

<template>
  <div class="navigation-section">
    <p class="navigation-section__note">Steps per press, or per notch of the wheel for z.</p>
    <FormField v-for="axis in AXES" :key="axis" :label="`${axis} step`">
      <NumberField v-model="preferences.steps[axis]" size="sm" :min="1" :step="1" />
    </FormField>
    <AppCheckbox
      v-for="axis in AXES"
      :key="`invert-${axis}`"
      v-model="preferences.invert[axis]"
      :label="`Invert ${axis}`"
    />
  </div>
</template>

<style scoped>
.navigation-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.navigation-section__note {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}
</style>
