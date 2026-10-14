<script setup lang="ts">
// The Sequence tab (P42). Before a sequence, the pane sets how long it runs,
// as hours, minutes and seconds, the interval, and an autofocus before each
// capture with its range, and names it. While one runs, wherever it was
// started, the pane shows its images captured and expected, the time elapsed
// and remaining, a mini stream, its log and Cancel, and the main view its
// latest image, or the live image until there is one. Once it ends, it links
// to its gallery entry, can be downloaded, and Close goes back to the form.
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'
import { RouterLink } from 'vue-router'

import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import LiveImage from '@/live/LiveImage.vue'
import MiniStream from '@/live/MiniStream.vue'
import DownloadButton from '@/server-ui/DownloadButton.vue'
import { dataUrl } from '@/server-ui/download'
import ServerInterface from '@/server-ui/ServerInterface.vue'
import type { DownloadButtonElement, UiElement } from '@/server-ui/types'
import AppButton from '@/ui/AppButton.vue'
import ControlPane from '@/ui/ControlPane.vue'
import FormField from '@/ui/FormField.vue'
import MainView from '@/ui/MainView.vue'
import { useToast } from '@/ui/overlays'
import TextField from '@/ui/TextField.vue'

import { clock, useSequence } from './sequence'

const connection = useConnectionStore()
const toast = useToast()
const sequence = useSequence((error) =>
  toast.error('The sequence didn’t start.', {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  }),
)
const { shown, current, running, cancelling, log } = sequence
const name = ref('')

const control = (property: string, label: string, more: object = {}): UiElement => ({
  kind: 'property_control',
  thing: 'sequence',
  property,
  label,
  ...more,
})
const form = [
  control('duration', 'Duration', { input: 'duration_hms' }),
  control('dt', 'Interval (s)'),
  control('autofocus_enabled', 'Autofocus before each capture'),
  control('autofocus_dz', 'Autofocus range (steps)'),
]

const info = computed(() => {
  const details = current.value
  if (!details) return []
  const lines = [
    ['Sequence', details.name],
    ['Images', `${details.image_count} of ${details.points - details.skipped}`],
    ['Time elapsed', clock(details.duration_seconds)],
  ]
  if (running.value) {
    lines.push(['Time remaining', clock(details.last_point_seconds - details.duration_seconds)])
  }
  const { skipped } = details
  if (skipped) lines.push(['Skipped', `${skipped} time point${skipped === 1 ? '' : 's'}`])
  return lines
})
const outcome = computed(() => {
  const result = current.value?.result
  if (result === 'success') return 'The sequence has finished.'
  if (result === 'cancelled') return 'The sequence was cancelled.'
  return result
    ? `The sequence stopped: ${result.replace(/^error: /, '')}.`
    : 'The sequence has ended.'
})
const zip = computed<DownloadButtonElement | undefined>(() => {
  const name = current.value?.name
  if (!name || !current.value?.image_count) return undefined
  return {
    kind: 'download_button',
    label: 'Download ZIP',
    source: 'action',
    thing: 'sequence',
    action: 'download_zip',
    input: { name },
    filename: `${name}.zip`,
  }
})
const latest = computed(() => {
  const path = current.value?.latest_image
  const client = connection.client
  return path && client ? dataUrl(path, client.baseUrl).href : undefined
})

// The log follows its latest line.
const logList = useTemplateRef<HTMLOListElement>('log')
watch(log, async () => {
  await nextTick()
  if (logList.value) logList.value.scrollTop = logList.value.scrollHeight
})
</script>

<template>
  <div class="sequence-page">
    <ControlPane width="wide">
      <h1 class="visually-hidden">Sequence</h1>
      <template v-if="shown">
        <h2 class="sequence-page__heading">{{ running ? 'Running' : 'Sequence ended' }}</h2>
        <dl v-if="info.length" class="sequence-page__info">
          <template v-for="[label, value] in info" :key="label">
            <dt>{{ label }}</dt>
            <dd>{{ value }}</dd>
          </template>
        </dl>
        <MiniStream v-if="latest" />
        <FormField label="Log">
          <ol ref="log" class="sequence-page__log" aria-live="polite">
            <li v-for="(line, index) in log" :key="index">{{ line }}</li>
          </ol>
        </FormField>
        <AppButton v-if="running" variant="danger" :disabled="cancelling" @click="sequence.cancel">
          {{ cancelling ? 'Cancelling…' : 'Cancel' }}
        </AppButton>
        <template v-else>
          <p class="sequence-page__outcome">{{ outcome }}</p>
          <RouterLink
            v-if="current"
            class="sequence-page__link"
            :to="{ name: 'gallery', query: { view: `sequences/${current.name}` } }"
          >
            See it in the gallery
          </RouterLink>
          <DownloadButton v-if="zip" :button="zip" />
          <AppButton @click="sequence.close">Close</AppButton>
        </template>
      </template>
      <template v-else>
        <ServerInterface :elements="form" />
        <FormField label="Name" help="Names the sequence. Left empty, it’s named by the time.">
          <TextField v-model="name" @submit="sequence.start(name)" />
        </FormField>
        <AppButton variant="primary" @click="sequence.start(name)">Start Sequence</AppButton>
      </template>
    </ControlPane>
    <MainView v-if="latest">
      <img class="sequence-page__latest" :src="latest" alt="The sequence's latest image" />
    </MainView>
    <LiveImage v-else />
  </div>
</template>

<style scoped>
.sequence-page {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.sequence-page :deep(.control-pane) {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.sequence-page__heading {
  margin: 0;
  font-size: var(--font-size-lg);
}

.sequence-page__info {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-1) var(--space-3);
  margin: 0;
}

.sequence-page__info dt,
.sequence-page__outcome {
  margin: 0;
  color: var(--color-text-muted);
}

.sequence-page__info dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.sequence-page__link {
  color: var(--color-accent-text);
}

.sequence-page__log {
  max-height: 12rem;
  margin: 0;
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-6);
  overflow-y: auto;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}

.sequence-page__latest {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
</style>
