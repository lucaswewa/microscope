<script setup lang="ts">
// The Logging tab: the server's log (P03), newest first, in groups of related
// records, 10 groups to a page. Each group shows its most severe level, what
// it's from, when it began and how many records it has, and opens to show
// them. A level hides less severe records, INFO by default. The log is read
// when the tab opens and on Refresh, and today's log file can be downloaded.
import downloadIcon from '@material-symbols/svg-400/outlined/download.svg?raw'
import chevron from '@material-symbols/svg-400/outlined/keyboard_arrow_down.svg?raw'
import refreshIcon from '@material-symbols/svg-400/outlined/refresh.svg?raw'
import { computed, reactive, ref, shallowRef, watch } from 'vue'

import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import AppButton from '@/ui/AppButton.vue'
import AppIcon from '@/ui/AppIcon.vue'
import AppPagination from '@/ui/AppPagination.vue'
import AppSelect from '@/ui/AppSelect.vue'
import FormField from '@/ui/FormField.vue'
import MainView from '@/ui/MainView.vue'
import { useToast } from '@/ui/overlays'

import {
  filterGroups,
  groupRecords,
  highest,
  LEVELS,
  type LogGroup,
  type ServerLogRecord,
} from './log'

const PER_PAGE = 10
const LEVEL_OPTIONS = LEVELS.map(({ name, levelno }) => ({ value: levelno, label: name }))

const connection = useConnectionStore()
const host = useHost()
const toast = useToast()

const records = shallowRef<ServerLogRecord[]>([])
const loaded = ref(false)
const loading = ref(false)
const level = ref(20)
const page = ref(1)
const open = reactive(new Set<string>())

const report = (message: string) => (error: unknown) =>
  toast.error(message, {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  })

/** Reads the log again. */
async function load() {
  const client = connection.client
  if (!client) return
  loading.value = true
  try {
    records.value = (await client.request(new URL('log/', client.baseUrl))) as ServerLogRecord[]
    loaded.value = true
  } catch (error) {
    report("Couldn't read the log.")(error)
  } finally {
    loading.value = false
  }
}
watch(() => connection.client, load, { immediate: true })

const groups = computed(() => filterGroups(groupRecords(records.value), level.value).reverse())
const onPage = computed(() =>
  groups.value.slice((page.value - 1) * PER_PAGE, page.value * PER_PAGE),
)
watch(level, () => (page.value = 1))
// After a refresh with fewer groups, the last page there is.
watch(
  groups,
  (list) => (page.value = Math.min(page.value, Math.max(1, Math.ceil(list.length / PER_PAGE)))),
)
const levelName = computed(() => LEVELS.find((l) => l.levelno === level.value)?.name)

const severity = (record: ServerLogRecord) => record.levelname.toLowerCase()
const time = (record: ServerLogRecord) => new Date(record.created).toLocaleTimeString()
const began = (group: LogGroup) => new Date(group.records[0]!.created).toLocaleString()
const count = (group: LogGroup) =>
  group.records.length === 1 ? '1 record' : `${group.records.length} records`

function toggled(group: LogGroup, event: Event) {
  if ((event.target as HTMLDetailsElement).open) open.add(group.key)
  else open.delete(group.key)
}
function setAll(opened: boolean) {
  for (const group of onPage.value) {
    if (opened) open.add(group.key)
    else open.delete(group.key)
  }
}

async function download() {
  const client = connection.client
  if (!client) return
  // The server names its files by the date in UTC.
  const name = `microscope.${new Date().toISOString().slice(0, 10)}.log`
  try {
    await host.saveFile(name, await client.download(new URL('logfile/', client.baseUrl)))
  } catch (error) {
    report("Couldn't download the log file.")(error)
  }
}
</script>

