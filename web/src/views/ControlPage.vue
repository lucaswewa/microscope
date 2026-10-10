<script setup lang="ts">
// The Control tab: navigation in a narrow pane beside the live image. The
// arrow keys and PgUp/PgDn move the stage while it is open.
import { ref } from 'vue'

import DirectionPad from '@/control/DirectionPad.vue'
import PositionSection from '@/control/PositionSection.vue'
import { useJog } from '@/control/useJog'
import LiveImage from '@/live/LiveImage.vue'
import AccordionSection from '@/ui/AccordionSection.vue'
import AppAccordion from '@/ui/AppAccordion.vue'
import ControlPane from '@/ui/ControlPane.vue'

const jog = useJog()
const open = ref(['position'])
</script>

<template>
  <div class="control-page">
    <ControlPane width="narrow">
      <h1 class="visually-hidden">Control</h1>
      <AppAccordion v-model="open">
        <AccordionSection value="position" title="Position">
          <PositionSection />
        </AccordionSection>
      </AppAccordion>
      <DirectionPad class="control-page__pad" :controller="jog" />
    </ControlPane>
    <LiveImage />
  </div>
</template>

<style scoped>
.control-page {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.control-page__pad {
  margin-top: var(--space-4);
}
</style>
