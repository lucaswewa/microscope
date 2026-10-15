import about from '@material-symbols/svg-400/outlined/info.svg?raw'
import power from '@material-symbols/svg-400/outlined/power_settings_new.svg?raw'
import logging from '@material-symbols/svg-400/outlined/receipt_long.svg?raw'
import settings from '@material-symbols/svg-400/outlined/settings.svg?raw'
import control from '@material-symbols/svg-400/outlined/control_camera.svg?raw'
import slideScan from '@material-symbols/svg-400/outlined/grid_on.svg?raw'
import gallery from '@material-symbols/svg-400/outlined/photo_library.svg?raw'
import sequence from '@material-symbols/svg-400/outlined/timelapse.svg?raw'
import view from '@material-symbols/svg-400/outlined/visibility.svg?raw'

/** A destination in the navigation rail, and its route. */
export interface Destination {
  /** The route's name, and its path (`/#/<id>`). */
  readonly id: string
  readonly label: string
  /** The icon's SVG source. */
  readonly icon: string
  /** Where it sits in the rail: with the workflows at the top, or pinned to the bottom. */
  readonly group: 'top' | 'bottom'
  /** The Things it needs: it's hidden while the microscope lacks any of them. */
  readonly requires: readonly string[]
}

/** The rail's destinations, in order: Shift+↑ and Shift+↓ follow it. */
export const DESTINATIONS: readonly Destination[] = [
  { id: 'view', label: 'View', icon: view, group: 'top', requires: [] },
  { id: 'control', label: 'Control', icon: control, group: 'top', requires: [] },
  {
    id: 'slide-scan',
    label: 'Slide Scan',
    icon: slideScan,
    group: 'top',
    requires: ['smart_scan'],
  },
  {
    id: 'sequence',
    label: 'Sequence',
    icon: sequence,
    group: 'top',
    requires: ['sequence'],
  },
  {
    id: 'gallery',
    label: 'Gallery',
    icon: gallery,
    group: 'top',
    requires: ['gallery'],
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: settings,
    group: 'bottom',
    requires: [],
  },
  { id: 'logging', label: 'Logging', icon: logging, group: 'bottom', requires: [] },
  { id: 'about', label: 'About', icon: about, group: 'bottom', requires: [] },
  { id: 'power', label: 'Power', icon: power, group: 'bottom', requires: [] },
]
