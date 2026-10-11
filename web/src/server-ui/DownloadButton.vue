<script setup lang="ts">
// A button that downloads a file, described by a Thing's panel (ADR-0024):
// what one of the Thing's endpoints answers, or the file an action returns.
// The file is saved through the host, under the panel's name for it. If the
// Thing lacks the source, the button says so.
import download from '@material-symbols/svg-400/outlined/download.svg?raw'
import { computed, ref } from 'vue'

import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import AppButton from '@/ui/AppButton.vue'
import { useToast } from '@/ui/overlays'

import type { DownloadButtonElement } from './types'

const props = defineProps<{ button: DownloadButtonElement }>()
const connection = useConnectionStore()
const host = useHost()
const toast = useToast()
const busy = ref(false)

const thing = computed(() => {
  const { button } = props
  const td = connection.descriptions?.[button.thing]
  const lacksAction = button.source === 'action' && !td?.actions?.[button.action]
  if (button.broken || !td || lacksAction) return undefined
  return connection.client?.consume(td)
})

/** Where the file is: the endpoint, or the link the action returns. */
async function locate(): Promise<URL> {
  const { button } = props
  const base = connection.client!.baseUrl
  if (button.source === 'endpoint') return new URL(`${button.thing}/${button.path}`, base)
  const output = await (await thing.value!.invokeAction(button.action, button.input ?? {})).output()
  const href = (output as { href?: unknown } | null)?.href
  if (typeof href !== 'string') throw new Error(`${button.action} didn't return a file.`)
  return new URL(href, base)
}

async function save() {
  const client = connection.client
  if (!thing.value || !client || busy.value) return
  busy.value = true
  try {
    await host.saveFile(props.button.filename, await client.download(await locate()))
  } catch (error) {
    toast.error(`Couldn't download ${props.button.filename}.`, {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <p v-if="!thing" class="download-button__broken">
    {{ button.label }} isn't available: {{ button.thing }} has no
    {{ button.source === 'endpoint' ? 'file' : 'action' }} “{{
      button.source === 'endpoint' ? button.path : button.action
    }}”.
  </p>
  <AppButton
    v-else
    :variant="button.secondary ? 'default' : 'primary'"
    :icon="download"
    :disabled="busy"
    @click="save"
  >
    {{ button.label }}
  </AppButton>
</template>

<style scoped>
.download-button__broken {
  margin: 0;
  color: var(--color-danger);
  font-size: var(--font-size-sm);
}
</style>
