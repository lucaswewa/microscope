<script setup lang="ts">
// A panel a Thing serves (ADR-0024) from one of its endpoints, such as the
// camera's `manual_camera_settings`, drawn by ServerInterface. It's fetched
// again when one of its actions asks for a refresh, and when the
// connection changes.
import { ref, watchEffect } from 'vue'

import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import AppSpinner from '@/ui/AppSpinner.vue'

import ServerInterface from './ServerInterface.vue'
import type { UiElement } from './types'

const props = defineProps<{ thing: string; path: string }>()
const connection = useConnectionStore()
const elements = ref<UiElement[]>()
const failure = ref<string>()
const refreshes = ref(0)

watchEffect(async (onCleanup) => {
  void refreshes.value
  const client = connection.client
  if (!client || !connection.descriptions?.[props.thing]) return
  const controller = new AbortController()
  onCleanup(() => controller.abort())
  const url = new URL(`${props.thing}/${props.path}`, client.baseUrl)
  try {
    elements.value = (await client.request(url, { signal: controller.signal })) as UiElement[]
    failure.value = undefined
  } catch (error) {
    if (controller.signal.aborted) return
    failure.value = error instanceof ApiError ? error.message : String(error)
  }
})
</script>

<template>
  <p v-if="failure" class="server-panel__failure">This panel isn't available. {{ failure }}</p>
  <ServerInterface v-else-if="elements" :elements="elements" @refresh="refreshes++" />
  <AppSpinner v-else label="Loading the panel" />
</template>

<style scoped>
.server-panel__failure {
  margin: 0;
  color: var(--color-danger);
}
</style>
