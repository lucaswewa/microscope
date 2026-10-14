<script setup lang="ts">
// The Gallery tab: everything the microscope has saved, newest first, as
// cards (ADR-0026), detailed or thumbnails only, 18 or 25 to a page. A
// sidebar, which can be hidden, chooses the types shown and the cards per
// page, runs the bulk actions, and deletes everything of the types shown.
// The gallery is read again when the tab opens, on Refresh, and after
// anything that changes it. View opens the viewer (ADR-0027), as does a link
// to an item, `/#/gallery?view=<its path>`.
import detailsIcon from '@material-symbols/svg-400/outlined/view_agenda.svg?raw'
import refreshIcon from '@material-symbols/svg-400/outlined/refresh.svg?raw'
import closeIcon from '@material-symbols/svg-400/outlined/right_panel_close.svg?raw'
import openIcon from '@material-symbols/svg-400/outlined/right_panel_open.svg?raw'
import thumbnailsIcon from '@material-symbols/svg-400/outlined/grid_view.svg?raw'
import { computed, h, ref, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { type GalleryEntry, useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import ActionButton from '@/server-ui/ActionButton.vue'
import type { ActionButtonElement } from '@/server-ui/types'
import AppButton from '@/ui/AppButton.vue'
import AppPagination from '@/ui/AppPagination.vue'
import AppSelect from '@/ui/AppSelect.vue'
import FormField from '@/ui/FormField.vue'
import IconButton from '@/ui/IconButton.vue'
import MainView from '@/ui/MainView.vue'
import MultiSelect from '@/ui/MultiSelect.vue'
import { useConfirm, useToast } from '@/ui/overlays'

import GalleryCard from './GalleryCard.vue'
import ImageViewer from './ImageViewer.vue'

const PER_PAGE = [18, 25].map((n) => ({ value: n, label: String(n) }))

const gallery = useThing('gallery')
const route = useRoute()
const router = useRouter()
const confirm = useConfirm()
const toast = useToast()

const entries = shallowRef<GalleryEntry[]>([])
const cardTypes = ref<string[]>([])
const bulkActions = shallowRef<ActionButtonElement[]>([])
const shown = ref<string[]>([])
const loaded = ref(false)
const loading = ref(false)
const page = ref(1)
const perPage = ref(18)
const thumbnailsOnly = ref(false)
const sidebar = ref(false)
const viewing = shallowRef<GalleryEntry>()

const report = (message: string) => (error: unknown) =>
  toast.error(message, {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  })

/** Reads the gallery again. Types it hadn't seen before are shown. */
async function load() {
  const thing = gallery.value
  if (!thing) return
  loading.value = true
  try {
    const [list, types, bulk] = await Promise.all([
      thing.read('list_data'),
      thing.read('card_types'),
      thing.read('bulk_actions'),
    ])
    const known = cardTypes.value
    shown.value = [
      ...shown.value.filter((type) => types.includes(type)),
      ...types.filter((type) => !known.includes(type)),
    ]
    cardTypes.value = types
    bulkActions.value = bulk.map((b) => ({ kind: 'action_button', ...b }) as ActionButtonElement)
    entries.value = list
    loaded.value = true
  } catch (error) {
    report("Couldn't read the gallery.")(error)
  } finally {
    loading.value = false
  }
}
watch(gallery, () => void load(), { immediate: true })
watch([entries, () => route.query.view], ([list, path]) => {
  const linked = list.find((entry) => entry.path === path)
  if (linked) viewing.value = linked
})
function closeViewer() {
  viewing.value = undefined
  if (route.query.view) void router.replace({ query: {} })
}

const filtered = computed(() => entries.value.filter((e) => shown.value.includes(e.card_type)))
const first = computed(() => (page.value - 1) * perPage.value)
const onPage = computed(() => filtered.value.slice(first.value, first.value + perPage.value))
const showing = computed(() => {
  const { length } = filtered.value
  return length
    ? `Showing ${first.value + 1}–${first.value + onPage.value.length} of ${length}`
    : ''
})
watch([shown, perPage], () => (page.value = 1))
// After deleting the last card on the last page, the page before it.
watch(filtered, (list) => {
  page.value = Math.min(page.value, Math.max(1, Math.ceil(list.length / perPage.value)))
})

async function deleteAll() {
  const thing = gallery.value
  const types = [...shown.value]
  const asked = await confirm({
    title: 'Delete all',
    message: "Delete everything of these types from the microscope? This can't be undone.",
    content: () =>
      h(
        'ul',
        types.map((type) => h('li', type)),
      ),
    confirmLabel: 'Delete all',
    danger: true,
  })
  if (!thing || !asked) return
  await thing.run('delete_all_data', { card_types: types }).catch(report("Couldn't delete it all."))
  await load()
}
</script>

<template>
  <div class="gallery-page">
    <MainView class="gallery-page__main">
      <h1 class="visually-hidden">Gallery</h1>
      <div class="gallery-page__bar">
        <p class="gallery-page__showing">{{ showing }}</p>
        <IconButton
          :icon="thumbnailsOnly ? detailsIcon : thumbnailsIcon"
          :label="thumbnailsOnly ? 'Show details' : 'Show thumbnails only'"
          @click="thumbnailsOnly = !thumbnailsOnly"
        />
        <AppButton :icon="refreshIcon" :disabled="loading" @click="load">Refresh</AppButton>
        <IconButton
          :icon="sidebar ? closeIcon : openIcon"
          :label="sidebar ? 'Hide the sidebar' : 'Show the sidebar'"
          :aria-expanded="sidebar"
          aria-controls="gallery-sidebar"
          @click="sidebar = !sidebar"
        />
      </div>
      <p v-if="loaded && !entries.length" class="gallery-page__empty">
        Nothing has been saved yet. Captures saved to the gallery appear here.
      </p>
      <p v-else-if="loaded && !filtered.length" class="gallery-page__empty">
        Nothing of the types shown. Choose more in the sidebar.
      </p>
      <ul class="gallery-page__grid" :class="{ 'gallery-page__grid--thumbnails': thumbnailsOnly }">
        <li v-for="entry in onPage" :key="entry.path">
          <GalleryCard
            :entry="entry"
            :thumbnails-only="thumbnailsOnly"
            @view="viewing = entry"
            @changed="load"
          />
        </li>
      </ul>
      <AppPagination
        v-if="filtered.length > perPage"
        v-model:page="page"
        :total="filtered.length"
        :per-page="perPage"
      />
    </MainView>
    <aside v-if="sidebar" id="gallery-sidebar" class="gallery-page__sidebar" aria-label="Gallery">
      <FormField label="Show">
        <MultiSelect
          v-model="shown"
          :options="cardTypes.map((type) => ({ value: type, label: type }))"
          placeholder="Nothing"
        />
      </FormField>
      <FormField label="Cards per page">
        <AppSelect v-model="perPage" :options="PER_PAGE" />
      </FormField>
      <ActionButton
        v-for="action in bulkActions"
        :key="`${action.thing}/${action.action}`"
        :button="action"
        @refresh="load"
      />
      <AppButton variant="danger" :disabled="!shown.length" @click="deleteAll">
        Delete all
      </AppButton>
    </aside>
    <ImageViewer v-if="viewing" :entry="viewing" @close="closeViewer" />
  </div>
</template>

<style scoped>
.gallery-page {
  display: flex;
  flex: 1;
  min-width: 0;
  height: 100%;
}

.gallery-page__main {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.gallery-page__bar {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.gallery-page__showing {
  flex: 1;
  margin: 0;
}

.gallery-page__empty {
  margin: 0;
  color: var(--color-text-muted);
}

.gallery-page__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, 17rem);
  gap: var(--space-6);
  margin: 0;
  padding: 0;
  list-style: none;
}

.gallery-page__grid--thumbnails {
  grid-template-columns: repeat(auto-fill, 12rem);
  gap: var(--space-4);
}

.gallery-page__sidebar {
  display: flex;
  flex: none;
  flex-direction: column;
  gap: var(--space-4);
  width: var(--pane-width-wide);
  height: 100%;
  padding: var(--space-4);
  overflow: hidden auto;
  border-left: var(--border-width) solid var(--color-border);
  background: var(--color-surface-panel);
}
</style>
