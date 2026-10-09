<script setup lang="ts">
// The Connect screen (/#/connect): this server, another microscope by its
// address, or a recent one. A successful connection goes to the View tab.
import close from '@material-symbols/svg-400/outlined/close.svg?raw'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import AppButton from '@/ui/AppButton.vue'
import AppCard from '@/ui/AppCard.vue'
import ErrorDetails from '@/ui/ErrorDetails.vue'
import FormField from '@/ui/FormField.vue'
import IconButton from '@/ui/IconButton.vue'
import MainView from '@/ui/MainView.vue'
import SectionHeading from '@/ui/SectionHeading.vue'
import TextField from '@/ui/TextField.vue'

import { displayOrigin, parseAddress, type Profile } from './profiles'
import { useConnectionStore } from './store'

const connection = useConnectionStore()
const router = useRouter()

const thisServer = displayOrigin(window.location.origin)
const address = ref('')
const addressError = ref<string>()

const status = computed(() => {
  const host = connection.hostname
  switch (connection.state) {
    case 'connected':
      return `Connected to ${host}.`
    case 'connecting':
    case 'reconnecting':
      return 'Connecting…'
    case 'lost':
      return 'Not connected: the microscope isn’t answering.'
    default:
      return 'Not connected.'
  }
})

async function go(profile: Profile) {
  await connection.connect(profile)
  if (connection.state === 'connected') await router.push({ name: 'view' })
}

function connectToAddress() {
  try {
    const origin = parseAddress(address.value)
    addressError.value = undefined
    void go({ kind: 'remote', origin })
  } catch (error) {
    addressError.value = (error as Error).message
  }
}
</script>

<template>
  <MainView>
    <div class="connect">
      <h1>Connect to a microscope</h1>
      <p class="connect__status" role="status">{{ status }}</p>
      <ErrorDetails
        v-if="connection.state === 'lost' && connection.error"
        :error="connection.error"
      />

      <AppCard>
        <SectionHeading :level="2">This server</SectionHeading>
        <p class="connect__note">The microscope that served this page, at {{ thisServer }}.</p>
        <AppButton variant="primary" @click="go({ kind: 'local' })">Connect</AppButton>
      </AppCard>

      <AppCard>
        <SectionHeading :level="2">Another microscope</SectionHeading>
        <form class="connect__form" novalidate @submit.prevent="connectToAddress">
          <FormField
            label="Address"
            help="Such as lab-pc:5000 or http://192.168.1.20:5000"
            :error="addressError"
          >
            <TextField v-model="address" placeholder="lab-pc:5000" autocomplete="url" />
          </FormField>
          <AppButton type="submit" variant="primary">Connect</AppButton>
        </form>
      </AppCard>

      <AppCard v-if="connection.recent.length > 0">
        <SectionHeading :level="2">Recent</SectionHeading>
        <ul class="connect__recent">
          <li v-for="origin in connection.recent" :key="origin">
            <AppButton variant="ghost" @click="go({ kind: 'remote', origin })">
              {{ displayOrigin(origin) }}
            </AppButton>
            <IconButton
              :icon="close"
              :label="`Forget ${displayOrigin(origin)}`"
              variant="ghost"
              size="sm"
              @click="connection.forget(origin)"
            />
          </li>
        </ul>
      </AppCard>
    </div>
  </MainView>
</template>

<style scoped>
.connect {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  max-width: 560px;
}

.connect__status,
.connect__note {
  margin: 0 0 var(--space-3);
  color: var(--color-text-muted);
}

.connect__form {
  display: flex;
  gap: var(--space-2);
  align-items: flex-start;
}

.connect__form > :first-child {
  flex: 1;
}

.connect__form > button {
  margin-top: calc(var(--font-size-md) * var(--line-height-normal) + var(--space-1));
}

.connect__recent {
  margin: 0;
  padding: 0;
  list-style: none;
}

.connect__recent li {
  display: flex;
  gap: var(--space-1);
  align-items: center;
  justify-content: space-between;
}
</style>
