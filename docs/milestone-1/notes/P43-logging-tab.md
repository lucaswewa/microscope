# P43: Logging tab

- Status: In review
- Pull request: [#56](https://github.com/lucaswewa/microscope/pull/56)
- ADRs: none new
- Spec: [phases.md#p43](../phases.md#p43)

## Summary

The Logging tab shows the server's log (P03) in the app.

- **Groups of related records,** newest first, 10 to a page:
  - an invocation's records make one group, named by its Thing and action;
  - each run of consecutive records from the same source, outside any invocation, makes another, named by the source.
- **Each group** shows its most severe level, when it began and how many records it has. It opens to show them, each with its time, level, message and, in an invocation's group, its source, then any error.
- **A level,** INFO by default, hides less severe records, and groups left with none.
- **Expand all and Collapse all** act on the page's groups.
- **Refresh** reads the log again, and **Download log file** saves today's.
- **When the log is empty,** or the level hides everything, the tab says so.

## What was built

| Where | What |
|---|---|
| `web/src/logging/log.ts` | `ServerLogRecord`, `LEVELS`, and `groupRecords`, `filterGroups` and `highest` |
| `web/src/logging/LoggingPage.vue` | The tab |
| `web/src/router/index.ts` | Its route, in place of the placeholder |

The server's side was built in P03: `GET /api/v1/log/` and `GET /api/v1/logfile/`.

## How to try it

```powershell
npm --prefix web run build
cargo run -p microscope-server -- -c configs/simulation.json --port 5090 --webapp-dir web/dist
```

1. **At `http://127.0.0.1:5090/#/logging`,** the server's startup shows as two groups, from `microscope_server::cli` and `microscope_server::lifecycle`.
2. **An action's error:** at `http://127.0.0.1:5090/docs`, run `sequence/download_zip` with `{"name": "missing"}`. Refresh: a `sequence.download_zip` group shows ERROR, and opens to "there's no sequence `missing`".
3. **A warning:** set `sequence/dt` to `0.5`, `duration` to `3` and `autofocus_enabled` to `true`, and run `sequence/run_sequence`. Refresh: its group shows WARNING, for the skipped points.
4. **The level:** choose WARNING, and only the two action groups are left. Choose CRITICAL, and the tab says nothing is at that level.
5. **Download log file** saves `microscope.<today>.log`.

## Design notes and deviations from the plan

- **Grouping** is done in the app, from the records' `invocation_id` and `name`. A record from inside an invocation joins its invocation's group, wherever it falls in the log; it also ends a run of records from a source, so the source's next record starts a new group.
- **The whole log is read,** and the level applied in the app. Changing the level needs no request, and the tab can tell an empty log from one the level hides.
- **Groups** are native `<details>` elements: opening and closing them needs no script, and the keyboard and screen readers know them. A chevron shows which way each will go.
- **Times** are in the browser's time zone. The log file's name is today's date in UTC, as the server names its files.
- **Download log file** fetches `logfile/` itself rather than through a download button, since the log isn't a Thing's.
- **No visual baseline:** the log's content and times differ on every run.
- **Size:** 669 changed lines of hand-written code, over M's 600 and my estimate of about 510. The project owner chose to keep one pull request, with no change to the phase's size.

  | Part | Lines |
  |---|---|
  | `LoggingPage.vue`, of which its styles 112 | 278 |
  | `log.ts` | 81 |
  | Routing | 4 |
  | Tests, with the fixture | 306 |

## Tests

- **`tests/unit/logging/log.spec.ts`,** against a fixture of a server's log modelled on real records (startup, two invocations, the camera between them, and an invocation whose Thing isn't known):
  - grouping: an invocation's records gathered, consecutive records from a source together, and a run broken by an invocation's record;
  - filtering by level, leaving out groups with nothing left;
  - each group's most severe record.
- **`tests/unit/logging/LoggingPage.spec.ts`,** against a fake server:
  - newest first, at INFO, with each group's level;
  - opening a group to its records, sources and error, and Expand all and Collapse all;
  - the level, and the message when it hides everything;
  - 10 groups to a page, and back to the first when the level changes;
  - an empty log, and Refresh;
  - Download log file, and its error.
- **`tests/e2e/logging.spec.ts`:** on the real server, a failed `download_zip` shows as an ERROR group, which opens to its message, and the log file downloads.

**Mutation checks:** I broke 16 behaviours, and a test failed for each.

- **Grouping:** invocations not gathered, any source joining a run, an invocation's record not ending a run, the level excluding itself, empty groups kept, and the first record taken as the most severe.
- **The tab:** oldest first, 20 to a page, DEBUG by default, the page kept when the level changes, Expand all doing nothing, Refresh doing nothing, a failed download not reported, sources and errors not shown, and no message when everything is hidden.

## Follow-ups and known gaps

- **The log isn't followed live:** Refresh reads it again. The server keeps its last 1000 records.
- **Most of the server's events** are at INFO or above, so DEBUG shows little unless the server runs with `--debug`.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
