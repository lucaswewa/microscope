# P44: Sidecar process contract

- Status: Done
- Pull request: [#57](https://github.com/lucaswewa/microscope/pull/57)
- ADRs: [ADR-0032](../../adr/0032-sidecar-process-contract.md)
- Spec: [phases.md#p44](../phases.md#p44)

## Summary

A stable contract between the server and whatever supervises it: the launcher in P45, and Tauri later.

- **`--managed`:** once its Things have started, the server prints `MICROSCOPE_READY {"url": …, "pid": …, "version": …}`, with the port that `--port 0` chose.
- **Closing standard input** stops a managed server gracefully, with exit code 0.
- **The `system` Thing** gains:
  - `managed`;
  - `shutdown`, which stops the server with 0;
  - `restart`, which stops a managed server with 75, for its supervisor to start it again.

## What was built

| Where | What |
|---|---|
| `crates/microscope-server/src/cli.rs` | `--managed`; one stop request, from Ctrl-C, standard input closing or the `system` Thing, and its exit code; the ready line |
| `crates/microscope-server/src/lifecycle.rs` | `serve` says when it's ready, once the Things have started |
| `crates/microscope-things/src/system.rs` | `Exit`, `Control` and `MicroscopeSystem::attach`; `managed`, `shutdown` and `restart` |
| `crates/microscope-server/tests/managed.rs` | The contract, through the binary |
| `docs/adr/0032-sidecar-process-contract.md` | ADR-0032 |
| `web/src/api/generated/` | The `system` Thing's new affordances |

## How to try it

In PowerShell:

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --managed --port 0
```

1. **The ready line,** `MICROSCOPE_READY {"url": …, "pid": …, "version": …}`, gives the URL with the port chosen, the process ID and the version.
2. **Restart:** at that URL's `/docs`, run `system/restart`. The server stops, and `$LASTEXITCODE` is 75.
3. **Standard input:** start it again, and in its window press Ctrl+Z, then Enter. Standard input closes, the server shuts down, and `$LASTEXITCODE` is 0.
4. **Shut down:** start it again, and run `system/shutdown`. `$LASTEXITCODE` is 0.
5. **Without `--managed`:** there's no ready line, `system/managed` is false, and `system/restart` fails, saying a supervisor must have started the server.

## Design notes and deviations from the plan

- **`--port 0` works without `--managed` too,** as it did before: the fallback test relies on it. Only the ready line says which port it chose, in a form a program can read.
- **One stop request:** Ctrl-C, standard input closing and the `system` Thing's actions all ask through one channel, and the first asks wins. The server then shuts down as it always has: streams ended, invocations cancelled, the grace period, then the Things stopped.
- **Standard input is read on a thread of its own,** which ends with the process. Tokio's standard input would hold the runtime open at shutdown until a line arrived.
- **The `system` Thing gets its control** from the server once it's built, as the server log gets the invocations. A `system` Thing without one, as in a test harness, says so if asked to stop.
- **Neither action takes the global lock,** so the server can be stopped while a scan or sequence holds it.
- **The ready line comes once the Things have started,** not merely once the port is open, so a supervisor can open the browser straight away.
- **A server listening on every address** gives the loopback address in its URL.
- **Size:** 448 changed lines of hand-written code, within M's 600 but over my estimate of about 300: the tests came to 272.

  | Part | Lines |
  |---|---|
  | `cli.rs` and `lifecycle.rs` | 107 |
  | The `system` Thing | 69 |
  | Tests | 272 |

## Tests

- **`tests/managed.rs`,** spawning the binary on a free port:
  - the ready line: its URL reached on the loopback address, its process ID and version, and `managed` true;
  - closing standard input: 0 within the grace period;
  - `restart`: 75;
  - `shutdown`: 0;
  - without `--managed`: no ready line, standard input closing ignored, `managed` false, `restart` refused, and `shutdown` still 0.
- **`system.rs`:** `managed`, `shutdown` and `restart` without a control, unmanaged and managed, and what each asks.
- **`cli.rs`:** the ready line's URL for loopback, every-address and other addresses.
- **`tests/lifecycle.rs`:** a server whose Thing can't start is never ready.

**Mutation checks:** I broke 12 behaviours, and a test failed for each.

- **The command line:** standard input watched without `--managed`, its closing ignored, a restart exiting with 0, the control not attached, the ready line printed without `--managed`, no loopback address for every address, and no process ID.
- **The lifecycle:** never ready, and ready before the Things start.
- **The `system` Thing:** never managed, a shutdown that restarts, and a restart allowed without a supervisor.

A first run found that the tests hung, rather than failed, when the ready line never came. They now wait at most 30 s for it.

## Follow-ups and known gaps

- **The launcher** (P45) supervises the server by this contract.
- **The Power tab** (P46) offers Shut down, and Restart when `managed` is true.
- **Untested:** that the first request to stop wins, such as a restart followed at once by standard input closing.
- **The fallback server** prints no ready line (ADR-0032).
- **A timing-sensitive P30 test,** `measures_how_far_the_focus_moves_at_the_ends`, failed once in the full workspace run: it measured −2994 steps against −3000, outside its tolerance of 3. Run alone, it passed twice. Like P36's dark stack test (P41's notes), it depends on preview frames during a sweep; this phase doesn't touch it.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
