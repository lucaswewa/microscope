import type { ServerLogRecord } from '@/logging/log'

/** An invocation: its ID, Thing and action. */
type Invocation = [string, string | null, string | null]

const autofocus: Invocation = ['a1', 'autofocus', 'fast_autofocus']
const sequence: Invocation = ['b2', 'sequence', 'run_sequence']
const unknown: Invocation = ['c3', null, null]

/** Record `n`, logged `n` seconds into 14 October 2026. */
export function record(
  n: number,
  levelname: string,
  name: string,
  message: string,
  [invocation_id, thing, action]: Partial<Invocation> = [],
  more: Partial<ServerLogRecord> = {},
): ServerLogRecord {
  const levelno = { DEBUG: 10, INFO: 20, WARNING: 30, ERROR: 40, CRITICAL: 50 }[levelname]!
  return {
    sequence: n,
    name,
    message,
    levelname,
    levelno,
    created: new Date(Date.UTC(2026, 9, 14, 0, 0, n)).toISOString(),
    filename: 'lib.rs',
    lineno: n,
    exception_type: null,
    traceback: null,
    invocation_id: invocation_id ?? null,
    thing: thing ?? null,
    action: action ?? null,
    ...more,
  }
}

/** A server's log, oldest first, as the server logs it: startup, two invocations and the camera between them. */
export const LOG: ServerLogRecord[] = [
  record(1, 'INFO', 'microscope_server::cli', 'application configuration data_folder=.microscope'),
  record(2, 'INFO', 'microscope_server::lifecycle', 'serving 16 Things on http://127.0.0.1:5000'),
  record(3, 'DEBUG', 'microscope_things::camera', 'streaming started'),
  record(4, 'DEBUG', 'microscope_things::camera', 'frame 1 encoded'),
  record(5, 'INFO', 'microscope_things::autofocus', 'sweeping 400 steps', autofocus),
  record(6, 'DEBUG', 'microscope_things::camera', 'frame 2 encoded'),
  record(7, 'WARNING', 'microscope_things::autofocus', 'no clear peak', autofocus),
  record(8, 'INFO', 'wot::things', 'running run_sequence', sequence),
  record(9, 'WARNING', 'microscope_things::sequence', 'Skipped 4 time points', sequence),
  record(10, 'ERROR', 'wot::things', "the captures can't be saved", sequence, {
    exception_type: 'ActionError',
    traceback: 'the disk is full',
  }),
  record(11, 'INFO', 'wot::things', "Global lock was busy: didn't run it.", unknown),
  record(12, 'INFO', 'microscope_server::lifecycle', 'stopping'),
]
