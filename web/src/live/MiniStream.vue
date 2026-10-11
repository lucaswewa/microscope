<script setup lang="ts">
// A small live image, from a camera's low-resolution stream: for panels and
// dialogs that need to see what the camera sees while they work.
import LiveImage from './LiveImage.vue'

withDefaults(defineProps<{ thing?: string }>(), { thing: 'camera' })
</script>

<template>
  <LiveImage :thing="thing" stream="lores_mjpeg_stream" class="mini-stream" />
</template>

<style scoped>
.mini-stream {
  flex: none;
  width: 100%;
  max-width: 20rem;
  aspect-ratio: 4 / 3;
  border-radius: var(--radius-md);
}
</style>
