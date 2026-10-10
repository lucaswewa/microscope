// @vitest-environment node
import { describe, expect, expectTypeOf, it, vi } from 'vitest'

import { TypedThing, type Position } from '@/api/things'
import { WotClient } from '@/api/wot/client'
import type { ThingDescription } from '@/api/wot/td'

import { eventStream, fakeServer, json, type Received } from './fakeServer'

/** The parts of the stage's description that the test uses, as the server writes them. */
const stageTd: ThingDescription = {
  title: 'SimulatedStage',
  base: 'http://microscope.test/',
  properties: {
    position: { forms: [{ href: '/api/v1/stage/position', op: ['readproperty'] }] },
    backlash_steps: {
      forms: [{ href: '/api/v1/stage/backlash_steps', op: ['readproperty', 'writeproperty'] }],
    },
  },
  actions: {
    move_relative: { forms: [{ href: '/api/v1/stage/move_relative', op: ['invokeaction'] }] },
  },
  events: {
    arrived: {
      forms: [{ href: '/api/v1/stage/arrived', op: ['subscribeevent'], subprotocol: 'sse' }],
    },
  },
}

describe('typed facades', () => {
  it('read, write, run and subscribe with the generated types', async () => {
    const at = { x: 10, y: 0, z: 0 }
    const server = fakeServer({
      'GET /api/v1/stage/position': { x: 1, y: 2, z: 3 },
      'PUT /api/v1/stage/backlash_steps': null,
      'POST /api/v1/stage/move_relative': () =>
        json(
          {
            id: '1',
            status: 'completed',
            href: 'http://microscope.test/api/v1/action_invocations/1',
            output: at,
          },
          201,
        ),
      'GET /api/v1/stage/arrived': (request: Received) =>
        eventStream([`data: ${JSON.stringify(at)}\n\n`], request, { open: true }),
    })
    const client = new WotClient({ baseUrl: 'http://microscope.test/api/v1/', fetch: server.fetch })
    const stage = new TypedThing<'stage'>(client.consume(stageTd))

    const position = await stage.read('position')
    expectTypeOf(position).toEqualTypeOf<Position>()
    expect(position).toEqual({ x: 1, y: 2, z: 3 })

    await stage.write('backlash_steps', { x: 100, y: -100, z: 0 })
    expect(JSON.parse(server.received[1]!.body!)).toEqual({ x: 100, y: -100, z: 0 })

    const moved = await stage.run('move_relative', { x: 10 })
    expectTypeOf(moved).toEqualTypeOf<Position>()
    expect(moved).toEqual(at)

    const arrivals: Position[] = []
    const stop = stage.subscribe('arrived', (data) => arrivals.push(data))
    await vi.waitFor(() => expect(arrivals).toEqual([at]))
    stop()
  })

  it('refuses at compile time what the server doesn’t offer', () => {
    // Never called: `npm run typecheck` checks each line marked as an error is one.
    const misuses = (stage: TypedThing<'stage'>, camera: TypedThing<'camera'>) => {
      // @ts-expect-error: the stage has no `speed` property.
      void stage.read('speed')
      // @ts-expect-error: `position` is read-only.
      void stage.write('position', { x: 0, y: 0, z: 0 })
      // @ts-expect-error: steps are numbers.
      void stage.write('backlash_steps', { x: '1', y: 0, z: 0 })
      // @ts-expect-error: `save_from_memory` needs a path.
      void camera.run('save_from_memory')
      // Inputs with nothing required may be left out.
      void stage.run('move_to_origin')
      void stage.run('set_zero_position')
      void camera.run('save_from_memory', { path: 'a.jpg' })
    }
    expect(misuses).toBeTypeOf('function')
  })
})
