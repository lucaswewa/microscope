<script setup lang="ts">
// A deep-zoom view (ADR-0027), on OpenSeadragon, which is loaded when first
// needed: of one image, a DZI pyramid, or images shown a page at a time. The
// wheel and a pinch zoom, and dragging pans. It's drawn through `filter`, a
// CSS filter, and marked as loading, shown or failed.
import type OpenSeadragon from 'openseadragon'
import { onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'

const props = defineProps<{
  /** A DZI file's URL, or images'. */
  sources: (string | { type: 'image'; url: string })[]
  /** The image shown, when there are several. */
  page: number
  filter: string
}>()

const element = useTemplateRef<HTMLElement>('element')
const status = ref<'loading' | 'shown' | 'failed'>('loading')
let viewer: OpenSeadragon.Viewer | undefined
let unmounted = false

onMounted(async () => {
  const { default: openSeadragon } = await import('openseadragon')
  if (unmounted || !element.value) return
  viewer = openSeadragon({
    element: element.value,
    // Its types leave out images in a sequence, which it takes.
    tileSources: props.sources as OpenSeadragon.Options['tileSources'],
    sequenceMode: props.sources.length > 1,
    initialPage: props.page,
    // The app draws its own controls, which follow the theme.
    showNavigationControl: false,
    showSequenceControl: false,
    // Images from another server can only be drawn with its permission.
    crossOriginPolicy: 'Anonymous',
    visibilityRatio: 1,
  })
  viewer.addHandler('open', () => (status.value = 'shown'))
  viewer.addHandler('open-failed', () => (status.value = 'failed'))
})
watch(
  () => props.page,
  (page) => viewer?.goToPage(page),
)
onBeforeUnmount(() => {
  unmounted = true
  viewer?.destroy()
})

defineExpose({
  /** Zooms in by `factor`, or out by less than 1. */
  zoom(factor: number) {
    viewer?.viewport.zoomBy(factor)
    viewer?.viewport.applyConstraints()
  },
  /** Shows it all again. */
  home: () => viewer?.viewport.goHome(),
})
</script>

<template>
  <div class="deep-zoom" :data-status="status">
    <div ref="element" class="deep-zoom__view" :style="{ filter }" />
    <p v-if="status === 'failed'" class="deep-zoom__failed">It couldn't be shown.</p>
  </div>
</template>

<style scoped>
.deep-zoom {
  position: relative;
  min-height: 0;
  background: var(--color-image-backdrop);
}

.deep-zoom__view {
  position: absolute;
  inset: 0;
}

.deep-zoom__failed {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  margin: 0;
  color: var(--color-text-muted);
}
</style>