<template>
  <MainView class="logging-page">
    <h1 class="visually-hidden">Logging</h1>
    <div class="logging-page__bar">
      <FormField label="Level" class="logging-page__level">
        <AppSelect v-model="level" :options="LEVEL_OPTIONS" />
      </FormField>
      <AppButton :disabled="!onPage.length" @click="setAll(true)">Expand all</AppButton>
      <AppButton :disabled="!onPage.length" @click="setAll(false)">Collapse all</AppButton>
      <AppButton :icon="refreshIcon" :disabled="loading" @click="load">Refresh</AppButton>
      <AppButton :icon="downloadIcon" variant="primary" @click="download">
        Download log file
      </AppButton>
    </div>
    <p v-if="loaded && !records.length" class="logging-page__empty">The log is empty.</p>
    <p v-else-if="loaded && !groups.length" class="logging-page__empty">
      Nothing at {{ levelName }} or above. Choose a lower level to see more.
    </p>
    <details
      v-for="group in onPage"
      :key="group.key"
      class="log-group"
      :open="open.has(group.key)"
      @toggle="toggled(group, $event)"
    >
      <summary class="log-group__summary">
        <span class="log-level" :class="`log-level--${severity(highest(group))}`">
          {{ highest(group).levelname }}
        </span>
        <span class="log-group__title">{{ group.title }}</span>
        <span class="log-group__meta">{{ began(group) }} · {{ count(group) }}</span>
        <AppIcon :svg="chevron" size="var(--icon-size-sm)" class="log-group__chevron" />
      </summary>
      <ol class="log-group__records">
        <li v-for="record in group.records" :key="record.sequence" class="log-record">
          <time :datetime="record.created">{{ time(record) }}</time>
          <span class="log-level" :class="`log-level--${severity(record)}`">
            {{ record.levelname }}
          </span>
          <span class="log-record__message">
            {{ record.message }}
            <span v-if="group.invocation" class="log-record__source">{{ record.name }}</span>
          </span>
          <pre v-if="record.exception_type || record.traceback" class="log-record__error">{{
            [record.exception_type, record.traceback].filter(Boolean).join('\n')
          }}</pre>
        </li>
      </ol>
    </details>
    <AppPagination
      v-if="groups.length > PER_PAGE"
      v-model:page="page"
      :total="groups.length"
      :per-page="PER_PAGE"
    />
  </MainView>
</template>

<style scoped>
.logging-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.logging-page__bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: flex-end;
}

.logging-page__level {
  width: 10rem;
  margin-right: auto;
}

.logging-page__empty {
  margin: 0;
  color: var(--color-text-muted);
}

.log-group {
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-panel);
}

.log-group__summary {
  display: flex;
  gap: var(--space-3);
  align-items: baseline;
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
}

.log-group[open] .log-group__chevron {
  transform: rotate(180deg);
}

.log-group__summary:focus-visible {
  outline: var(--focus-ring-width) solid var(--color-focus);
  outline-offset: var(--focus-ring-offset);
}

.log-group__title {
  flex: 1;
  overflow-wrap: anywhere;
  font-weight: var(--font-weight-semibold);
}

.log-group__meta,
.log-record time,
.log-record__source {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.log-group__records {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: var(--space-2) var(--space-3) var(--space-3);
  border-top: var(--border-width) solid var(--color-border);
  list-style: none;
}

.log-record {
  display: grid;
  grid-template-columns: auto 6rem 1fr;
  gap: var(--space-3);
  align-items: baseline;
}

.log-record__message {
  overflow-wrap: anywhere;
}

.log-record__source {
  margin-left: var(--space-2);
}

.log-record__error {
  grid-column: 2 / -1;
  margin: 0;
  overflow-x: auto;
  font-size: var(--font-size-sm);
  white-space: pre-wrap;
}

.log-level {
  width: 6rem;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.log-level--debug {
  color: var(--color-text-muted);
}

.log-level--warning {
  color: var(--color-warning);
}

.log-level--error,
.log-level--critical {
  color: var(--color-danger);
}
</style>
