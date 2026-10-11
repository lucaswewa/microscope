<script setup lang="ts">
// The Settings tab: its sections listed in the control pane, by Application
// and Microscope, and the chosen one beside them. The section is in the
// address (`/#/settings/camera`); without one, or with one it doesn't know,
// it's Display. The calibration wizard's button waits for P29.
import { computed, type Component } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

import ConnectView from '@/connection/ConnectView.vue'
import NavigationSection from '@/control/NavigationSection.vue'
import CameraSettings from '@/settings/CameraSettings.vue'
import DisplaySettings from '@/settings/DisplaySettings.vue'
import MappingSettings from '@/settings/MappingSettings.vue'
import StageSettings from '@/settings/StageSettings.vue'
import AppButton from '@/ui/AppButton.vue'
import ControlPane from '@/ui/ControlPane.vue'
import MainView from '@/ui/MainView.vue'
import SectionHeading from '@/ui/SectionHeading.vue'

interface Section {
  id: string
  title: string
  component: Component
  props?: Record<string, unknown>
}

const GROUPS: { title: string; sections: Section[] }[] = [
  {
    title: 'Application',
    sections: [
      { id: 'display', title: 'Display', component: DisplaySettings },
      { id: 'stage-control', title: 'Stage control', component: NavigationSection },
      { id: 'connection', title: 'Connection', component: ConnectView, props: { embedded: true } },
    ],
  },
  {
    title: 'Microscope',
    sections: [
      { id: 'camera', title: 'Camera', component: CameraSettings },
      { id: 'stage', title: 'Stage', component: StageSettings },
      { id: 'camera-stage-mapping', title: 'Camera–stage mapping', component: MappingSettings },
    ],
  },
]

const route = useRoute()
const section = computed(() => {
  const sections = GROUPS.flatMap((group) => group.sections)
  return sections.find((s) => s.id === route.params.section) ?? sections[0]!
})
</script>

<template>
  <div class="settings-page">
    <ControlPane width="narrow">
      <h1 class="visually-hidden">Settings</h1>
      <nav aria-label="Settings sections" class="settings-page__nav">
        <template v-for="group in GROUPS" :key="group.title">
          <h2 class="settings-page__group">{{ group.title }}</h2>
          <AppButton v-if="group.title === 'Microscope'" size="sm" disabled>
            Calibration wizard
          </AppButton>
          <ul class="settings-page__list">
            <li v-for="item in group.sections" :key="item.id">
              <RouterLink
                :to="{ name: 'settings', params: { section: item.id } }"
                class="settings-page__link"
                :aria-current="item.id === section.id ? 'page' : undefined"
              >
                {{ item.title }}
              </RouterLink>
            </li>
          </ul>
        </template>
      </nav>
    </ControlPane>
    <MainView>
      <div class="settings-page__section">
        <SectionHeading>{{ section.title }}</SectionHeading>
        <component :is="section.component" :key="section.id" v-bind="section.props" />
      </div>
    </MainView>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.settings-page__nav {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.settings-page__group {
  margin: var(--space-3) 0 0;
  color: var(--color-text-muted);
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-sm);
  text-transform: uppercase;
}

.settings-page__list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.settings-page__link {
  display: block;
  padding: var(--space-2) var(--space-3);
  color: var(--color-text);
  text-decoration: none;
  border-radius: var(--radius-sm);
}

.settings-page__link:hover {
  background: var(--color-accent-subtle);
}

.settings-page__link[aria-current='page'] {
  color: var(--color-accent-text);
  font-weight: var(--font-weight-medium);
  background: var(--color-accent-subtle);
}

.settings-page__section {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-6);
  overflow: auto;
}
</style>
