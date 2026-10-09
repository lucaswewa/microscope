<script setup lang="ts">
// Covers the page while the microscope can't be reached, saying why and when
// the app tries again. Retry tries at once; the Connect screen, which it
// leaves uncovered, can choose another microscope.
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import AppButton from '@/ui/AppButton.vue'
import AppCard from '@/ui/AppCard.vue'
import AppSpinner from '@/ui/AppSpinner.vue'
import ErrorDetails from '@/ui/ErrorDetails.vue'
import SectionHeading from '@/ui/SectionHeading.vue'

import { apiRoot, displayOrigin } from './profiles'
import { useConnectionStore } from './store'

const connection = useConnectionStore()
const route = useRoute()
const router = useRouter()

const shown = computed(
  () =>
    (connection.state === 'lost' || connection.state === 'reconnecting') &&
    route.name !== 'connect',
)
const reconnecting = computed(() => connection.state === 'reconnecting')
const origin = computed(
  () => connection.profile && displayOrigin(apiRoot(connection.profile).origin),
)
</script>

<template>
  <div v-if="shown" class="offline-overlay">
    <AppCard class="offline-overlay__card" role="alert">
      <SectionHeading :level="2">
        {{ connection.descriptions ? 'Connection lost' : 'Can’t reach the microscope' }}
      </SectionHeading>
      <p>The microscope at {{ origin }} isn’t answering.</p>
      <ErrorDetails v-if="connection.error" :error="connection.error" />
      <p class="offline-overlay__status">
        <template v-if="reconnecting">
          <AppSpinner size="var(--icon-size-sm)" /> Trying to reconnect…
        </template>
        <template v-else>The app will try again shortly.</template>
      </p>
      <div class="offline-overlay__actions">
        <AppButton variant="primary" :disabled="reconnecting" @click="connection.retry()">
          Retry
        </AppButton>
        <AppButton @click="router.push({ name: 'connect' })">Choose a microscope</AppButton>
      </div>
    </AppCard>
  </div>
</template>

<style scoped>
.offline-overlay {
  position: absolute;
  inset: 0;
  z-index: var(--z-dropdown);
  display: grid;
  place-items: center;
  padding: var(--space-6);
  background: var(--color-overlay);
}

.offline-overlay__card {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  max-width: 480px;
}

.offline-overlay__card p {
  margin: 0;
}

.offline-overlay__status {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  color: var(--color-text-muted);
}

.offline-overlay__actions {
  display: flex;
  gap: var(--space-2);
}
</style>
