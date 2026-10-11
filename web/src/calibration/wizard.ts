import { defineStore } from 'pinia'
import { ref, shallowRef, watch } from 'vue'

import type { Profile } from '@/connection/profiles'
import { useConnectionStore } from '@/connection/store'

import {
  CALIBRATIONS,
  calibratable,
  FINAL,
  needingCalibration,
  WELCOME,
  type WizardTask,
} from './tasks'

/**
 * Whether the calibration wizard is open, and with which tasks. It opens by
 * itself once for each connection, not after reconnecting, when something
 * needs calibrating; Settings opens it with every task it can do.
 */
export const useCalibrationWizard = defineStore('calibration-wizard', () => {
  const connection = useConnectionStore()
  const open = ref(false)
  const tasks = shallowRef<WizardTask[]>([])

  async function show(onlyNeeded: boolean) {
    const { client, descriptions } = connection
    if (!client || !descriptions) return
    let things = await calibratable(client, descriptions)
    if (onlyNeeded) things = await needingCalibration(client, descriptions, things)
    if (onlyNeeded && things.length === 0) return
    const chosen = things.map((name) => CALIBRATIONS[name]!)
    tasks.value = [...(onlyNeeded ? [WELCOME] : []), ...chosen, FINAL]
    open.value = true
  }

  let checked: Profile | undefined
  watch(
    () => connection.state === 'connected' && connection.profile,
    (profile) => {
      if (!profile || profile === checked) return
      checked = profile
      void show(true)
    },
    { immediate: true },
  )

  return { open, tasks, openAll: () => show(false) }
})
