import { describe, expect, it } from 'vitest'

import { filterGroups, groupRecords, highest } from '@/logging/log'

import { LOG } from './fixture'

const shape = (groups: ReturnType<typeof groupRecords>) =>
  groups.map((group) => [group.title, group.records.map((r) => r.sequence)])

describe('the server log', () => {
  it('groups an invocation’s records, and consecutive records from the same source', () => {
    const groups = groupRecords(LOG)
    expect(shape(groups)).toEqual([
      ['microscope_server::cli', [1]],
      ['microscope_server::lifecycle', [2]],
      ['microscope_things::camera', [3, 4]],
      // An invocation's records, whatever came between them.
      ['autofocus.fast_autofocus', [5, 7]],
      // Another run from the camera: a record of the invocation came between.
      ['microscope_things::camera', [6]],
      ['sequence.run_sequence', [8, 9, 10]],
      ['Invocation c3', [11]],
      ['microscope_server::lifecycle', [12]],
    ])
    expect(new Set(groups.map((g) => g.key)).size).toBe(groups.length)
    expect(groups.map((g) => g.invocation ?? '-').join(' ')).toBe('- - - a1 - b2 c3 -')
  })

  it('keeps the records at a level or above, leaving out groups with none', () => {
    const groups = groupRecords(LOG)
    expect(shape(filterGroups(groups, 20))).toEqual([
      ['microscope_server::cli', [1]],
      ['microscope_server::lifecycle', [2]],
      ['autofocus.fast_autofocus', [5, 7]],
      ['sequence.run_sequence', [8, 9, 10]],
      ['Invocation c3', [11]],
      ['microscope_server::lifecycle', [12]],
    ])
    expect(shape(filterGroups(groups, 30))).toEqual([
      ['autofocus.fast_autofocus', [7]],
      ['sequence.run_sequence', [9, 10]],
    ])
    expect(shape(filterGroups(groups, 50))).toEqual([])
    // The groups themselves are left as they were.
    expect(groups[5]!.records).toHaveLength(3)
  })

  it('finds each group’s most severe record', () => {
    const levels = groupRecords(LOG).map((group) => highest(group).levelname)
    expect(levels).toEqual(['INFO', 'INFO', 'DEBUG', 'WARNING', 'DEBUG', 'ERROR', 'INFO', 'INFO'])
  })
})
