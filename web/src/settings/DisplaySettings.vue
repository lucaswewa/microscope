<script setup lang="ts">
// Settings, Display: the theme, the accent colour (ADR-0047), filling the
// screen, and turning the live image off.
import { onBeforeUnmount, ref } from 'vue'

import { useHost } from '@/host'
import { useLiveImagePreferences } from '@/live/preferences'
import { ACCENTS, SETTINGS_ACCENTS, useAccentStore } from '@/theme/accents'
import { useThemeStore, type ThemePreference } from '@/theme/store'
import AppSelect from '@/ui/AppSelect.vue'
import AppToggle from '@/ui/AppToggle.vue'
import FormField from '@/ui/FormField.vue'

const theme = useThemeStore()
const accent = useAccentStore()
const accents = SETTINGS_ACCENTS.flatMap(({ group, ids }) =>
  ids.map((id) => {
    const { label, hex } = ACCENTS.find((accent) => accent.id === id)!
    return { value: id, label, group, swatch: hex }
  }),
)
const live = useLiveImagePreferences()
const host = useHost()
const themes: { value: ThemePreference; label: string }[] = [
  { value: 'system', label: 'As the system is' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
]

const fullscreen = ref(host.fullscreen.active())
onBeforeUnmount(host.fullscreen.watch((on) => (fullscreen.value = on)))

async function setFullscreen(on: boolean) {
  try {
    await host.fullscreen.set(on)
  } finally {
    fullscreen.value = host.fullscreen.active()
  }
}
</script>

<template>
  <div class="display-settings">
    <FormField label="Theme">
      <AppSelect
        :model-value="theme.preference"
        :options="themes"
        @update:model-value="(choice) => choice && theme.setPreference(choice)"
      />
    </FormField>
    <FormField label="Accent colour">
      <AppSelect
        :model-value="accent.accent"
        :options="accents"
        @update:model-value="(choice) => choice && accent.choose(choice)"
      />
    </FormField>
    <AppToggle :model-value="fullscreen" label="Full screen" @update:model-value="setFullscreen" />
    <AppToggle v-model="live.streamDisabled" label="Turn the live image off" />
  </div>
</template>

<style scoped>
.display-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  max-width: 20rem;
}
</style>
