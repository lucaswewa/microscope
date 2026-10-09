# P02: Server lifecycle and composed router

- Status: In review
- Pull request: to be linked once opened
- ADRs: [ADR-0006](../../adr/0006-own-the-server-lifecycle-around-teta-wots-runtime.md), [ADR-0007](../../adr/0007-api-prefix-and-openflexure-mirrored-names.md)
- Spec: [phases.md#p02](../phases.md#p02)

## Summary

`microscope-server` now serves Things from a configuration file. It runs the start → serve → shutdown lifecycle itself instead of calling `teta-wot`'s `serve`, so that its own routes (`/api/v1/health` for now) sit in front of the Web of Things API without any change to `teta-wot`. The first Thing, `system`, reports the hostname, the server's version and the OS.

## What was built

| Where | What |
|---|---|
| `microscope-things/src/system.rs` | The `system` Thing (`MicroscopeSystem`): properties `hostname`, `version_data` (`version`, `version_source`, mirroring OpenFlexure) and `os_version` |
| `microscope-things/build.rs` | Records `git describe --tags --always` at build time for `version_source` (`unknown` outside a checkout) |
| `microscope-things/src/lib.rs` | `registry()`: Thing types by class name (`microscope.system:MicroscopeSystem`) |
| `microscope-server/src/cli.rs` | The command line: `teta-wot`'s `CliArgs` flattened (`-c`, `-j`, `--host`, `--port`, `--debug`, `--fallback`) plus `--version`; `teta-wot`'s messages and exit codes; the fallback page |
| `microscope-server/src/config.rs` | Reading `-c`/`-j`; `teta-wot`'s `ServerConfig` plus the typed `application_config` (`data_folder`, `log_folder`, with defaults) |
| `microscope-server/src/lifecycle.rs` | `compose` (application routes first, `teta-wot`'s router as the fallback service) and `serve` (`serve_with`'s lifecycle, step by step) |
| `microscope-server/src/routes.rs` | `GET {prefix}/health` (`status`, `version`, `uptime_seconds`), with CORS matching `teta-wot`'s policy |
| `configs/simulation.json` | The development configuration: `system` only for now, `/api/v1`, the global lock on, data under `.microscope/` |

`microscope-server` is now a library plus a thin binary, as ADR-0005 allowed, so integration tests can use its modules.

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json
```

Then open:

- <http://127.0.0.1:5000/api/v1/health>: `{"status":"ok","version":"0.1.0","uptime_seconds":…}`
- <http://127.0.0.1:5000/api/v1/system/>: the `system` Thing's description; `…/system/hostname`, `…/version_data`, `…/os_version`
- <http://127.0.0.1:5000/docs>: the interactive API docs

Press Ctrl-C to stop it. To see the fallback page, run `cargo run -p microscope-server -- --fallback -j "{\"things\": {\"x\": \"no.such:Thing\"}}"` and open <http://127.0.0.1:5000/>.

## Design notes and deviations from the plan

- **Where `teta-wot` serves what.** Under `/api/v1`: the Things, `/thing_descriptions/`, `/things/`, `/action_invocations` and `/directory/things`. At the root, whatever the prefix: `/docs`, `/redoc`, `/openapi.json` and `/.well-known/wot`. The phase spec mentioned "the API docs" without a path; they are at `/docs`. ADR-0007 records this, and P12 must keep the web app clear of these paths.
- **CORS for the application's routes.** The spec didn't mention CORS. `teta-wot`'s CORS middleware is private and only wraps its own router, so the application's routes get their own layer with the same policy (tower-http, very permissive, with Private Network Access). The web app will call `/api/v1/health` and later routes cross-origin, from Tauri or remote connections. A test checks that `teta-wot`'s routes still carry exactly one set of CORS headers.
- **`application_config` keys are optional,** defaulting to `.microscope/data` and `.microscope/logs` (git-ignored since P01). Unknown keys are ignored, and keys of the wrong type make the configuration invalid (exit code 3). Nothing uses the folders yet. P03 uses `log_folder`, and P26 uses `data_folder`.
- **`mdns: true` is warned about and ignored** (ADR-0006).
- **The hostname** comes from the `gethostname` crate rather than `teta-wot`'s `default_server_id()`, which reads `COMPUTERNAME`, upper-cased and limited to 15 characters on Windows. The OS description comes from `os_info`, for example `Windows 10.0.26200 (Windows 11 Professional) [64-bit]`.
- **Size.** The phase came out larger than its M estimate: 1,131 changed lines, or 835 without blank lines and comments, of which about 415 are tests. That's over 900 by the first measure and under it by the second. It stays one phase because the command line and the lifecycle only make sense together. Splitting it would leave a server that can't be run. The plan doesn't say whether comments and tests count towards the budget. If they should, P02 could be split, with the command line in its own pull request.
- **New dependencies**: `clap`, `tokio`, `serde`, `serde_json`, `tracing` (all already used by `teta-wot`), `tower-http` (CORS), `gethostname` and `os_info`. Dev-only: `anyhow`, `tower`, `http-body-util`. All are MIT or Apache-2.0.

## Tests

22 tests, all passing locally on Rust 1.98.1:

- **`system`** (3): `version_data` contents; the properties served over HTTP (`TestClient`); the Thing alone in a `Harness`.
- **Configuration** (4): defaults for missing keys, an invalid `application_config`, and the shipped `configs/simulation.json` (prefix `/api/v1`, global lock on).
- **Composed router** (5, `tests/routes.rs`): health under the prefix; Things served behind the application's routes; unmatched requests get exactly `teta-wot`'s answer (status and body); CORS on the application's routes, including the preflight; `teta-wot`'s CORS headers appear once.
- **Lifecycle** (2, `tests/lifecycle.rs`):
  - a shutdown while a one-minute action runs cancels it (the action sees the cancellation), stops the Thing, and finishes in well under half the grace period;
  - a Thing whose `on_start` fails gives `ServeError::Startup` naming it.
- **Command line** (8, `tests/cli.rs`, through the built binary): `--version` (0); an invalid option (2); no configuration, a missing file, or both `-c` and `-j` (1); an unknown Thing type or an invalid `application_config` (3); and `--fallback` serving the error page on the port it prints.

Manual check: the server was run with `configs/simulation.json`, and every URL in "How to try it" answered as expected, including the CORS headers.

## Follow-ups and known gaps

- Logging still goes to the console only; P03 adds the ring buffer, the log file and the log routes.
- Ctrl-C itself wasn't tested by sending a console signal. The shutdown path is tested through the `shutdown` future that Ctrl-C completes.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. `cli.rs` and `lifecycle.rs` follow `teta-wot`'s own MIT-licensed code (same owner), as ADR-0006 describes.
