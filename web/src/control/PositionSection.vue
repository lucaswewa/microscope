<script setup lang="ts">
// The stage's position: where it is, observed live, and fields to send it
// somewhere. A move (Move or Move Home) can be cancelled while it runs.
import refreshIcon from '@material-symbols/svg-400/outlined/sync.svg?raw'
import { ref, shallowRef, watch } from 'vue'

import { useThing, type Position, type TypedInvocation } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import AppButton from '@/ui/AppButton.vue'
import IconButton from '@/ui/IconButton.vue'
import NumberField from '@/ui/NumberField.vue'
import { useConfirm, useToast } from '@/ui/overlays'

const AXES = ['x', 'y', 'z'] as const

const stage = useThing('stage')
const confirm = useConfirm()
const toast = useToast()
const live = ref<Position>()
const moving = ref(false)
const target = ref<Position>({ x: 0, y: 0, z: 0 })
const running = shallowRef<TypedInvocation<unknown>>()

watch(
  stage,
  (thing, _, onCleanup) => {
    if (!thing) return
    const stops = [
      thing.observe('position', (position) => (live.value = position)),
      thing.observe('moving', (value) => (moving.value = value)),
    ]
    onCleanup(() => stops.forEach((stop) => stop()))
    void refresh()
  },
  { immediate: true },
)

function report(message: string, error: unknown) {
  if (error instanceof ApiError && error.kind === 'cancelled') return
  toast.error(message, {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  })
}

/** Fills the fields in with where the stage is. */
async function refresh() {
  try {
    const position = await stage.value?.read('position')
    if (position) live.value = target.value = { ...position }
  } catch (error) {
    report('The stage’s position couldn’t be read.', error)
  }
}

async function follow(start: () => Promise<TypedInvocation<unknown>> | undefined) {
  try {
    running.value = await start()
    await running.value?.output()
  } catch (error) {
    report('The stage didn’t move.', error)
  } finally {
    running.value = undefined
    await refresh()
  }
}

const move = () => follow(() => stage.value?.invoke('move_absolute', { ...target.value }))

async function moveHome() {
  const confirmed = await confirm({
    title: 'Move to (0, 0, 0)?',
    message: 'Remove your sample first: the stage may travel far.',
    confirmLabel: 'Move Home',
  })
  if (confirmed) await follow(() => stage.value?.invoke('move_to_origin'))
}

async function setHome() {
  try {
    await stage.value?.run('set_zero_position')
  } catch (error) {
    report('Home couldn’t be set.', error)
  }
  await refresh()
}
</script>

<template>
  <div class="position-section">
    <div class="position-section__live">
      <span>{{ live ? `x ${live.x}, y ${live.y}, z ${live.z}` : 'Position unknown' }}</span>
      <IconButton
        :icon="refreshIcon"
        label="Fill in where the stage is"
        size="sm"
        variant="ghost"
        @click="refresh"
      />
    </div>
    <p v-if="moving" class="position-section__moving">Moving…</p>
    <label v-for="axis in AXES" :key="axis" class="position-section__axis">
      <span>{{ axis }}</span>
      <NumberField v-model="target[axis]" size="sm" :step="1" @submit="move" />
    </label>
    <AppButton v-if="running" variant="danger" @click="running.cancel()">Cancel</AppButton>
    <AppButton v-else variant="primary" :disabled="!stage" @click="move">Move</AppButton>
    <AppButton :disabled="!stage || !!running" @click="setHome">Set Home</AppButton>
    <AppButton :disabled="!stage || !!running" @click="moveHome">Move Home</AppButton>
  </div>
</template>

<style scoped>
.position-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.position-section__live {
  display: flex;
  gap: var(--space-1);
  align-items: center;
  justify-content: space-between;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
}

.position-section__moving {
  margin: 0;
  color: var(--color-accent-text);
}

.position-section__axis {
  display: grid;
  grid-template-columns: var(--space-4) 1fr;
  gap: var(--space-2);
  align-items: center;
}
</style>
