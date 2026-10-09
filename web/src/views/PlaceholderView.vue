<script setup lang="ts">
import { useHost } from '@/host'

const { info } = useHost()
</script>

<template>
  <main class="placeholder">
    <h1>Microscope</h1>
    <p>The interface is built in the coming phases.</p>
    <p class="version">Web app {{ info.version }}</p>
  </main>
</template>

<style scoped>
.placeholder {
  margin: 2rem;
  font-family: system-ui, sans-serif;
}

.version {
  color: GrayText;
}
</style>
