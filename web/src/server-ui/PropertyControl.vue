<script setup lang="ts">
// An input for a property, described by a Thing's panel (ADR-0024). It suits
// the value: the panel's options or the schema's enum as a list, hours,
// minutes and seconds for a duration, a switch, a number field within the
// schema's limits, a text field, or else the value, read-only. It follows the
// value if the property is observable. A committed edit is written at once,
// and read back after `read_back_ms` if the panel asks. Until the last write
// is done, values the Thing sends are older than the edits, so they're passed
// over, and the value is read once it is. While the value
// differs from the schema's default, the label is highlighted. If the Thing
// lacks the property, the control says so.
import { computed, ref, watch, watchEffect } from 'vue'

import { ApiError } from '@/api/wot/errors'
import { sleep } from '@/api/wot/invocation'
import { useConnectionStore } from '@/connection/store'
import AppSelect from '@/ui/AppSelect.vue'
import AppToggle from '@/ui/AppToggle.vue'
import FormField from '@/ui/FormField.vue'
import NumberField from '@/ui/NumberField.vue'
import TextField from '@/ui/TextField.vue'
import { useToast } from '@/ui/overlays'

import DurationField from './DurationField.vue'
import { followProperty } from './follow'
import type { PropertyControlElement } from './types'

const props = defineProps<{ control: PropertyControlElement }>()
const connection = useConnectionStore()
const toast = useToast()

/** The Thing and the property's schema, unless the Thing lacks it. */
const target = computed(() => {
  const td = connection.descriptions?.[props.control.thing]
  const schema = td?.properties?.[props.control.property]
  if (props.control.broken || !td || !schema || !connection.client) return undefined
  return { thing: connection.client.consume(td), schema }
})

const value = ref<unknown>()
const unreadable = ref(false)
/** How many writes are on their way. */
let writing = 0

async function read() {
  const property = target.value
  if (!property) return
  try {
    value.value = await property.thing.readProperty(props.control.property)
    unreadable.value = false
  } catch {
    unreadable.value = value.value === undefined
  }
}
watchEffect((onCleanup) => {
  const property = target.value
  if (!property) return
  void read()
  if (!property.schema.observable) return
  const follow = (next: unknown) => {
    if (writing === 0) value.value = next
  }
  onCleanup(followProperty(property.thing, props.control.property, follow))
})

async function write(next: unknown) {
  const property = target.value
  if (!property || same(next, value.value)) return
  const previous = value.value
  value.value = next
  writing += 1
  try {
    await property.thing.writeProperty(props.control.property, next)
    if (props.control.read_back_ms !== undefined) await sleep(props.control.read_back_ms)
  } catch (error) {
    value.value = previous
    toast.error(`Couldn't set ${props.control.label}.`, {
      details: error instanceof ApiError ? (error.body ?? error.message) : error,
    })
    return
  } finally {
    writing -= 1
  }
  const passedOver = property.schema.observable
  if (writing === 0 && (passedOver || props.control.read_back_ms !== undefined)) await read()
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

const choices = computed(
  () =>
    props.control.options ??
    target.value?.schema.enum?.map((choice) => ({ label: String(choice), value: choice })) ??
    [],
)

const kind = computed(() => {
  const schema = target.value?.schema
  if (!schema) return 'broken'
  if (value.value === undefined) return unreadable.value ? 'unreadable' : 'loading'
  if (schema.readOnly) return 'readonly'
  if (choices.value.length > 0) return 'choice'
  if (props.control.input === 'duration_hms' && typeof value.value === 'number') return 'duration'
  return ['boolean', 'number', 'string'].includes(typeof value.value)
    ? typeof value.value
    : 'readonly'
})

const number = computed(() => {
  const schema = target.value?.schema
  const step = props.control.step ?? (schema?.multipleOf as number | undefined)
  return {
    min: schema?.minimum,
    max: schema?.maximum,
    step: step ?? (schema?.type === 'integer' ? 1 : undefined),
  }
})

// The text field's draft, written when it's committed.
const draft = ref('')
watch(value, (text) => typeof text === 'string' && (draft.value = text), { immediate: true })

const changed = computed(() => {
  const fallback = target.value?.schema.default
  return fallback !== undefined && value.value !== undefined && !same(value.value, fallback)
})
</script>

<template>
  <div class="property-control" :class="{ 'property-control--changed': changed }">
    <AppToggle
      v-if="kind === 'boolean'"
      :label="control.label"
      :model-value="value === true"
      @update:model-value="write"
    />
    <fieldset v-else-if="kind === 'duration'" class="property-control__group">
      <legend class="property-control__label">{{ control.label }}</legend>
      <DurationField :model-value="value as number" @update:model-value="write" />
    </fieldset>
    <FormField v-else-if="['choice', 'number', 'string'].includes(kind)" :label="control.label">
      <AppSelect
        v-if="kind === 'choice'"
        size="sm"
        :options="choices.map((choice, index) => ({ value: index, label: choice.label }))"
        :model-value="choices.findIndex((choice) => same(choice.value, value))"
        @update:model-value="(index) => index !== undefined && write(choices[index]?.value)"
      />
      <NumberField
        v-else-if="kind === 'number'"
        size="sm"
        v-bind="number"
        :model-value="value as number"
        @update:model-value="write"
      />
      <TextField v-else v-model="draft" size="sm" @submit="write" @change="write(draft)" />
    </FormField>
    <template v-else>
      <span class="property-control__label">{{ control.label }}</span>
      <p v-if="kind === 'broken'" class="property-control__broken">
        Not available: {{ control.thing }} has no property “{{ control.property }}”.
      </p>
      <p v-else-if="kind === 'unreadable'" class="property-control__broken">Couldn't read it.</p>
      <output v-else-if="kind === 'readonly'">
        {{ typeof value === 'object' ? JSON.stringify(value) : value }}
      </output>
    </template>
    <p v-if="changed" class="property-control__note">Changed from its default.</p>
  </div>
</template>

<style scoped>
.property-control {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.property-control__group {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}

.property-control__label {
  padding: 0;
  font-weight: var(--font-weight-medium);
}

.property-control--changed .property-control__label,
.property-control--changed :deep(.form-field__label),
.property-control--changed :deep(.app-toggle) {
  color: var(--color-accent-text);
}

.property-control__broken,
.property-control__note {
  margin: 0;
  font-size: var(--font-size-sm);
}

.property-control__broken {
  color: var(--color-danger);
}

.property-control__note {
  color: var(--color-text-muted);
}
</style>
