<script setup lang="ts">
// One item in the gallery (ADR-0026): its thumbnail, or a placeholder if it
// has none or it won't load, which opens the viewer; its name and when it was
// made. Unless only thumbnails are shown, then: its download (a button, or a
// menu when there are several), its actions, View and Delete, which asks
// first; its info lines; and its warnings and alerts.
import downloadIcon from '@material-symbols/svg-400/outlined/download.svg?raw'
import alertIcon from '@material-symbols/svg-400/outlined/error.svg?raw'
import imageIcon from '@material-symbols/svg-400/outlined/image.svg?raw'
import warningIcon from '@material-symbols/svg-400/outlined/warning.svg?raw'
import { computed, ref } from 'vue'

import type { GalleryEntry } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import ActionButton from '@/server-ui/ActionButton.vue'
import DownloadButton from '@/server-ui/DownloadButton.vue'
import { dataUrl, useDownload } from '@/server-ui/download'
import type { ActionButtonElement, DownloadButtonElement } from '@/server-ui/types'
import AppButton from '@/ui/AppButton.vue'
import AppCard from '@/ui/AppCard.vue'
import AppIcon from '@/ui/AppIcon.vue'
import ButtonMenu from '@/ui/ButtonMenu.vue'
import { useConfirm, useToast } from '@/ui/overlays'

const props = defineProps<{ entry: GalleryEntry; thumbnailsOnly?: boolean }>()
const emit = defineEmits<{ view: []; changed: [] }>()
const connection = useConnectionStore()
const downloads = useDownload()
const confirm = useConfirm()
const toast = useToast()

const unloadable = ref(false)
const thumbnail = computed(() => {
  const { thumbnail: path } = props.entry
  const base = connection.client?.baseUrl
  return path && base && !unloadable.value ? dataUrl(path, base).href : undefined
})
const created = computed(() => new Date(props.entry.created).toLocaleString())
// The gallery's buttons are those of panels (ADR-0024), without their kind.
const buttons = computed(() =>
  props.entry.download_buttons.map(
    (button) => ({ kind: 'download_button', ...button }) as DownloadButtonElement,
  ),
)
const menu = computed(() =>
  buttons.value.map((button) => ({ label: button.label, run: () => void downloads.save(button) })),
)
const actions = computed(() =>
  props.entry.card_actions.map(
    (action) => ({ kind: 'action_button', ...action }) as ActionButtonElement,
  ),
)

async function remove() {
  const client = connection.client
  const { name, delete_endpoint: endpoint } = props.entry
  const message = `Delete ${name} from the microscope? This can't be undone.`
  if (
    !client ||
    !(await confirm({ title: 'Delete', message, confirmLabel: 'Delete', danger: true }))
  )
    return
  try {
    await client.request(new URL(endpoint, client.baseUrl), { method: 'DELETE' })
  } catch (error) {
    toast.error(`Couldn't delete ${name}.`, {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
  }
  emit('changed')
}
</script>

<template>
  <AppCard class="gallery-card">
    <button
      type="button"
      class="gallery-card__thumbnail"
      :aria-label="`View ${entry.name}`"
      @click="emit('view')"
    >
      <img v-if="thumbnail" :src="thumbnail" alt="" @error="unloadable = true" />
      <AppIcon v-else :svg="imageIcon" size="var(--space-12)" />
    </button>
    <h2 class="gallery-card__title">{{ entry.name }}</h2>
    <p class="gallery-card__created">{{ created }}</p>
    <template v-if="!thumbnailsOnly">
      <div class="gallery-card__buttons">
        <DownloadButton v-if="buttons.length === 1" :button="buttons[0]!" />
        <ButtonMenu
          v-else-if="buttons.length"
          label="Download"
          :icon="downloadIcon"
          :items="menu"
          variant="primary"
          class="gallery-card__download"
        />
        <ActionButton
          v-for="action in actions"
          :key="action.action"
          :button="action"
          @refresh="emit('changed')"
        />
        <AppButton @click="emit('view')">View</AppButton>
        <AppButton @click="remove">Delete</AppButton>
      </div>
      <dl v-if="entry.info.length" class="gallery-card__info">
        <div v-for="line in entry.info" :key="line.label">
          <dt>{{ line.label }}</dt>
          <dd>{{ line.value }}</dd>
        </div>
      </dl>
      <ul v-if="entry.card_messages.length" class="gallery-card__messages">
        <li
          v-for="message in entry.card_messages"
          :key="message.message"
          :class="`gallery-card__message--${message.level}`"
        >
          <AppIcon
            :svg="message.level === 'alert' ? alertIcon : warningIcon"
            size="var(--icon-size-sm)"
          />
          {{ message.message }}
        </li>
      </ul>
    </template>
  </AppCard>
</template>

<style scoped>
.gallery-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  height: 100%;
}

.gallery-card__thumbnail {
  display: grid;
  place-items: center;
  aspect-ratio: 4 / 3;
  padding: 0;
  overflow: hidden;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--color-surface-sunken);
  color: var(--color-text-muted);
  cursor: pointer;
}

.gallery-card__thumbnail img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.gallery-card__thumbnail:focus-visible {
  outline: var(--focus-ring-width) solid var(--color-focus);
  outline-offset: var(--focus-ring-offset);
}

.gallery-card__title {
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--color-accent-text);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-light);
  text-align: center;
}

.gallery-card__created,
.gallery-card__info,
.gallery-card__messages {
  margin: 0;
  font-size: var(--font-size-sm);
}

.gallery-card__created {
  color: var(--color-text-muted);
  text-align: center;
}

.gallery-card__buttons {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.gallery-card__download {
  display: flex;
}

.gallery-card__download :deep(button) {
  flex: 1;
}

.gallery-card__info div {
  display: flex;
  gap: var(--space-1);
}

.gallery-card__info dt::after {
  content: ':';
}

.gallery-card__info dd {
  margin: 0;
}

.gallery-card__messages {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: 0;
  list-style: none;
}

.gallery-card__messages li {
  display: flex;
  gap: var(--space-1);
  align-items: flex-start;
}

.gallery-card__message--warning {
  color: var(--color-warning);
}

.gallery-card__message--alert {
  color: var(--color-danger);
}
</style>
