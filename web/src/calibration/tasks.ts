// The calibration wizard's tasks (P29), each a series of steps. There's one
// for each Thing the wizard knows how to calibrate, offered when the server
// has that Thing, the Thing has `calibration_required`, and it doesn't say it
// can't be calibrated (`can_calibrate`). On connecting, only the tasks whose
// Things need calibrating are offered, after a welcome.
import { markRaw, type Component } from 'vue'

import type { WotClient } from '@/api/wot/client'
import type { ThingDescription } from '@/api/wot/td'
import type { ActionButtonElement } from '@/server-ui/types'

import CalibrateStep from './CalibrateStep.vue'
import GuideStep from './GuideStep.vue'

export interface WizardStep {
  component: Component
  props: Record<string, unknown>
}

export interface WizardTask {
  title?: string
  steps: WizardStep[]
}

/** A step that says what to do, with the live image and the d-pad if it helps. */
const guide = (props: {
  lines: string[]
  bullets?: string[]
  stream?: boolean
  pad?: boolean
}) => ({
  component: markRaw(GuideStep),
  props,
})

/** A step that calibrates a Thing, through its own panel or a button. */
const calibrate = (props: {
  thing: string
  lines?: string[]
  stream?: boolean
  panel?: string
  button?: ActionButtonElement
}) => ({ component: markRaw(CalibrateStep), props })

export const WELCOME: WizardTask = {
  steps: [
    guide({
      lines: [
        "Some of the microscope's calibration is missing. It works without it, but some features are limited, and its images may suffer.",
        'Press Next to calibrate it.',
      ],
    }),
  ],
}

export const FINAL: WizardTask = {
  steps: [
    guide({
      lines: [
        "That's the calibration done.",
        'If you change the objective, calibrate again from Settings.',
        'Press Finish to start using the microscope.',
      ],
    }),
  ],
}

/** The tasks, by the Thing they calibrate, in the order they run. */
export const CALIBRATIONS: Record<string, WizardTask> = {
  stage: {
    title: 'Stage',
    steps: [calibrate({ thing: 'stage', panel: 'z_direction_check' })],
  },
  camera: {
    title: 'Camera',
    steps: [
      guide({
        lines: ['First, the illumination.'],
        bullets: ['Make sure its light is on.', 'Take any slide out of the microscope.'],
      }),
      guide({
        lines: ['Is the light in the middle of the image, and as bright as it gets?'],
        bullets: [
          'To centre it, loosen the screws that hold the illumination, slide it until the light is in the middle, and tighten them again.',
          "To focus it, loosen the condenser's thumbscrew, move the condenser up or down until the image is brightest, and tighten it again.",
        ],
        stream: true,
      }),
      guide({
        lines: [
          'Now show the camera an empty part of a slide, under its coverslip if it has one. Without such a slide, leave the microscope empty.',
        ],
        bullets: [
          'Nothing should be in view. If specks of dust are, focus away from them a little.',
        ],
        stream: true,
        pad: true,
      }),
      calibrate({
        thing: 'camera',
        lines: ['When the view is empty and evenly lit, calibrate the camera.'],
        stream: true,
        panel: 'primary_calibration_actions',
      }),
    ],
  },
  camera_stage_mapping: {
    title: 'Camera–stage mapping',
    steps: [
      guide({
        lines: [
          'The mapping measures how the image moves as the stage does, so it needs a sample with plenty of detail to follow.',
        ],
        bullets: ['Put one in, roughly centred.', 'Bring it into focus.'],
        stream: true,
        pad: true,
      }),
      calibrate({
        thing: 'camera_stage_mapping',
        lines: ["With the sample in focus, calibrate. If it isn't in focus, go back and focus it."],
        stream: true,
        button: {
          kind: 'action_button',
          thing: 'camera_stage_mapping',
          action: 'calibrate_xy',
          label: 'Calibrate',
          progress: 'dialog',
          stream: 'camera',
          success: { message: 'The camera–stage mapping is calibrated.' },
          refresh: true,
        },
      }),
    ],
  },
}

/** Reads a Thing's boolean property, or `fallback` if that fails. */
const read = (client: WotClient, td: ThingDescription, name: string, fallback: boolean) =>
  client
    .consume(td)
    .readProperty<boolean>(name)
    .catch(() => fallback)

/** The Things on the server that the wizard can calibrate, in order. */
export async function calibratable(
  client: WotClient,
  descriptions: Readonly<Record<string, ThingDescription>>,
): Promise<string[]> {
  const known = Object.keys(CALIBRATIONS).filter(
    (name) => descriptions[name]?.properties?.calibration_required,
  )
  const can = await Promise.all(
    known.map((name) =>
      descriptions[name]!.properties!.can_calibrate
        ? read(client, descriptions[name]!, 'can_calibrate', false)
        : true,
    ),
  )
  return known.filter((_, i) => can[i])
}

/** Of `things`, those that need calibrating. */
export async function needingCalibration(
  client: WotClient,
  descriptions: Readonly<Record<string, ThingDescription>>,
  things: string[],
): Promise<string[]> {
  const needed = await Promise.all(
    things.map((name) => read(client, descriptions[name]!, 'calibration_required', false)),
  )
  return things.filter((_, i) => needed[i])
}
