<script setup lang="ts">
// Settings, Camera: a small live image, the camera's calibration actions
// and settings, and the settings of the background detector it uses, all as
// the camera and the detector describe them (ADR-0024).
import { ref, watchEffect } from 'vue'

import { useThing } from '@/api/things'
import MiniStream from '@/live/MiniStream.vue'
import ServerPanel from '@/server-ui/ServerPanel.vue'
import SectionHeading from '@/ui/SectionHeading.vue'

const camera = useThing('camera')
const detector = ref<string | null>()

watchEffect((onCleanup) => {
  const thing = camera.value
  if (!thing) return
  const show = (name: string | null) => (detector.value = name)
  void thing.read('background_detector_name').then(show, () => {})
  onCleanup(thing.observe('background_detector_name', show))
})
</script>

<template>
  <div class="camera-settings">
    <MiniStream />
    <SectionHeading :level="3">Calibration</SectionHeading>
    <ServerPanel thing="camera" path="primary_calibration_actions" />
    <ServerPanel thing="camera" path="secondary_calibration_actions" />
    <SectionHeading :level="3">Settings</SectionHeading>
    <ServerPanel thing="camera" path="manual_camera_settings" />
    <template v-if="detector">
      <SectionHeading :level="3">Background detection</SectionHeading>
      <ServerPanel :thing="detector" path="settings_ui" />
    </template>
  </div>
</template>

<style scoped>
.camera-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  max-width: 36rem;
}
</style>
