<script setup lang="ts">
// The Slide Scan tab (ADR-0029). Before a scan, the pane chooses its
// workflow, with what it does and its settings, the stitching settings, whose
// title says when they've been changed, and a sample ID. While one runs, wherever it was started, the pane shows its
// details, log and a Cancel that names its phase, and the main view its live
// stitching preview, or the live image until there is one. Once it ends, it
// can be downloaded, and Close goes back to the settings.
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'

import { ApiError } from '@/api/wot/errors'
import { useConnectionStore } from '@/connection/store'
import LiveImage from '@/live/LiveImage.vue'
import MiniStream from '@/live/MiniStream.vue'
import DownloadButton from '@/server-ui/DownloadButton.vue'
import ServerInterface from '@/server-ui/ServerInterface.vue'
import { followProperty } from '@/server-ui/follow'
import ServerPanel from '@/server-ui/ServerPanel.vue'
import type { DownloadButtonElement, UiElement } from '@/server-ui/types'
import AccordionSection from '@/ui/AccordionSection.vue'
import AppAccordion from '@/ui/AppAccordion.vue'
import AppButton from '@/ui/AppButton.vue'
import AppSelect from '@/ui/AppSelect.vue'
import ControlPane from '@/ui/ControlPane.vue'
import FormField from '@/ui/FormField.vue'
import MainView from '@/ui/MainView.vue'
import { useToast } from '@/ui/overlays'
import TextField from '@/ui/TextField.vue'

import { useScan } from './scan'
import ScanInfo from './ScanInfo.vue'

const connection = useConnectionStore()
const toast = useToast()
const scan = useScan((error) =>
  toast.error('The scan didn’t start.', {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  }),
)
const { smartScan, shown, current, running, cancelling, log } = scan
const workflow = ref<string>()
const names = ref<Record<string, string>>({})
const descriptions = ref<Record<string, string>>({})
const sampleId = ref('')
// Closed, so Start Smart Scan shows without scrolling (P47).
const open = ref<string[]>([])
const automatic = ref<boolean>()

watch(
  smartScan,
  async (thing, _, onCleanup) => {
    if (!thing) return
    const follow = (value: unknown) => (automatic.value = value as boolean)
    onCleanup(followProperty(thing.thing, 'stitch_automatically', follow))
    try {
      ;[workflow.value, names.value, descriptions.value, automatic.value] = await Promise.all([
        thing.read('workflow_name'),
        thing.read('workflow_display_names'),
        thing.read('workflow_descriptions'),
        thing.read('stitch_automatically'),
      ])
    } catch (error) {
      toast.error('The workflows couldn’t be read.', { details: error })
    }
  },
  { immediate: true },
)
const stitchingChanged = computed(() => {
  const fallback = smartScan.value?.thing.td.properties?.stitch_automatically?.default
  return automatic.value !== undefined && fallback !== undefined && automatic.value !== fallback
})

const options = computed(() =>
  Object.entries(names.value).map(([value, label]) => ({ value, label })),
)

async function choose(name: string | undefined) {
  if (!name || name === workflow.value) return
  const before = workflow.value
  workflow.value = name
  try {
    await smartScan.value?.write('workflow_name', name)
  } catch (error) {
    workflow.value = before
    toast.error('The workflow couldn’t be chosen.', { details: error })
  }
}

const stitching: UiElement[] = [
  {
    kind: 'property_control',
    thing: 'smart_scan',
    property: 'stitch_automatically',
    label: 'Stitch the scan once it ends',
  },
]

const cancelLabel = computed(() => {
  if (cancelling.value) return 'Cancelling…'
  return current.value?.scan_phase === 'scanning' ? 'Cancel scan' : 'Cancel setup'
})
const outcome = computed(() => {
  const result = current.value?.result
  if (result === 'success') return 'The scan has finished.'
  if (result === 'cancelled') return 'The scan was cancelled.'
  return result ? `The scan stopped: ${result.replace(/^error: /, '')}.` : 'The scan has ended.'
})
const zip = computed<DownloadButtonElement | undefined>(() => {
  const name = current.value?.name
  if (!name || !current.value?.image_count) return undefined
  return {
    kind: 'download_button',
    label: 'Download ZIP',
    source: 'action',
    thing: 'smart_scan',
    action: 'download_zip',
    input: { scan_name: name },
    filename: `${name}.zip`,
  }
})
const preview = computed(() => {
  const time = current.value?.preview_stitch_time
  const client = connection.client
  if (!time || !client) return undefined
  const url = new URL('smart_scan/latest_preview_stitch.jpg', client.baseUrl)
  url.searchParams.set('drawn', time)
  return url.href
})

// The log follows its latest line.
const logList = useTemplateRef<HTMLOListElement>('log')
watch(log, async () => {
  await nextTick()
  if (logList.value) logList.value.scrollTop = logList.value.scrollHeight
})
</script>

<template>
  <div class="slide-scan">
    <ControlPane width="wide">
      <h1 class="visually-hidden">Slide Scan</h1>
      <template v-if="shown">
        <h2 class="slide-scan__heading">{{ running ? 'Scanning' : 'Scan ended' }}</h2>
        <ScanInfo v-if="current" :details="current" />
        <MiniStream v-if="preview" />
        <FormField label="Log">
          <ol ref="log" class="slide-scan__log" aria-live="polite">
            <li v-for="(line, index) in log" :key="index">{{ line }}</li>
          </ol>
        </FormField>
        <AppButton v-if="running" variant="danger" :disabled="cancelling" @click="scan.cancel">
          {{ cancelLabel }}
        </AppButton>
        <template v-else>
          <p class="slide-scan__outcome">{{ outcome }}</p>
          <DownloadButton v-if="zip" :button="zip" />
          <AppButton @click="scan.close">Close</AppButton>
        </template>
      </template>
      <template v-else>
        <FormField label="Workflow">
          <AppSelect :model-value="workflow" :options="options" @update:model-value="choose" />
        </FormField>
        <p v-if="workflow" class="slide-scan__blurb">{{ descriptions[workflow] }}</p>
        <AppAccordion v-model="open">
          <AccordionSection value="settings" title="Scan settings">
            <ServerPanel v-if="workflow" :key="workflow" :thing="workflow" path="settings_ui" />
          </AccordionSection>
          <AccordionSection
            value="stitching"
            :title="stitchingChanged ? 'Stitching settings (changed)' : 'Stitching settings'"
          >
            <ServerInterface :elements="stitching" />
          </AccordionSection>
        </AppAccordion>
        <FormField label="Sample ID" help="Names the scan. Left empty, it’s named by the time.">
          <TextField v-model="sampleId" @submit="scan.start(sampleId)" />
        </FormField>
        <AppButton variant="primary" @click="scan.start(sampleId)">Start Smart Scan</AppButton>
      </template>
    </ControlPane>
    <MainView v-if="preview">
      <img class="slide-scan__preview" :src="preview" alt="The scan's live stitching preview" />
    </MainView>
    <LiveImage v-else />
  </div>
</template>

<style scoped>
.slide-scan {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.slide-scan :deep(.control-pane) {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.slide-scan__heading {
  margin: 0;
  font-size: var(--font-size-lg);
}

.slide-scan__blurb,
.slide-scan__outcome {
  margin: 0;
  color: var(--color-text-muted);
}

.slide-scan__log {
  max-height: 12rem;
  margin: 0;
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-6);
  overflow-y: auto;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--font-size-sm);
}

.slide-scan__preview {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
</style>
