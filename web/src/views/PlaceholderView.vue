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
  padding: var(--space-8);
}

.version {
  color: var(--color-text-muted);
}
</style>
