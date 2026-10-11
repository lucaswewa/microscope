<script setup lang="ts">
// A number of seconds, entered as hours, minutes and seconds. Each part
// commits as a NumberField does, and commits the new total.
import { computed } from 'vue'

import NumberField from '@/ui/NumberField.vue'

import { joinDuration, splitDuration, type Duration } from './duration'

const model = defineModel<number>({ required: true })
const parts = computed(() => splitDuration(model.value))

const fields = [
  { part: 'hours', unit: 'h', max: undefined, step: 1 },
  { part: 'minutes', unit: 'min', max: 59, step: 1 },
  { part: 'seconds', unit: 's', max: 59.999999, step: undefined },
] as const

function set(part: keyof Duration, value: number) {
  model.value = joinDuration({ ...parts.value, [part]: value })
}
</script>

<template>
  <div class="duration-field">
    <label v-for="field in fields" :key="field.part" class="duration-field__part">
      <NumberField
        size="sm"
        :min="0"
        :max="field.max"
        :step="field.step"
        :aria-label="field.part"
        :model-value="parts[field.part]"
        @update:model-value="set(field.part, $event)"
      />
      {{ field.unit }}
    </label>
  </div>
</template>

<style scoped>
.duration-field {
  display: flex;
  gap: var(--space-2);
}

.duration-field__part {
  display: flex;
  gap: var(--space-1);
  align-items: center;
}
</style>
