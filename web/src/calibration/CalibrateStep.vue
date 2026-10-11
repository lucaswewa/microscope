<script setup lang="ts">
// A step of the calibration wizard that calibrates a Thing, through the
// Thing's own panel (ADR-0024) or a button. It tells the wizard whether the
// Thing still needs calibrating, when it starts and after each calibration,
// so that Next can offer to skip it until then.
import { onBeforeUnmount } from 'vue'

import { useConnectionStore } from '@/connection/store'
import MiniStream from '@/live/MiniStream.vue'
import ActionButton from '@/server-ui/ActionButton.vue'
import ServerPanel from '@/server-ui/ServerPanel.vue'
import type { ActionButtonElement } from '@/server-ui/types'

const props = defineProps<{
  thing: string
  lines?: string[]
  stream?: boolean
  panel?: string
  button?: ActionButtonElement
}>()
const emit = defineEmits<{ waiting: [waiting: boolean] }>()
const connection = useConnectionStore()

let shown = true
onBeforeUnmount(() => (shown = false))

async function check() {
  const td = connection.descriptions?.[props.thing]
  if (!td || !connection.client) return
  const thing = connection.client.consume(td)
  const required = await thing.readProperty<boolean>('calibration_required').catch(() => false)
  if (shown) emit('waiting', required)
}
void check()
</script>

<template>
  <div class="wizard-step">
    <p v-for="line in lines" :key="line">{{ line }}</p>
    <MiniStream v-if="stream" />
    <ServerPanel v-if="panel" :thing="thing" :path="panel" @refresh="check" />
    <ActionButton v-if="button" :button="button" @refresh="check" />
  </div>
</template>
