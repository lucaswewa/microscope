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
  /** The phase that builds its page; until then a placeholder says so. */
  readonly builtIn: string
}

/** The rail's destinations, in order: Shift+↑ and Shift+↓ follow it. */
export const DESTINATIONS: readonly Destination[] = [
  { id: 'view', label: 'View', icon: view, group: 'top', requires: [], builtIn: 'P18' },
  { id: 'control', label: 'Control', icon: control, group: 'top', requires: [], builtIn: 'P20' },
  {
    id: 'slide-scan',
    label: 'Slide Scan',
    icon: slideScan,
    group: 'top',
    requires: ['smart_scan'],
    builtIn: 'P40',
  },
  {
    id: 'sequence',
    label: 'Sequence',
    icon: sequence,
    group: 'top',
    requires: ['sequence'],
    builtIn: 'P42',
  },
  {
    id: 'gallery',
    label: 'Gallery',
    icon: gallery,
    group: 'top',
    requires: ['gallery'],
    builtIn: 'P32',
  },
  {
    id: 'settings',
    label: 'Settings',
    icon: settings,
    group: 'bottom',
    requires: [],
    builtIn: 'P28',
  },
  { id: 'logging', label: 'Logging', icon: logging, group: 'bottom', requires: [], builtIn: 'P43' },
  { id: 'about', label: 'About', icon: about, group: 'bottom', requires: [], builtIn: 'P46' },
  { id: 'power', label: 'Power', icon: power, group: 'bottom', requires: [], builtIn: 'P46' },
]
