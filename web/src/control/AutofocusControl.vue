<script setup lang="ts">
// Autofocus: sweeps through focus and moves to focus, from the button or
// the `a` key while the Control tab is open. While it runs, the button
// cancels it. If the image hardly changes, there's nothing to focus on, and
// the stage goes back to where it was (P54).
import { shallowRef } from 'vue'

import { useThing, type TypedInvocation } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import { useShortcut } from '@/app/shortcuts'
import AppButton from '@/ui/AppButton.vue'
import { useToast } from '@/ui/overlays'

/** The sweep, in steps: OpenFlexure's. */
const DZ = 2000

const autofocus = useThing('autofocus')
const toast = useToast()
const running = shallowRef<TypedInvocation<unknown>>()

async function run() {
  if (!autofocus.value || running.value) return
  try {
    running.value = await autofocus.value.invoke('fast_autofocus', { dz: DZ })
    const curve = (await running.value.output()) as { focus_z?: number | null } | null
    if (curve && curve.focus_z == null) {
      toast.info('Nothing to focus on here: move to the sample and try again.')
    }
  } catch (error) {
    if (error instanceof ApiError && error.kind === 'cancelled') return
    toast.error('Autofocus failed.', {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
  } finally {
    running.value = undefined
  }
}

useShortcut({ keys: 'a', description: 'Autofocus', group: 'Stage', run: () => void run() })
</script>

<template>
  <div v-if="autofocus" class="autofocus-control">
    <AppButton v-if="running" variant="danger" @click="running.cancel()">
      Cancel autofocus
    </AppButton>
    <AppButton v-else variant="primary" @click="run">Autofocus</AppButton>
  </div>
</template>

<style scoped>
.autofocus-control {
  display: flex;
  flex-direction: column;
  margin-top: var(--space-4);
}
</style>
