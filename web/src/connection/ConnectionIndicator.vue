<script setup lang="ts">
// The connection's state at the foot of the rail: a dot and the microscope's
// name. It links to the Connect screen, and its tooltip says where.
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

import AppSpinner from '@/ui/AppSpinner.vue'
import AppTooltip from '@/ui/AppTooltip.vue'

import { apiRoot, displayOrigin } from './profiles'
import { useConnectionStore } from './store'

const connection = useConnectionStore()

const label = computed(
  () =>
    ({
      idle: 'Not connected',
      connecting: 'Connecting…',
      connected: connection.hostname ?? 'Connected',
      lost: 'Offline',
      reconnecting: 'Reconnecting…',
    })[connection.state],
)

const where = computed(() => {
  const profile = connection.profile
  if (!profile) return 'Choose a microscope'
  const origin = displayOrigin(apiRoot(profile).origin)
  return profile.kind === 'local' ? `This server (${origin})` : origin
})

const busy = computed(
  () => connection.state === 'connecting' || connection.state === 'reconnecting',
)
</script>

<template>
  <AppTooltip :text="where" side="right">
    <RouterLink
      to="/connect"
      class="connection-indicator"
      :data-state="connection.state"
      :aria-label="`${label}: ${where}`"
    >
      <AppSpinner v-if="busy" size="10px" class="connection-indicator__dot" />
      <span v-else class="connection-indicator__dot" />
      <span class="connection-indicator__label">{{ label }}</span>
    </RouterLink>
  </AppTooltip>
</template>

<style scoped>
.connection-indicator {
  display: flex;
  gap: var(--space-1);
  align-items: center;
  justify-content: center;
  height: var(--control-height-sm);
  padding: 0 var(--space-1);
  border-top: var(--border-width) solid var(--color-border);
  color: var(--color-rail-text);
  font-size: var(--font-size-xs);
  text-decoration: none;
}

.connection-indicator:hover {
  background: var(--color-rail-hover-bg);
}

.connection-indicator:focus-visible {
  outline-offset: calc(-1 * var(--focus-ring-width));
}

.connection-indicator__dot {
  flex: none;
  width: 8px;
  height: 8px;
  border-radius: var(--radius-full);
  background: var(--color-text-disabled);
}

.connection-indicator__dot.app-spinner {
  width: 10px;
  height: 10px;
  background: none;
}

[data-state='connected'] .connection-indicator__dot {
  background: var(--color-success);
}

[data-state='lost'] .connection-indicator__dot {
  background: var(--color-danger);
}

.connection-indicator__label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
