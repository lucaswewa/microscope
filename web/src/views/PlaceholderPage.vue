<script setup lang="ts">
// Stands in for a destination's page until the phase that builds it, with
// the layout that page will have.
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { DESTINATIONS } from '@/app/navigation'
import ControlPane from '@/ui/ControlPane.vue'
import MainView from '@/ui/MainView.vue'

const props = defineProps<{
  destinationId: string
  /** The control pane the page will have, if any. */
  pane?: 'narrow' | 'wide'
  /** Whether the page's main view will show the live image. */
  liveImage?: boolean
}>()

const route = useRoute()
const destination = computed(() => DESTINATIONS.find((d) => d.id === props.destinationId))
const section = computed(() => route.params.section as string | undefined)
</script>

<template>
  <div class="placeholder-page">
    <ControlPane v-if="pane" :width="pane">
      <p class="placeholder-page__note">Controls</p>
    </ControlPane>
    <MainView :backdrop="liveImage">
      <h1>{{ destination?.label }}</h1>
      <p v-if="section" class="placeholder-page__note">Section: {{ section }}</p>
      <p class="placeholder-page__note">This page is built in {{ destination?.builtIn }}.</p>
    </MainView>
  </div>
</template>

<style scoped>
.placeholder-page {
  display: flex;
  flex: 1;
  min-height: 0;
}

.placeholder-page__note {
  margin: 0 0 var(--space-2);
  color: var(--color-text-muted);
}
</style>
