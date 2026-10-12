<script setup lang="ts">
// Settings, Stage: the stage's range of motion, measured from the image
// (P30), and recentring on it. Both ask first; measuring shows its progress
// with the stream.
import { ref, watchEffect } from 'vue'

import type { components } from '@/api/generated/schema'
import { useThing } from '@/api/things'
import ActionButton from '@/server-ui/ActionButton.vue'
import type { ActionButtonElement } from '@/server-ui/types'
import SectionHeading from '@/ui/SectionHeading.vue'

type Range = components['schemas']['stage_measure_calibrated_range_value']

const measure = useThing('stage_measure')
const range = ref<Range>()

async function load() {
  const thing = measure.value
  if (thing) range.value = await thing.read('calibrated_range')
}
watchEffect(() => void load().catch(() => {}))

function describe(axis: 'x' | 'y') {
  const { min, max, focus_shift: focus } = range.value![axis]
  const travel = `${axis}: ${max - min} steps, from ${min} to ${max}`
  return focus
    ? `${travel}; the focus moves ${focus[0]} and ${focus[1]} z steps at the ends`
    : travel
}

const test: ActionButtonElement = {
  kind: 'action_button',
  thing: 'stage_measure',
  action: 'perform_rom_test',
  label: 'Measure the range of motion',
  confirm:
    'Measure how far the stage moves? It drives x and y to both ends of their travel, watching the image, which can take several minutes. The sample must have plenty of detail all over, and be in focus.',
  progress: 'dialog',
  stream: 'camera',
  success: { message: 'The range of motion is measured.' },
  refresh: true,
}

const recentre: ActionButtonElement = {
  kind: 'action_button',
  thing: 'stage_measure',
  action: 'perform_recentre',
  label: 'Recentre',
  confirm: 'Move the stage to the middle of its measured range, and make that (0, 0, 0)?',
  success: { message: 'The stage is recentred.' },
  refresh: true,
  secondary: true,
}
</script>

<template>
  <section v-if="measure" class="stage-measure">
    <SectionHeading :level="3">Range of motion</SectionHeading>
    <ul v-if="range">
      <li>{{ describe('x') }}</li>
      <li>{{ describe('y') }}</li>
    </ul>
    <p v-else-if="range === null">It isn't measured yet.</p>
    <div class="stage-measure__actions">
      <ActionButton :button="test" @refresh="load" />
      <ActionButton v-if="range" :button="recentre" @refresh="load" />
    </div>
  </section>
</template>

<style scoped>
.stage-measure {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  align-self: stretch;
}

.stage-measure p,
.stage-measure ul {
  margin: 0;
}

.stage-measure__actions {
  display: flex;
  gap: var(--space-2);
}
</style>
