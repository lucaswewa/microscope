<script setup lang="ts">
// Every UI component in both themes, side by side, sharing their state. For
// development only: the route exists only in development builds
// (src/router/index.ts).
import add from '@material-symbols/svg-400/outlined/add.svg?raw'
import arrowUpward from '@material-symbols/svg-400/outlined/arrow_upward.svg?raw'
import photoCamera from '@material-symbols/svg-400/outlined/photo_camera.svg?raw'
import refresh from '@material-symbols/svg-400/outlined/refresh.svg?raw'
import { computed, ref } from 'vue'

import AccordionSection from '@/ui/AccordionSection.vue'
import AppAccordion from '@/ui/AppAccordion.vue'
import AppButton from '@/ui/AppButton.vue'
import AppCard from '@/ui/AppCard.vue'
import AppCheckbox from '@/ui/AppCheckbox.vue'
import AppSelect from '@/ui/AppSelect.vue'
import AppSpinner from '@/ui/AppSpinner.vue'
import AppToggle from '@/ui/AppToggle.vue'
import FormField from '@/ui/FormField.vue'
import IconButton from '@/ui/IconButton.vue'
import NumberField from '@/ui/NumberField.vue'
import ProgressBar from '@/ui/ProgressBar.vue'
import SectionHeading from '@/ui/SectionHeading.vue'
import TextField from '@/ui/TextField.vue'

const themes = ['light', 'dark'] as const
const variants = ['default', 'primary', 'danger', 'ghost'] as const
const sizes = ['md', 'sm'] as const

const name = ref('')
const exposure = ref(5000)
const stepSize = ref(200)
const resolution = ref<string>()
const resolutions = [
  { value: '820x616', label: '820 × 616' },
  { value: '1640x1232', label: '1640 × 1232' },
  { value: '3280x2464', label: '3280 × 2464 (full)' },
  { value: 'video', label: 'Video only', disabled: true },
]
const saveToGallery = ref(true)
const autoExposure = ref(false)
const openSections = ref(['move'])
const progress = ref(35)
const lastSubmit = ref('none')

const state = computed(() =>
  JSON.stringify({
    name: name.value,
    exposure: exposure.value,
    stepSize: stepSize.value,
    resolution: resolution.value ?? null,
    saveToGallery: saveToGallery.value,
    autoExposure: autoExposure.value,
    openSections: openSections.value,
  }),
)
</script>

<template>
  <main class="gallery">
    <h1>Components</h1>
    <p class="gallery__note">
      For development only. Each panel forces its theme; both share their state.
    </p>

    <div class="gallery__panels">
      <section v-for="theme in themes" :key="theme" :data-theme="theme" class="gallery__panel">
        <SectionHeading>
          {{ theme === 'light' ? 'Light' : 'Dark' }}
          <template #actions>
            <IconButton :icon="refresh" label="Refresh" variant="ghost" />
          </template>
        </SectionHeading>

        <h3>Buttons</h3>
        <div v-for="size in sizes" :key="size" class="gallery__row">
          <AppButton v-for="variant in variants" :key="variant" :variant="variant" :size="size">
            {{ variant[0]!.toUpperCase() + variant.slice(1) }}
          </AppButton>
          <AppButton :size="size" :icon="photoCamera" variant="primary">Capture</AppButton>
          <AppButton :size="size" disabled>Disabled</AppButton>
          <IconButton :icon="arrowUpward" label="Move up" :size="size" />
          <IconButton :icon="add" label="Add" :size="size" variant="primary" />
        </div>

        <h3>Fields</h3>
        <div class="gallery__fields">
          <FormField label="Sample name" help="Press Enter to submit.">
            <TextField
              v-model="name"
              placeholder="Untitled"
              @submit="lastSubmit = `name ${JSON.stringify($event)}`"
            />
          </FormField>
          <FormField label="Exposure (µs)" help="1–100 000, whole numbers. ↑ and ↓ step.">
            <NumberField
              v-model="exposure"
              :min="1"
              :max="100000"
              :step="1"
              @submit="lastSubmit = `exposure ${$event}`"
            />
          </FormField>
          <FormField
            label="Step size (µm)"
            :error="stepSize > 500 ? 'Larger than the field of view.' : undefined"
          >
            <NumberField v-model="stepSize" :min="0" :step="0.5" size="sm" />
          </FormField>
          <FormField label="Resolution">
            <AppSelect v-model="resolution" :options="resolutions" placeholder="Choose…" />
          </FormField>
          <FormField label="Disabled">
            <TextField model-value="Read only" disabled />
          </FormField>
        </div>
        <div class="gallery__row">
          <AppCheckbox v-model="saveToGallery" label="Save to gallery" />
          <AppCheckbox :model-value="true" label="Disabled" disabled />
          <AppToggle v-model="autoExposure" label="Auto exposure" />
          <AppToggle :model-value="false" label="Disabled" disabled />
        </div>

        <h3>Accordion</h3>
        <AppAccordion v-model="openSections">
          <AccordionSection value="move" title="Move">
            <AppButton size="sm">Move to centre</AppButton>
          </AccordionSection>
          <AccordionSection value="capture" title="Capture">
            <AppCheckbox v-model="saveToGallery" label="Save to gallery" />
          </AccordionSection>
          <AccordionSection value="disabled" title="Disabled" disabled>Hidden</AccordionSection>
        </AppAccordion>

        <h3>Progress</h3>
        <div class="gallery__progress">
          <ProgressBar :value="progress" label="Scan progress" />
          <input
            v-model.number="progress"
            type="range"
            min="0"
            max="100"
            aria-label="Progress"
            class="gallery__range"
          />
          <ProgressBar label="Autofocusing" />
          <div class="gallery__row">
            <AppSpinner label="Loading" />
            <AppSpinner size="var(--icon-size-sm)" class="gallery__accent" />
            <AppButton variant="primary" disabled>
              <AppSpinner size="var(--icon-size-sm)" /> Scanning…
            </AppButton>
          </div>
        </div>

        <h3>Card</h3>
        <AppCard>
          <SectionHeading :level="3">Capture</SectionHeading>
          <p class="gallery__note">A card holds a block of related content.</p>
        </AppCard>
      </section>
    </div>

    <p class="gallery__state"><strong>Last submit:</strong> {{ lastSubmit }}</p>
    <p class="gallery__state"><strong>State:</strong> {{ state }}</p>
  </main>
</template>

<style scoped>
.gallery {
  flex: 1;
  padding: var(--space-6);
  overflow: auto;
}

.gallery__note {
  color: var(--color-text-muted);
}

.gallery__panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-6);
  margin: var(--space-6) 0;
}

.gallery__panel {
  padding: var(--space-6);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-panel);
  color: var(--color-text);
}

.gallery__row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  align-items: center;
  margin-bottom: var(--space-4);
}

.gallery__fields {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--space-4);
  margin-bottom: var(--space-4);
}

.gallery__progress {
  display: grid;
  gap: var(--space-3);
}

.gallery__accent {
  color: var(--color-accent-text);
}

.gallery__range {
  accent-color: var(--color-accent);
}

.gallery__state {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}
</style>
