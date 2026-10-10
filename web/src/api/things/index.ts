/**
 * Typed facades for the Things (ADR-0021). The names of a Thing's
 * properties, actions and events, and their types, all come from the schemas
 * generated from the server's OpenAPI document, so code that uses them fails
 * to compile when the server changes under it.
 */
import { computed } from 'vue'

import type { components, paths } from '@/api/generated/schema'
import type { ConsumedThing, RequestOptions, StreamOptions } from '@/api/wot/client'
import type { FollowOptions, Invocation } from '@/api/wot/invocation'
import { useConnectionStore } from '@/connection/store'

type Schemas = components['schemas']

/** The affordances of Thing `T` with a schema named `${T}_{name}_${Kind}`. */
type Names<T extends string, Kind extends string> = keyof Schemas extends infer Key
  ? Key extends `${T}_${infer Name}_${Kind}`
    ? Name
    : never
  : never
type Schema<Name extends string> = Schemas[Name & keyof Schemas]

/** The properties of `T` that can be written: those the server accepts a PUT for. */
type Writable<T extends string> = {
  [P in Names<T, 'value'>]: paths[`/api/v1/${T}/${P}` & keyof paths] extends { put: object }
    ? P
    : never
}[Names<T, 'value'>]

/** An action's input, which may be left out when nothing in it is required, and the options. */
type InputAndOptions<Input> =
  Record<string, never> extends Input
    ? [input?: Input, options?: FollowOptions]
    : [input: Input, options?: FollowOptions]

/** An invocation whose output has its action's type. */
export type TypedInvocation<Output> = Omit<Invocation, 'output'> & { output(): Promise<Output> }

/** A Thing, used through its affordances' generated types, such as `TypedThing<'stage'>`. */
export class TypedThing<T extends string> {
  constructor(readonly thing: ConsumedThing) {}

  read<P extends Names<T, 'value'>>(name: P, options?: RequestOptions) {
    return this.thing.readProperty<Schema<`${T}_${P}_value`>>(name, options)
  }

  write<P extends Writable<T>>(
    name: P,
    value: Schema<`${T}_${P}_value`>,
    options?: RequestOptions,
  ) {
    return this.thing.writeProperty(name, value, options)
  }

  observe<P extends Names<T, 'value'>>(
    name: P,
    onValue: (value: Schema<`${T}_${P}_value`>) => void,
    options?: StreamOptions,
  ) {
    return this.thing.observeProperty(name, (value) => onValue(value as never), options)
  }

  /** Starts an action, and returns its invocation, which follows it. */
  invoke<A extends Names<T, 'input'>>(
    name: A,
    ...[input, options]: InputAndOptions<Schema<`${T}_${A}_input`>>
  ): Promise<TypedInvocation<Schema<`${T}_${A}_output`>>> {
    // An action with nothing required still takes an object, even an empty one.
    return this.thing.invokeAction(name, input ?? {}, options)
  }

  /** Runs an action to its end, and returns its output. */
  async run<A extends Names<T, 'input'>>(
    name: A,
    ...args: InputAndOptions<Schema<`${T}_${A}_input`>>
  ): Promise<Schema<`${T}_${A}_output`>> {
    return (await this.invoke(name, ...args)).output()
  }

  subscribe<E extends Names<T, 'data'>>(
    name: E,
    onEvent: (data: Schema<`${T}_${E}_data`>) => void,
    options?: StreamOptions,
  ) {
    return this.thing.subscribeEvent(name, (data) => onEvent(data as never), options)
  }
}

/** The Things the app knows, by the names `configs/simulation.json` gives them. */
export type ThingName = 'system' | 'stage' | 'camera' | 'illumination'

/** Thing `name` as a typed facade, while the app has a client and the Thing's description. */
export function useThing<T extends ThingName>(name: T) {
  const connection = useConnectionStore()
  return computed(() => {
    const td = connection.descriptions?.[name]
    return connection.client && td ? new TypedThing<T>(connection.client.consume(td)) : undefined
  })
}

/** A stage position, in steps. */
export type Position = Schemas['stage_position_value']
/** µm per step on each axis, where known. */
export type AxisScale = Schemas['stage_um_per_step_value']
/** A camera's streaming mode. */
export type StreamingMode = Schemas['camera_streaming_mode_value']
