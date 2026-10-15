<script setup lang="ts">
// The About tab (P46): the microscope's computer, address, server and its
// version, and whether the launcher manages it (ADR-0033); its camera, stage
// and illumination, by their Things' types, with a button that flashes the
// illumination to find which microscope this is; and what the software is
// for, with links to its source and its issues.
import { computed, ref, watch } from 'vue'

import { useThing } from '@/api/things'
import { apiRoot, displayOrigin } from '@/connection/profiles'
import { useConnectionStore } from '@/connection/store'
import { useHost } from '@/host'
import ActionButton from '@/server-ui/ActionButton.vue'
import type { ActionButtonElement } from '@/server-ui/types'
import MainView from '@/ui/MainView.vue'
import SectionHeading from '@/ui/SectionHeading.vue'

const REPOSITORY = 'https://github.com/lucaswewa/microscope'

const connection = useConnectionStore()
const system = useThing('system')
const host = useHost()

const server = ref<{ version: string; os: string; managed: boolean }>()
watch(
  system,
  async (thing) => {
    if (!thing) return
    const [version, os, managed] = await Promise.all([
      thing.read('version_data'),
      thing.read('os_version'),
      thing.read('managed'),
    ]).catch(() => [])
    if (version) {
      server.value = { version: `${version.version} (${version.version_source})`, os, managed }
    }
  },
  { immediate: true },
)

const facts = computed(() => [
  ['Computer', connection.hostname ?? '…'],
  ['Address', connection.profile ? displayOrigin(apiRoot(connection.profile).origin) : '…'],
  ['Server version', server.value?.version ?? '…'],
  ['Operating system', server.value?.os ?? '…'],
  [
    'Managed by the launcher',
    server.value ? (server.value.managed ? 'Yes: it can restart' : 'No') : '…',
  ],
  ['App version', host.info.version],
])
const hardware = computed(() =>
  (['camera', 'stage', 'illumination'] as const).map((thing) => [
    thing[0]!.toUpperCase() + thing.slice(1),
    connection.descriptions?.[thing]?.title ?? 'None',
  ]),
)
const flash: ActionButtonElement = {
  kind: 'action_button',
  thing: 'illumination',
  action: 'flash',
  label: 'Flash illumination',
}

function open(event: MouseEvent, url: string) {
  event.preventDefault()
  host.openLink(url)
}
</script>

<template>
  <MainView class="about-page">
    <h1 class="visually-hidden">About</h1>
    <section>
      <SectionHeading>This microscope</SectionHeading>
      <dl class="about-page__facts">
        <template v-for="[label, value] in facts" :key="label">
          <dt>{{ label }}</dt>
          <dd>{{ value }}</dd>
        </template>
      </dl>
    </section>
    <section>
      <SectionHeading>Hardware</SectionHeading>
      <dl class="about-page__facts">
        <template v-for="[label, value] in hardware" :key="label">
          <dt>{{ label }}</dt>
          <dd>{{ value }}</dd>
        </template>
      </dl>
      <p class="about-page__note">
        Flashing the illumination shows which microscope this is, among several.
      </p>
      <ActionButton v-if="connection.isAvailable('illumination')" :button="flash" />
    </section>
    <section>
      <SectionHeading>Intended use</SectionHeading>
      <p>
        This software is for research and education. It isn’t a medical device, and mustn’t be used
        to diagnose, treat or prevent disease, or to make decisions about anyone’s care.
      </p>
      <p class="about-page__links">
        <a :href="REPOSITORY" @click="open($event, REPOSITORY)">Source code</a>
        <a :href="`${REPOSITORY}/issues`" @click="open($event, `${REPOSITORY}/issues`)">
          Report a problem
        </a>
      </p>
    </section>
  </MainView>
</template>

<style scoped>
.about-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  max-width: 48rem;
}

.about-page section {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-3);
}

.about-page p {
  margin: 0;
}

.about-page__facts {
  display: grid;
  grid-template-columns: 12rem 1fr;
  gap: var(--space-1) var(--space-4);
  margin: 0;
}

.about-page__facts dt,
.about-page__note {
  color: var(--color-text-muted);
}

.about-page__facts dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.about-page__links {
  display: flex;
  gap: var(--space-4);
}

.about-page__links a {
  color: var(--color-accent-text);
}
</style>
