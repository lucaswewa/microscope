<script setup lang="ts">
// A deep-zoom view (ADR-0027), on OpenSeadragon, which is loaded when first
// needed: of one image, a DZI pyramid, or images shown a page at a time. The
// wheel and a pinch zoom, and dragging pans. It's drawn through `filter`, a
// CSS filter, and marked as loading, shown or failed.
//
// A sequence's pages (P52) are images in one world, hidden until shown. A
// new page is shown once it's fully loaded, and only then is the one before
// it hidden, so the view never empties; the zoom and pan stay as they were.
// The next pages are loaded ahead, hidden. `shown` says when a page is drawn.
//
// Overlay layers (ADR-0036, P53b) are drawn over the image shown, placed
// again whenever the view pans, zooms or changes size.
import type OpenSeadragon from 'openseadragon'
import { onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'

import OverlayLayers from '@/overlay/OverlayLayers.vue'
import type { OverlayLayer, Placement } from '@/overlay/types'

type Source = string | { type: 'image'; url: string }

const props = defineProps<{
  /** A DZI file's URL, or images'. */
  sources: Source[]
  /** The image shown, when there are several. */
  page: number
  filter: string
  /** Drawn over the image, in its pixels. */
  layers?: OverlayLayer[]
}>()
const emit = defineEmits<{ shown: [page: number] }>()

/** How many pages after the one shown are loaded ahead. */
const AHEAD = 2

const element = useTemplateRef<HTMLElement>('element')
const status = ref<'loading' | 'shown' | 'failed'>('loading')
/** The page drawn, once one is. */
const drawn = ref<number>()
/** Where the image shown is, in the view's pixels. */
const placement = ref<Placement>()
let viewer: OpenSeadragon.Viewer | undefined
let unmounted = false
/** A sequence's pages in the viewer, loaded or loading: none if one failed. */
const pages = new Map<number, Promise<OpenSeadragon.TiledImage | undefined>>()
/** The page asked for last. */
let wanted = props.page

/** Page `page`'s image, added hidden, once it's fully loaded. */
function load(page: number) {
  let image = pages.get(page)
  if (!image) {
    image = new Promise((resolve) =>
      viewer!.addTiledImage({
        tileSource: props.sources[page]!,
        opacity: 0,
        // Loaded while hidden.
        preload: true,
        success: (event) => {
          const { item } = event as unknown as { item: OpenSeadragon.TiledImage }
          item.whenFullyLoaded(() => resolve(item))
        },
        error: () => resolve(undefined),
      }),
    )
    pages.set(page, image)
  }
  return image
}

/** Shows `page` once it's loaded, then hides the one before. */
async function show(page: number) {
  wanted = page
  const image = await load(page)
  if (unmounted || page !== wanted) return
  if (!image) {
    status.value = 'failed'
    return
  }
  image.setOpacity(1)
  const ahead = (other: number) => other > page && other <= page + AHEAD
  for (const [other, loading] of pages) {
    if (other === page) continue
    void loading.then((item) => {
      if (!item) return
      item.setOpacity(0)
      if (!ahead(other)) viewer?.world.removeItem(item)
    })
    if (!ahead(other)) pages.delete(other)
  }
  for (let next = page + 1; next <= page + AHEAD && next < props.sources.length; next++) {
    void load(next)
  }
  status.value = 'shown'
  drawn.value = page
  place()
  emit('shown', page)
}

/** Measures where the image shown is now. */
function place() {
  const count = viewer?.world.getItemCount() ?? 0
  for (let i = count - 1; i >= 0; i--) {
    const item = viewer!.world.getItemAt(i)
    if (item.getOpacity() === 0) continue
    const { x, y, width, height } = viewer!.viewport.viewportToViewerElementRectangle(
      item.getBounds(true),
    )
    placement.value = { x, y, width, height }
    return
  }
}

onMounted(async () => {
  const { default: openSeadragon } = await import('openseadragon')
  if (unmounted || !element.value) return
  const sequence = props.sources.length > 1
  viewer = openSeadragon({
    element: element.value,
    // Its types leave out images given as objects, which it takes.
    tileSources: (sequence ? [] : props.sources) as OpenSeadragon.Options['tileSources'],
    // The app draws its own controls, which follow the theme.
    showNavigationControl: false,
    showSequenceControl: false,
    // Images from another server can only be drawn with its permission.
    crossOriginPolicy: 'Anonymous',
    visibilityRatio: 1,
  })
  viewer.addHandler('update-viewport', place)
  viewer.addHandler('resize', place)
  if (sequence) {
    void show(props.page)
  } else {
    viewer.addHandler('open', () => {
      status.value = 'shown'
      drawn.value = 0
      place()
    })
    viewer.addHandler('open-failed', () => (status.value = 'failed'))
  }
})
watch(
  () => props.page,
  (page) => viewer && void show(page),
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
  <div class="deep-zoom" :data-status="status" :data-page="drawn">
    <div ref="element" class="deep-zoom__view" :style="{ filter }" />
    <OverlayLayers :layers="layers ?? []" :placement="placement" />
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
