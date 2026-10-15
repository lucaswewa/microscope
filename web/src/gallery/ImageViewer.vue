<script setup lang="ts">
// The viewer (ADR-0027): an item's image in a dialog, to zoom and pan. It's
// one image, a deep-zoom image, or a sequence stepped through with a slider,
// or played, each image for a number of seconds, once or in a loop (P52).
// Its brightness, contrast and saturation can be changed and reset, and it
// can fill the screen. Its controls are the app's, so they follow the theme.
import fitIcon from '@material-symbols/svg-400/outlined/fit_screen.svg?raw'
import fullIcon from '@material-symbols/svg-400/outlined/fullscreen.svg?raw'
import leaveIcon from '@material-symbols/svg-400/outlined/fullscreen_exit.svg?raw'
import pauseIcon from '@material-symbols/svg-400/outlined/pause.svg?raw'
import playIcon from '@material-symbols/svg-400/outlined/play_arrow.svg?raw'
import zoomInIcon from '@material-symbols/svg-400/outlined/zoom_in.svg?raw'
import zoomOutIcon from '@material-symbols/svg-400/outlined/zoom_out.svg?raw'
import { computed, onBeforeUnmount, reactive, ref, useTemplateRef } from 'vue'

import type { GalleryEntry } from '@/api/things'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import { dataUrl } from '@/server-ui/download'
import AppButton from '@/ui/AppButton.vue'
import AppCheckbox from '@/ui/AppCheckbox.vue'
import AppDialog from '@/ui/AppDialog.vue'
import AppSlider from '@/ui/AppSlider.vue'
import IconButton from '@/ui/IconButton.vue'
import NumberField from '@/ui/NumberField.vue'

import DeepZoom from './DeepZoom.vue'
import { usePlayback } from './playback'

const props = defineProps<{ entry: GalleryEntry }>()
const emit = defineEmits<{ close: [] }>()
const connection = useConnectionStore()
const host = useHost()

const url = (path: string) => dataUrl(path, connection.client!.baseUrl).href
const sources = computed(() => {
  const { viewer } = props.entry
  if (viewer.type === 'dzi') return [url(viewer.path)]
  const paths = viewer.type === 'image' ? [viewer.path] : viewer.paths
  return paths.map((path) => ({ type: 'image' as const, url: url(path) }))
})
const page = ref(props.entry.viewer.type === 'directory' ? props.entry.viewer.start : 0)
const playback = usePlayback(page, () => sources.value.length)
const { playing, seconds, loop } = playback

const LOOKS = { brightness: 100, contrast: 100, saturation: 100 }
const looks = reactive({ ...LOOKS })
const filter = computed(
  () =>
    `brightness(${looks.brightness}%) contrast(${looks.contrast}%) saturate(${looks.saturation}%)`,
)
const sliders = [
  { key: 'brightness', label: 'Brightness' },
  { key: 'contrast', label: 'Contrast' },
  { key: 'saturation', label: 'Saturation' },
] as const

const view = useTemplateRef<InstanceType<typeof DeepZoom>>('view')
const full = ref(false)
async function fillScreen(on: boolean) {
  full.value = on
  await host.fullscreen.set(on).catch(() => {})
}
// Leaving full screen otherwise, as Escape does, ends it here too.
const unwatch = host.fullscreen.watch((active) => active || (full.value = false))
onBeforeUnmount(() => {
  unwatch()
  if (full.value) void host.fullscreen.set(false).catch(() => {})
})

const open = computed({ get: () => true, set: (open) => open || emit('close') })
</script>

<template>
  <AppDialog v-model:open="open" :title="entry.name" :size="full ? 'full' : 'lg'">
    <div class="image-viewer">
      <div class="image-viewer__bar">
        <IconButton :icon="zoomInIcon" label="Zoom in" size="sm" @click="view?.zoom(1.5)" />
        <IconButton :icon="zoomOutIcon" label="Zoom out" size="sm" @click="view?.zoom(1 / 1.5)" />
        <IconButton :icon="fitIcon" label="Show it all" size="sm" @click="view?.home()" />
        <IconButton
          :icon="full ? leaveIcon : fullIcon"
          :label="full ? 'Leave full screen' : 'Full screen'"
          size="sm"
          @click="fillScreen(!full)"
        />
      </div>
      <DeepZoom
        ref="view"
        :sources="sources"
        :page="page"
        :filter="filter"
        class="image-viewer__view"
        :class="{ 'image-viewer__view--full': full }"
        @shown="playback.shown"
      />
      <div v-if="sources.length > 1" class="image-viewer__row">
        <IconButton
          :icon="playing ? pauseIcon : playIcon"
          :label="playing ? 'Pause' : 'Play'"
          size="sm"
          @click="playing ? playback.pause() : playback.play()"
        />
        <AppSlider v-model="page" label="Image" :max="sources.length - 1" />
        <span class="image-viewer__count">{{ page + 1 }} of {{ sources.length }}</span>
        <label class="image-viewer__seconds">
          <span>Seconds each</span>
          <NumberField v-model="seconds" size="sm" :min="0.1" :max="60" :step="0.1" />
        </label>
        <AppCheckbox v-model="loop" label="Loop" />
      </div>
      <div class="image-viewer__looks">
        <div v-for="slider in sliders" :key="slider.key" class="image-viewer__look">
          <span aria-hidden="true">{{ slider.label }}</span>
          <AppSlider v-model="looks[slider.key]" :label="slider.label" :max="200" />
        </div>
        <AppButton size="sm" @click="Object.assign(looks, LOOKS)">Reset</AppButton>
      </div>
    </div>
  </AppDialog>
</template>

<style scoped>
.image-viewer {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  height: 100%;
}

.image-viewer__bar,
.image-viewer__row,
.image-viewer__looks {
  display: flex;
  gap: var(--space-3);
  align-items: center;
}

.image-viewer__view {
  height: 60vh;
}

.image-viewer__view--full {
  flex: 1;
  height: auto;
}

.image-viewer__count {
  flex: none;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.image-viewer__seconds {
  display: flex;
  flex: none;
  gap: var(--space-2);
  align-items: center;
  font-size: var(--font-size-sm);
}

.image-viewer__seconds :deep(input) {
  width: var(--space-12);
}

.image-viewer__look {
  display: flex;
  flex: 1;
  flex-direction: column;
  font-size: var(--font-size-sm);
}
</style>
