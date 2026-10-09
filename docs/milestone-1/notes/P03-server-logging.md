# P03: Server logging and log endpoints

- Status: In review
- Pull request: to be linked once opened
- ADRs: [ADR-0008](../../adr/0008-server-logging-and-log-endpoints.md)
- Spec: [phases.md#p03](../phases.md#p03)

## Summary

The server now keeps a log that the app can show and download. Every event goes to the console, to its invocation's log (as before), to an in-memory server log of the last 1000 records served at `/api/v1/log/`, and to a daily log file served at `/api/v1/logfile/`. Records from actions name their invocation, Thing and action, which the Logging tab (P43) will group by.

## What was built

| Where | What |
|---|---|
| `microscope-server/src/logging.rs` | `init` (the subscriber with four outputs), `Logs` (the server log: records by level, text for the fallback page, the current log file, `attach` to the invocation manager), the layer that records events and credits them to invocations, `capture_filter`, `parse_level` |
| `microscope-server/src/routes.rs` | `GET {prefix}/log/` (with `?level=`) and `GET {prefix}/logfile/`, with and without the trailing slash; errors as `{"detail": …}` |
| `microscope-server/src/cli.rs` | Logging starts once the configuration names the log folder; the server log is attached to the invocation manager once the server is built; the fallback page shows the log so far |
| `microscope-server/tests/logging.rs` | The end-to-end test, in a test binary of its own because it installs the global subscriber |

A server-log record (from a real run):

```json
{
  "sequence": 1,
  "name": "microscope_server::cli",
  "message": "application configuration data_folder=.microscope/data log_folder=.microscope/logs",
  "levelname": "INFO",
  "levelno": 20,
  "lineno": 170,
  "filename": "cli.rs",
  "created": "2026-10-09T04:37:03.387255Z",
  "exception_type": null,
  "traceback": null,
  "invocation_id": null,
  "thing": null,
  "action": null
}
```

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json
```

Then open:

- <http://127.0.0.1:5000/api/v1/log/>, and the same with `?level=WARNING`;
- <http://127.0.0.1:5000/api/v1/logfile/>, which downloads `microscope.<today>.log`.

The files are in `.microscope/logs/`. Add `--debug` to capture DEBUG events from the application and `teta-wot`.

## Design notes and deviations from the plan

- **Record fields.** The spec listed `exception`; the records have `teta-wot`'s `exception_type` and `traceback` instead, so they match invocation logs exactly. A `sequence` number was also added, as OpenFlexure has, for stable group keys in the Logging tab. ADR-0008 explains both.
- **Crediting events to actions.** Invocation spans carry only an ID. The Thing and action come from `teta-wot`'s invocation manager, looked up when the event is recorded. That's safe at `teta-wot` v0.1.0 because the manager never logs while holding its lock, and ADR-0008 asks for a re-check on upgrades.
- **Logging now starts after the configuration is read,** not before, because the configuration names the log folder. Nothing was logged before that point anyway: configuration errors are printed.
- **An unwritable log folder** doesn't stop the server. It warns, on the console and in the server log, and `/logfile/` answers 404.
- **The fallback page shows the server log**, using the `logging` field of `teta-wot`'s `FallbackPage`, which P02 left empty. The spec didn't ask for this; OpenFlexure's fallback page shows its log history.
- **`?level=CRITICAL` is accepted** for the Logging tab's filter, as OpenFlexure has that level, but `tracing` has nothing above ERROR, so it returns nothing.
- **The application's routes aren't in `/openapi.json`,** which `teta-wot` builds from the Things.
- **The command-line tests** now run in temporary folders, since the server writes `.microscope/logs` relative to its working directory.
- **New dependencies:** `tracing-subscriber` and `chrono` (already used by `teta-wot`), `uuid`, `tracing-appender` (MIT). Dev-only: `tempfile`. Workspace `tokio` gains the `fs` feature.

## Tests

32 tests across the workspace, all passing locally on Rust 1.98.1. New in this phase:

- **The server log** (9 unit tests in `logging.rs`):
  - capacity and ordering (1005 events keep the last 1000, with sequence numbers);
  - the record's fields and JSON keys;
  - `exception_type` and `traceback` kept out of the message;
  - filtering by level;
  - invocation IDs from nested invocation spans;
  - the debug-mode capture filter (this application's and `teta-wot`'s DEBUG events, not hyper's);
  - level names and numbers;
  - choosing the newest log file;
  - the fallback-page text.
- **End to end** (`tests/logging.rs`): an action logs; the line appears in the invocation's log and in `/api/v1/log/`, credited to `chatty` and `speak` with the invocation's ID; `?level=WARNING` filters; `?level=loud` answers 400; `/api/v1/logfile/` downloads the file containing the line.
- **Command line** (updated): the fallback page includes the server log, and the default log folder and file are created in the working directory.

Manual check: the server was run with `configs/simulation.json`, and `/api/v1/log/`, `?level=warning` and `/api/v1/logfile/` answered as shown above.

## Follow-ups and known gaps

- The Logging tab is P43.
- Only the newest log file can be downloaded. Older days stay in the folder for 30 days.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. Only the route paths and the idea of crediting records to actions follow OpenFlexure's behaviour. The record formatting follows `teta-wot`'s own code (same owner, MIT).
