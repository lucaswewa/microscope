<script setup lang="ts">
// A step of the calibration wizard that says what to do, with the live image
// and the d-pad when they help.
import DirectionPad from '@/control/DirectionPad.vue'
import { useJog } from '@/control/useJog'
import MiniStream from '@/live/MiniStream.vue'

const props = defineProps<{
  lines: string[]
  bullets?: string[]
  stream?: boolean
  pad?: boolean
}>()
// The step is made afresh for each step of the wizard, so `pad` doesn't change.
const jog = props.pad ? useJog() : undefined
</script>

<template>
  <div class="wizard-step">
    <p v-for="line in lines" :key="line">{{ line }}</p>
    <ul v-if="bullets">
      <li v-for="bullet in bullets" :key="bullet">{{ bullet }}</li>
    </ul>
    <div v-if="stream || jog" class="wizard-step__view">
      <MiniStream v-if="stream" />
      <DirectionPad v-if="jog" :controller="jog" />
    </div>
  </div>
</template>

<style>
/* Not scoped: CalibrateStep shares it. */
.wizard-step {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.wizard-step p,
.wizard-step ul {
  margin: 0;
}

.wizard-step__view {
  display: flex;
  gap: var(--space-6);
  align-items: center;
}
</style>
