<script setup lang="ts">
// A button that downloads a file, described by a Thing's panel (ADR-0024)
// or a gallery card (ADR-0026): what one of a Thing's endpoints answers, the
// file an action returns, or a file in the data folder. The file is saved
// through the host, under the name given for it. If the source is missing,
// the button says so.
import download from '@material-symbols/svg-400/outlined/download.svg?raw'
import { computed, ref } from 'vue'

import AppButton from '@/ui/AppButton.vue'

import { useDownload } from './download'
import type { DownloadButtonElement } from './types'

const props = defineProps<{ button: DownloadButtonElement }>()
const downloads = useDownload()
const busy = ref(false)
const missing = computed(() => downloads.missing(props.button))

async function save() {
  if (busy.value) return
  busy.value = true
  try {
    await downloads.save(props.button)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <p v-if="missing" class="download-button__broken">
    {{ button.label }} isn't available: {{ missing }}.
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
