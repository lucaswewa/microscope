# ADR-0032: Sidecar process contract

- Status: Accepted
- Date: 2026-10-14
- Phase: P44

## Context

- **A supervisor runs the server** for a user who doesn't start it from a terminal: the launcher in milestone 1 (P45), and the Tauri desktop app later (plan D5).
- **It needs to know** when the server is ready and where, how to stop it, and whether it asked to be started again.
- **Windows** has no portable way to send a child process a polite stop signal. Ctrl-C and Ctrl-Break reach every process on a console, and a process started without one gets none.
- **The app's Power tab** (P46) offers Shut down and Restart, and needs to know whether a restart is possible.

## Decision

- **`microscope-server --managed`** runs under a supervisor. Without it, nothing below changes but the `system` Thing's actions.
- **The ready line:** once its Things have started and it's serving, the server prints one line to standard output:

  ```text
  MICROSCOPE_READY {"url": "http://127.0.0.1:5000", "pid": 1234, "version": "0.1.0"}
  ```

  - **`url`** is where it serves, with the port that `--port 0` chose. A server listening on every address gives the loopback one.
  - **`pid`** and **`version`** are the server process's and its build's.
  - **The supervisor keeps reading** standard output afterwards, since the console log goes there too.
- **Stopping:** the supervisor closes the server's standard input. The server then shuts down gracefully, as it does for Ctrl-C: it ends streams, cancels invocations, waits up to the grace period, and stops its Things. What's written to standard input is ignored.
- **Exit codes:**

  | Code | Meaning |
  |---|---|
  | 0 | Stopped: standard input closed, Ctrl-C, or `system.shutdown` |
  | 75 | `system.restart`: start it again |
  | 1, 2, 3 | Errors, as in P02: general, the command line, the configuration or a Thing's start |

- **The `system` Thing:**
  - **`managed`,** a property: whether a supervisor started the server;
  - **`shutdown`,** an action that stops the server with 0, managed or not;
  - **`restart`,** an action that stops it with 75, refused unless it's managed.

  Neither action takes the global lock, so they work while a scan or sequence holds it. The server gives the Thing a control handle once it's built; the first request to stop wins.
- **`--port 0`** picks a free port with or without `--managed`. Only the ready line says which, in a form a program can read.

## Consequences

- **The launcher (P45)** waits for the ready line, opens `url`, closes standard input to stop, restarts on 75, and treats 1 to 3, or an exit before the ready line, as a crash.
- **Tauri** can run the same binary as a sidecar, with the same contract.
- **A supervisor that leaves standard input unconnected** stops the server at once: it must pipe it and keep it open.
- **The fallback server** (`--fallback`) prints no ready line. A supervisor sees exit code 3 once it's stopped.

## Alternatives considered

- **Signals or console events to stop the server:** Windows can't send them to one child process without disturbing others on the console.
- **An HTTP request to stop it:** it needs the server's address and to be listening, and a stuck server can't answer. Closing standard input works whenever the process runs.
- **A file the server writes with its port:** it can be stale, and the supervisor would have to poll for it. The ready line arrives once and in order.
- **Restarting from inside the server:** the supervisor would have to tell a crash from a restart some other way. An exit code says it.
