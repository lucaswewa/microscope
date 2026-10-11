<script setup lang="ts">
// A panel a Thing describes (ADR-0024), drawn element by element. Accordions
// and containers draw their children the same way. An element of a kind this
// app doesn't know is drawn as a note, so that a newer server's panel still
// shows the rest. `refresh` asks for the panel to be described again, after
// an action that changes it.
import AccordionSection from '@/ui/AccordionSection.vue'
import AppAccordion from '@/ui/AppAccordion.vue'
import SectionHeading from '@/ui/SectionHeading.vue'

import ActionButton from './ActionButton.vue'
import DownloadButton from './DownloadButton.vue'
import PropertyControl from './PropertyControl.vue'
import RichText from './RichText.vue'
import type { UiElement } from './types'

defineProps<{ elements: UiElement[]; layout?: 'column' | 'row' }>()
const emit = defineEmits<{ refresh: [] }>()
</script>

<template>
  <div class="server-interface" :class="`server-interface--${layout ?? 'column'}`">
    <template v-for="(element, index) in elements" :key="index">
      <SectionHeading v-if="element.kind === 'header'" :level="3">
        <RichText :text="element.text" />
      </SectionHeading>
      <p v-else-if="element.kind === 'text'" class="server-interface__text">
        <RichText :text="element.text" />
      </p>
      <ul v-else-if="element.kind === 'bullets'" class="server-interface__text">
        <li v-for="(item, i) in element.items" :key="i"><RichText :text="item" /></li>
      </ul>
      <PropertyControl v-else-if="element.kind === 'property_control'" :control="element" />
      <AppAccordion v-else-if="element.kind === 'accordion'">
        <AccordionSection value="section" :title="element.title">
          <ServerInterface :elements="element.children" @refresh="emit('refresh')" />
        </AccordionSection>
      </AppAccordion>
      <ServerInterface
        v-else-if="element.kind === 'container'"
        :elements="element.children"
        :layout="element.layout"
        @refresh="emit('refresh')"
      />
      <ActionButton
        v-else-if="element.kind === 'action_button'"
        :button="element"
        @refresh="emit('refresh')"
      />
      <DownloadButton v-else-if="element.kind === 'download_button'" :button="element" />
      <p v-else class="server-interface__unsupported">
        This part of the panel needs a newer version of the app.
      </p>
    </template>
  </div>
</template>

<style scoped>
.server-interface {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.server-interface--row {
  flex-flow: row wrap;
  gap: var(--space-4);
  align-items: flex-end;
}

.server-interface__text {
  margin: 0;
}

.server-interface :deep(a) {
  color: var(--color-accent-text);
}

.server-interface__unsupported {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}
</style>
