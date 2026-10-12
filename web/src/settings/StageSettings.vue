<script setup lang="ts">
// Settings, Stage: which way the z axis counts, and a button that turns it
// round; then the range of motion, measured from the image (P30).
import { ref, watchEffect } from 'vue'

import { useThing } from '@/api/things'
import ActionButton from '@/server-ui/ActionButton.vue'
import type { ActionButtonElement } from '@/server-ui/types'

import StageMeasure from './StageMeasure.vue'

const stage = useThing('stage')
const inverted = ref<boolean>()

watchEffect((onCleanup) => {
  const thing = stage.value
  if (!thing) return
  const show = (axes: { z: boolean }) => (inverted.value = axes.z)
  void thing.read('axis_inverted').then(show, () => {})
  onCleanup(thing.observe('axis_inverted', show))
})

const invert: ActionButtonElement = {
  kind: 'action_button',
  thing: 'stage',
  action: 'invert_axis_direction',
  label: 'Invert z',
  input: { axis: 'z' },
  secondary: true,
}
</script>

<template>
  <div class="stage-settings">
    <p v-if="inverted !== undefined">
      {{
        inverted
          ? 'The z axis is inverted: it counts against the motor.'
          : "The z axis isn't inverted."
      }}
    </p>
    <p>
      Moving +z should raise the objective, towards the sample. If it lowers it, invert z, and check
      again.
    </p>
    <ActionButton :button="invert" />
    <StageMeasure />
  </div>
</template>

<style scoped>
.stage-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  align-items: flex-start;
  max-width: 36rem;
}

.stage-settings p {
  margin: 0;
}
</style>
