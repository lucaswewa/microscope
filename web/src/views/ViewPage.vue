<script setup lang="ts">
// The View tab: the live image, full window, with a button that turns the
// stream off or on.
import visibility from '@material-symbols/svg-400/outlined/visibility.svg?raw'
import visibilityOff from '@material-symbols/svg-400/outlined/visibility_off.svg?raw'

import LiveImage from '@/live/LiveImage.vue'
import { useLiveImagePreferences } from '@/live/preferences'
import IconButton from '@/ui/IconButton.vue'

const preferences = useLiveImagePreferences()
</script>

<template>
  <section class="view-page" aria-labelledby="view-page-heading">
    <h1 id="view-page-heading" class="visually-hidden">View</h1>
    <LiveImage />
    <div class="view-page__tools">
      <IconButton
        v-if="preferences.streamDisabled"
        :icon="visibility"
        label="Enable stream"
        @click="preferences.streamDisabled = false"
      />
      <IconButton
        v-else
        :icon="visibilityOff"
        label="Disable stream"
        @click="preferences.streamDisabled = true"
      />
    </div>
  </section>
</template>

<style scoped>
.view-page {
  position: relative;
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.view-page__tools {
  position: absolute;
  top: var(--space-4);
  right: var(--space-4);
}
</style>
