/**
 * The server log (P03, ADR-0008), as the Logging tab shows it: its records
 * in groups of related ones. An invocation's records make one group, and
 * each run of consecutive records from the same source, outside any
 * invocation, makes another.
 */

/** One record of the server log, as `GET /api/v1/log/` returns it. */
export interface ServerLogRecord {
  /** Counts records from 1, in the order they were logged. */
  sequence: number
  /** Where it came from: a module path, such as `microscope_things::camera`. */
  name: string
  message: string
  levelname: string
  levelno: number
  /** When it was logged: RFC 3339, in UTC. */
  created: string
  filename: string
  lineno: number
  exception_type: string | null
  traceback: string | null
  /** The invocation it was logged in, with its Thing and action, if any. */
  invocation_id: string | null
  thing: string | null
  action: string | null
}

/** The levels, least severe first, with Python's numbers. */
export const LEVELS = [
  { name: 'DEBUG', levelno: 10 },
  { name: 'INFO', levelno: 20 },
  { name: 'WARNING', levelno: 30 },
  { name: 'ERROR', levelno: 40 },
  { name: 'CRITICAL', levelno: 50 },
] as const

/** Related records, in the order they were logged. */
export interface LogGroup {
  /** Unique in the log: the invocation's ID, or the first record's number. */
  key: string
  /** The invocation the records were logged in, if any. */
  invocation: string | null
  /** The invocation's Thing and action, or the records' source. */
  title: string
  records: ServerLogRecord[]
}

/** `records`, oldest first, in groups ordered by their first records. */
export function groupRecords(records: readonly ServerLogRecord[]): LogGroup[] {
  const groups: LogGroup[] = []
  const invocations = new Map<string, LogGroup>()
  let previous: LogGroup | undefined
  for (const record of records) {
    const id = record.invocation_id
    const sameSource = !previous?.invocation && previous?.records[0]!.name === record.name
    let group = id ? invocations.get(id) : sameSource ? previous : undefined
    if (!group) {
      const action = record.thing && record.action && `${record.thing}.${record.action}`
      const title = id ? action || `Invocation ${id}` : record.name
      group = { key: id ?? `#${record.sequence}`, invocation: id, title, records: [] }
      if (id) invocations.set(id, group)
      groups.push(group)
    }
    group.records.push(record)
    previous = group
  }
  return groups
}

/** `groups` with only their records at `levelno` or above, leaving out those with none. */
export function filterGroups(groups: readonly LogGroup[], levelno: number): LogGroup[] {
  return groups
    .map((group) => ({ ...group, records: group.records.filter((r) => r.levelno >= levelno) }))
    .filter((group) => group.records.length)
}

/** The most severe of `group`'s records. */
export function highest(group: LogGroup): ServerLogRecord {
  return group.records.reduce((most, record) => (record.levelno > most.levelno ? record : most))
}
