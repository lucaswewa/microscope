# ADR-0008: Server logging and log endpoints

- Status: Accepted
- Date: 2026-10-09
- Phase: P03

## Context

- The Logging tab (P43) shows the server's recent log, grouped by the action that produced it, with a level filter and a log-file download. Action dialogs show an invocation's own log, which `teta-wot` already keeps and serves with the invocation.
- OpenFlexure keeps its last 250 records in memory, each with its invocation ID and calling action, and serves them at `/log/`, with the log file at `/logfile/`. Its record keys are its own (`timestamp`, `level`, `logger`, `calling_action`, `sequence`, …).
- `teta-wot` has an invocation-log layer and a public `LogRecord` that uses Python's logging attribute names (`message`, `levelname`, `levelno`, `lineno`, `filename`, `created`, `exception_type`, `traceback`). Its invocation spans carry only `invocation_id`, and the Thing and action names live in its `InvocationManager`. `teta_wot::logging::init` installs only the console and the invocation layer.

## Decision

**Four outputs.** One subscriber, installed by `logging::init`:

1. the console, filtered by `RUST_LOG` (`info` by default, `debug` with `--debug`);
2. `teta-wot`'s invocation-log layer, unchanged;
3. the server log: the last 1000 records in memory;
4. daily log files in `application_config.log_folder`, named `microscope.YYYY-MM-DD.log` (the date in UTC), with the last 30 kept (tracing-appender).

**What the server log and the files capture:** INFO and more severe. With `--debug`, they also capture DEBUG events from this application's crates and `teta-wot`'s, but not from other libraries, whose chatter would crowd the 1000 records out.

**The record:** `teta-wot`'s `LogRecord` fields, flattened, plus:

- `sequence`, counting from 1;
- `name`, the event's target (Python's logger name);
- `invocation_id`, `thing` and `action`.

This keeps the server log and the invocation logs in one shape. Python's attribute names are used rather than OpenFlexure's own keys; the web app does any mapping it needs (P43).

**Crediting events to actions:** the innermost invocation span that the `InvocationManager` knows wins, so events from in-process calls are credited to the action a client invoked. Failing that, the innermost invocation's ID is kept without names. The names are looked up while the event is recorded, so they survive the invocation's expiry. This is safe because `InvocationManager` (at `teta-wot` v0.1.0) never logs while holding its lock. Upgrades of `teta-wot` re-check it.

**Routes**, beside `/health`, with and without the trailing slash:

- `GET {prefix}/log/` answers the records as JSON, oldest first. `?level=` keeps that level and above, given as a name (`DEBUG` … `CRITICAL`, any case) or a number. An unknown level answers 400 `{"detail": …}`.
- `GET {prefix}/logfile/` answers the newest log file as a `text/plain` attachment, or 404 `{"detail": …}` if there's none.

**When logging starts:** once the configuration has been read, since it names the log folder. Problems before that are printed, as before. If the folder can't be created, logging carries on without files and warns on the console and in the server log, and `/logfile/` answers 404.

**Writes:** log files are written synchronously, with the appender as the writer, so a download includes every line logged so far.

**The fallback page** shows the server log so far.

## Consequences

- The web app reads one record shape for the server log and for invocation logs.
- The `tracing` subscriber is process-wide, so tests that need it run in a test binary of their own (`tests/logging.rs`).
- Synchronous writes put file I/O on the logging thread. That's fine at an instrument's log volume. If heavy debug logging makes it a problem, switch to tracing-appender's non-blocking writer.
- Log files accumulate for up to 30 days.
- `teta-wot` builds the OpenAPI document from the Things, so the application's routes aren't in it. They're documented in `routes.rs` and here.

## Alternatives considered

- **OpenFlexure's record keys.** The server log would then differ from the invocation logs `teta-wot` serves.
- **Looking the names up when `/log/` is read.** Invocations are forgotten five minutes after they finish, so older records would lose their names.
- **tracing-appender's non-blocking writer.** It needs a guard kept alive for the process, and a download could miss the latest lines. There's no need for it at this volume.
- **Serving `/log/` by parsing the log files.** Slower, and it ties the API to the file format.
- **Capturing every library's DEBUG events with `--debug`.** It would bury the application's own messages.
