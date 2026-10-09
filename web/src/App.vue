<script setup lang="ts">
import AppShell from './app/AppShell.vue'
import OverlayProvider from './ui/OverlayProvider.vue'
</script>

<template>
  <OverlayProvider>
    <AppShell />
  </OverlayProvider>
</template>
