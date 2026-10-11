<script setup lang="ts">
// Capture, in the control pane: a full-resolution image, saved to the
// gallery (the data folder's captures, ADR-0025) or downloaded, as the
// choice beside the button says. The choice is kept in this browser. The `c`
// key captures too, and the live image flashes as it's taken.
import photoCamera from '@material-symbols/svg-400/outlined/photo_camera.svg?raw'
import { ref, watch } from 'vue'

import { useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import { useShortcut } from '@/app/shortcuts'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import { flashLiveImage } from '@/live/flash'
import AppButton from '@/ui/AppButton.vue'
import AppSelect from '@/ui/AppSelect.vue'
import FormField from '@/ui/FormField.vue'
import { useToast } from '@/ui/overlays'

type Destination = 'gallery' | 'download'

/** Where the choice is kept, in this browser. */
const KEY = 'microscope.capture-to'
const destinations = [
  { value: 'gallery' as const, label: 'Save to gallery' },
  { value: 'download' as const, label: 'Download' },
]

const camera = useThing('camera')
const connection = useConnectionStore()
const host = useHost()
const toast = useToast()
const busy = ref(false)

const destination = ref<Destination>(read())
watch(destination, (value) => {
  try {
    window.localStorage.setItem(KEY, value)
  } catch {
    // Without storage, the choice lasts until the page reloads.
  }
})

function read(): Destination {
  try {
    return window.localStorage.getItem(KEY) === 'download' ? 'download' : 'gallery'
  } catch {
    return 'gallery'
  }
}

async function capture() {
  const client = connection.client
  if (!camera.value || !client || busy.value) return
  busy.value = true
  flashLiveImage()
  const keep = destination.value === 'gallery'
  try {
    const file = await camera.value.run('capture', { capture_mode: 'standard', retain_image: keep })
    if (keep) {
      toast.success('Saved to the gallery.')
    } else {
      const time = new Date().toISOString().slice(0, 19).replaceAll(':', '-')
      await host.saveFile(
        `capture-${time}Z.jpeg`,
        await client.download(new URL(file.href, client.baseUrl)),
      )
    }
  } catch (error) {
    toast.error("Couldn't capture.", {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
  } finally {
    busy.value = false
  }
}

useShortcut({ keys: 'c', description: 'Capture', group: 'Camera', run: () => void capture() })
</script>

<template>
  <div v-if="camera" class="capture-control">
    <FormField label="On capture">
      <AppSelect v-model="destination" size="sm" :options="destinations" />
    </FormField>
    <AppButton :icon="photoCamera" :disabled="busy" @click="capture">Capture</AppButton>
  </div>
</template>

<style scoped>
.capture-control {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-top: var(--space-4);
}
</style>
