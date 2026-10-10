<script setup lang="ts">
// The Control tab: navigation in a narrow pane beside the live image. The
// arrow keys and PgUp/PgDn move the stage while it is open, and the mouse
// wheel over the image focuses.
import { ref } from 'vue'

import { useThing } from '@/api/things'
import DirectionPad from '@/control/DirectionPad.vue'
import NavigationSection from '@/control/NavigationSection.vue'
import { useNavigationPreferences } from '@/control/navigation'
import PositionSection from '@/control/PositionSection.vue'
import { useJog } from '@/control/useJog'
import { wheelFocus } from '@/control/wheel'
import LiveImage from '@/live/LiveImage.vue'
import AccordionSection from '@/ui/AccordionSection.vue'
import AppAccordion from '@/ui/AppAccordion.vue'
import ControlPane from '@/ui/ControlPane.vue'

const jog = useJog()
const open = ref(['position'])
const stage = useThing('stage')
const preferences = useNavigationPreferences()
const onWheel = wheelFocus(
  (z) => void stage.value?.invoke('jog', { z }).catch(() => {}),
  () => preferences.signedSteps().z,
)
</script>

<template>
  <div class="control-page">
    <ControlPane width="narrow">
      <h1 class="visually-hidden">Control</h1>
      <AppAccordion v-model="open">
        <AccordionSection value="position" title="Position">
          <PositionSection />
        </AccordionSection>
        <AccordionSection value="navigation" title="Navigation">
          <NavigationSection />
        </AccordionSection>
      </AppAccordion>
      <DirectionPad class="control-page__pad" :controller="jog" />
    </ControlPane>
    <LiveImage @wheel="onWheel" />
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
