<script setup lang="ts">
// What a scan is and how far it has got: its ID, which opens the workflow's
// settings for it, the images it has captured, and how long it has run.
import settingsIcon from '@material-symbols/svg-400/outlined/tune.svg?raw'
import { computed, ref } from 'vue'

import type { LiveScanDetails } from '@/api/things'
import AppDialog from '@/ui/AppDialog.vue'
import IconButton from '@/ui/IconButton.vue'

const props = defineProps<{ details: LiveScanDetails }>()
const open = ref(false)

const duration = computed(() => {
  const seconds = Math.round(props.details.duration_seconds)
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
})
const settings = computed(() => {
  const settings = props.details.settings
  return settings && typeof settings === 'object' ? Object.entries(settings) : []
})
</script>

<template>
  <dl class="scan-info">
    <dt>Scan ID</dt>
    <dd class="scan-info__name">
      {{ details.name }}
      <IconButton
        v-if="settings.length"
        :icon="settingsIcon"
        label="The scan's settings"
        size="sm"
        @click="open = true"
      />
    </dd>
    <dt>Images captured</dt>
    <dd>{{ details.image_count }}</dd>
    <dt>Duration</dt>
    <dd>{{ duration }}</dd>
  </dl>
  <AppDialog v-model:open="open" title="Scan settings" :description="details.name">
    <dl class="scan-info__settings">
      <template v-for="[name, value] in settings" :key="name">
        <dt>{{ name }}</dt>
        <dd>{{ typeof value === 'string' ? value : JSON.stringify(value) }}</dd>
      </template>
    </dl>
  </AppDialog>
</template>

<style scoped>
.scan-info,
.scan-info__settings {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-1) var(--space-3);
  margin: 0;
}

.scan-info dt,
.scan-info__settings dt {
  color: var(--color-text-muted);
}

.scan-info dd,
.scan-info__settings dd {
  margin: 0;
}

.scan-info__name {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  overflow-wrap: anywhere;
}
</style>
