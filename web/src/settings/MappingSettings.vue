<script setup lang="ts">
// Settings, Camera–stage mapping: calibrating it, after a question and with
// the stream in the progress dialog; what it found, with its matrix; and its
// data, to download.
import download from '@material-symbols/svg-400/outlined/download.svg?raw'
import { computed, ref, watchEffect } from 'vue'

import { useThing } from '@/api/things'
import { useHost } from '@/host'
import ActionButton from '@/server-ui/ActionButton.vue'
import type { ActionButtonElement } from '@/server-ui/types'
import AppButton from '@/ui/AppButton.vue'

const mapping = useThing('camera_stage_mapping')
const host = useHost()
const matrix = ref<number[][] | null>()
const resolution = ref<number[] | null>()
const calibrated = ref<string | null>()

async function load() {
  const thing = mapping.value
  if (!thing) return
  ;[matrix.value, resolution.value, calibrated.value] = await Promise.all([
    thing.read('image_to_stage_displacement_matrix'),
    thing.read('image_resolution'),
    thing.read('last_cal_timestamp'),
  ])
}
watchEffect(() => void load().catch(() => {}))

const calibrate: ActionButtonElement = {
  kind: 'action_button',
  thing: 'camera_stage_mapping',
  action: 'calibrate_xy',
  label: 'Calibrate',
  confirm:
    'Calibrate how the stage moves the image? It moves the stage about for a minute, so the sample must be in focus, and the microscope is busy meanwhile.',
  progress: 'dialog',
  stream: 'camera',
  success: { message: 'The camera–stage mapping is calibrated.' },
  refresh: true,
}

/** The scale, and the field of view along each of the image's axes, in steps. */
const derived = computed(() => {
  const [[a, b], [c, d]] = (matrix.value ?? [
    [0, 0],
    [0, 0],
  ]) as [[number, number], [number, number]]
  const [width, height] = (resolution.value ?? [0, 0]) as [number, number]
  return {
    stepsPerPixel: Math.sqrt(Math.abs(a * d - b * c)),
    field: [Math.hypot(a, c) * width, Math.hypot(b, d) * height].map(Math.round),
  }
})

async function save() {
  const data = await mapping.value?.read('last_calibration')
  if (!data) return
  const json = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
  await host.saveFile(`camera_stage_mapping_${data.timestamp}.json`, json)
}
</script>

<template>
  <div class="mapping-settings">
    <ActionButton :button="calibrate" @refresh="load" />
    <template v-if="matrix && resolution">
      <dl class="mapping-settings__details">
        <dt>Steps per pixel moved</dt>
        <dd>
          <table class="mapping-settings__matrix" aria-label="The image-to-stage matrix">
            <tr v-for="(row, i) in matrix" :key="i">
              <td v-for="(value, j) in row" :key="j">
                {{ (Math.abs(value) < 0.0005 ? 0 : value).toFixed(3) }}
              </td>
            </tr>
          </table>
        </dd>
        <dt>Scale</dt>
        <dd>{{ derived.stepsPerPixel.toFixed(3) }} steps per pixel</dd>
        <dt>Field of view</dt>
        <dd>{{ derived.field[0] }} × {{ derived.field[1] }} steps</dd>
        <dt>Calibrated with</dt>
        <dd>{{ resolution[0] }} × {{ resolution[1] }} pixel frames, at {{ calibrated }}</dd>
      </dl>
      <AppButton :icon="download" @click="save">Download the calibration data</AppButton>
    </template>
    <p v-else class="mapping-settings__note">It isn't calibrated yet.</p>
  </div>
</template>

<style scoped>
.mapping-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  align-items: flex-start;
}

.mapping-settings__details {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-2) var(--space-4);
  align-items: center;
  margin: 0;
}

.mapping-settings__details dt {
  color: var(--color-text-muted);
}

.mapping-settings__details dd {
  margin: 0;
}

.mapping-settings__matrix {
  border-right: 2px solid var(--color-border-strong);
  border-left: 2px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  font-family: var(--font-family-mono);
  font-variant-numeric: tabular-nums;
}

.mapping-settings__matrix td {
  padding: 0 var(--space-2);
  text-align: right;
}

.mapping-settings__note {
  margin: 0;
  color: var(--color-text-muted);
}
</style>
