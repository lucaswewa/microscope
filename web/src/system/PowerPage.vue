<script setup lang="ts">
// The Power tab (P46): shuts the server down, after asking, and then the app
// shows it has stopped, with Reconnect; restarts it, also after asking, when
// the launcher manages it (ADR-0033); and, connected to another microscope,
// disconnects from it.
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { useThing } from '@/api/things'
import { ApiError } from '@/api/wot/errors'
import { apiRoot, displayOrigin } from '@/connection/profiles'
import { useConnectionStore } from '@/connection/store'
import AppButton from '@/ui/AppButton.vue'
import MainView from '@/ui/MainView.vue'
import { useConfirm, useToast } from '@/ui/overlays'
import SectionHeading from '@/ui/SectionHeading.vue'

const connection = useConnectionStore()
const system = useThing('system')
const confirm = useConfirm()
const toast = useToast()
const router = useRouter()

const managed = ref(false)
watch(
  system,
  async (thing) => {
    managed.value = (await thing?.read('managed').catch(() => false)) ?? false
  },
  { immediate: true },
)
const remote = computed(() => connection.profile?.kind === 'remote')
const origin = computed(
  () => connection.profile && displayOrigin(apiRoot(connection.profile).origin),
)

const report = (message: string) => (error: unknown) =>
  toast.error(message, {
    details: error instanceof ApiError ? (error.body ?? error.message) : error,
  })

async function shutDown() {
  const asked = await confirm({
    title: 'Shut down',
    message: `Shut down the microscope’s server at ${origin.value}? Anything running, such as a scan, stops. It has to be started again on its computer.`,
    confirmLabel: 'Shut down',
    danger: true,
  })
  if (!asked || !system.value) return
  try {
    await system.value.invoke('shutdown')
    connection.markShutDown()
  } catch (error) {
    report('The server couldn’t be shut down.')(error)
  }
}

async function restart() {
  const asked = await confirm({
    title: 'Restart',
    message: 'Restart the microscope’s server? Anything running, such as a scan, stops.',
    confirmLabel: 'Restart',
    danger: true,
  })
  if (!asked || !system.value) return
  try {
    await system.value.invoke('restart')
    toast.success('The server is restarting. The app reconnects once it’s back.')
  } catch (error) {
    report('The server couldn’t restart.')(error)
  }
}

async function disconnect() {
  connection.disconnect()
  await router.push({ name: 'connect' })
}
</script>

<template>
  <MainView class="power-page">
    <h1 class="visually-hidden">Power</h1>
    <section>
      <SectionHeading>Shut down</SectionHeading>
      <p>
        Stops the microscope’s server, and anything it’s running.
        {{ managed ? 'The launcher stops with it.' : '' }}
      </p>
      <AppButton variant="danger" @click="shutDown">Shut down</AppButton>
    </section>
    <section v-if="managed">
      <SectionHeading>Restart</SectionHeading>
      <p>Stops the server and starts it again. The app reconnects by itself.</p>
      <AppButton @click="restart">Restart</AppButton>
    </section>
    <section v-if="remote">
      <SectionHeading>Disconnect</SectionHeading>
      <p>Stops using the microscope at {{ origin }}, which keeps running.</p>
      <AppButton @click="disconnect">Disconnect</AppButton>
    </section>
  </MainView>
</template>

<style scoped>
.power-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  max-width: 48rem;
}

.power-page section {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-3);
}

.power-page p {
  margin: 0;
}
</style>
