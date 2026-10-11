<script setup lang="ts">
// A button that runs an action, described by a Thing's panel (ADR-0024). If
// the panel gives a question, it's asked first. While the action runs, its
// latest log message shows beside the button, or its log shows in a dialog,
// with a camera's small live stream if the panel names one, and it can be
// cancelled unless the panel says not. Once it succeeds, it says so if the
// panel asks, and asks for the panel to be refreshed if the panel says to. A
// failure is reported. If the Thing lacks the action, the button says so.
import { computed, ref, shallowRef } from 'vue'

import { ApiError } from '@/api/wot/errors'
import type { Invocation, InvocationRecord } from '@/api/wot/invocation'
import { useConnectionStore } from '@/connection/store'
import LiveImage from '@/live/LiveImage.vue'
import AppButton from '@/ui/AppButton.vue'
import AppDialog from '@/ui/AppDialog.vue'
import AppSpinner from '@/ui/AppSpinner.vue'
import { useConfirm, useToast } from '@/ui/overlays'
import ProgressBar from '@/ui/ProgressBar.vue'

import type { ActionButtonElement } from './types'

const props = defineProps<{ button: ActionButtonElement }>()
const emit = defineEmits<{ refresh: [] }>()
const connection = useConnectionStore()
const confirm = useConfirm()
const toast = useToast()

const thing = computed(() => {
  const td = connection.descriptions?.[props.button.thing]
  if (props.button.broken || !td?.actions?.[props.button.action]) return undefined
  return connection.client?.consume(td)
})
const running = shallowRef<Invocation>()
const log = ref<string[]>([])
const inDialog = computed(() => props.button.progress === 'dialog')
const cancellable = computed(() => props.button.cancellable !== false)

async function run() {
  const target = thing.value
  const { action, input, label, confirm: question, success } = props.button
  if (!target || running.value) return
  if (question && !(await confirm({ title: label, message: question, confirmLabel: label }))) return
  log.value = []
  const onUpdate = (record: InvocationRecord) =>
    (log.value = (record.log ?? []).map((entry) => entry.message))
  try {
    running.value = await target.invokeAction(action, input ?? {}, { onUpdate })
    const output = await running.value.output()
    if (success === 'output')
      toast.success(typeof output === 'string' ? output : JSON.stringify(output))
    else if (success) toast.success(success.message)
    if (props.button.refresh) emit('refresh')
  } catch (error) {
    if (error instanceof ApiError && error.kind === 'cancelled') return
    toast.error(`${label} failed.`, {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
  } finally {
    running.value = undefined
  }
}

const cancel = () => void running.value?.cancel().catch(() => {})
</script>

<template>
  <p v-if="!thing" class="action-button__broken">
    {{ button.label }} isn't available: {{ button.thing }} has no action “{{ button.action }}”.
  </p>
  <div v-else class="action-button">
    <div class="action-button__row">
      <AppButton
        :variant="button.secondary ? 'default' : 'primary'"
        :disabled="!!running"
        @click="run"
      >
        <AppSpinner v-if="running && !inDialog" size="var(--icon-size-sm)" />
        {{ button.label }}
      </AppButton>
      <AppButton v-if="running && !inDialog && cancellable" variant="danger" @click="cancel">
        Cancel
      </AppButton>
    </div>
    <p v-if="running && !inDialog && log.length" class="action-button__status" role="status">
      {{ log.at(-1) }}
    </p>
    <AppDialog :open="!!running && inDialog" :title="button.label" :dismissible="false">
      <LiveImage
        v-if="button.stream"
        :thing="button.stream"
        stream="lores_mjpeg_stream"
        class="action-button__stream"
      />
      <ProgressBar :label="`${button.label} is running`" />
      <ul v-if="log.length" class="action-button__log" role="log">
        <li v-for="(message, index) in log" :key="index">{{ message }}</li>
      </ul>
      <template v-if="cancellable" #footer>
        <AppButton variant="danger" @click="cancel">Cancel</AppButton>
      </template>
    </AppDialog>
  </div>
</template>

<style scoped>
.action-button {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.action-button__row {
  display: flex;
  gap: var(--space-2);
}

.action-button__broken {
  margin: 0;
  color: var(--color-danger);
  font-size: var(--font-size-sm);
}

.action-button__status {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.action-button__stream {
  aspect-ratio: 4 / 3;
  margin-bottom: var(--space-3);
}

.action-button__log {
  margin: var(--space-3) 0 0;
  padding-left: var(--space-5);
  font-size: var(--font-size-sm);
}
</style>
