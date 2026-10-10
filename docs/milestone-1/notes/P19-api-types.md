# P19: Generated API types and typed facades

- Status: In review
- Pull request: (added once opened)
- ADRs: [ADR-0021](../../adr/0021-generated-api-types.md)
- Spec: [phases.md#p19](../phases.md#p19)

## Summary

The web app's API types now come from the server, and can't drift from it:

- **`microscope-server --print-openapi`** writes the configured Things' OpenAPI document, without starting them.
- **Generated files:** `npm run api:openapi` and `npm run api:types` turn it into `web/src/api/generated/openapi.json` and `schema.d.ts`, both committed.
- **CI** fails when either is stale. The server's tests compare the document, and the web job checks the types.
- **Typed facades:** `TypedThing<'stage'>` (and `'system'`, `'camera'`, `'illumination'`) wrap the P10 client. Property, action and event names, and their values, are checked against the generated types. `useThing('stage')` gives one while connected.

## What was built

| Where | What |
|---|---|
| `crates/microscope-server/src/cli.rs` | `--print-openapi`, which asks the built server's router for `/openapi.json` in-process |
| `crates/microscope-server/tests/cli.rs` | The committed document must equal what `--print-openapi` writes |
| `web/src/api/generated/` | `openapi.json` and `schema.d.ts`, generated and committed |
| `web/package.json` | `openapi-typescript` 7.13.0, the `api:openapi` and `api:types` scripts, and an override for its TypeScript peer |
| `web/src/api/things/index.ts` | `TypedThing`, `useThing`, and the `Position`, `AxisScale` and `StreamingMode` types |
| `web/tests/unit/api/things.spec.ts` | The tests below |
| `.github/workflows/ci.yml`, `CLAUDE.md` | The types check in CI, and the commands to regenerate |

## How to try it

From `web/`:

```powershell
npm run api:openapi      # builds the server and writes openapi.json
npm run api:types        # writes schema.d.ts
npm run api:types -- --check
```

Then, in any `.ts` file, `new TypedThing<'stage'>(thing).write('position', …)` is an error: `position` is read-only.

## Design notes and deviations from the plan

ADR-0021 records the design. Its details:

- **Where the document is served:** `teta-wot` serves the OpenAPI document and the docs pages at the root, as FastAPI does, not under the API prefix. The P16, P17 and P18b notes pointed at `/api/v1/docs`, which doesn't exist. They now say `http://127.0.0.1:5090/docs`.
- **Facades:** one generic `TypedThing<T>` instead of a hand-written class per Thing. Each Thing's affordances are found by the generated schemas' names (`stage_position_value`, `stage_move_relative_input`), so a new property is typed as soon as the types are regenerated, and nothing hand-written lists them.
  - **Writes:** a property can be written when its OpenAPI path has a PUT.
  - **Inputs:** an action's input can be left out when nothing in it is required.
  - **`run`** invokes an action and returns its typed output; **`invoke`** returns the invocation, for cancelling or following it.
- **Inputs with defaults are optional** (`--default-non-nullable false`): the server fills them in. Values the server sends keep their fields required.
- **Tied to the shipped configuration:** the types come from `configs/simulation.json`, so they carry its names and the `/api/v1` prefix (ADR-0021 discusses this).
- **The deviation in tooling:** `openapi-typescript` declares TypeScript 5 as a peer, and the app uses 6. An npm override gives it TypeScript 6. The generated types compile, and the checks below show they're right.
- **Dependencies:** 27 new dev packages, under MIT, ISC, Apache-2.0, PSF (`argparse`) and MIT-or-CC0 licences, all published before 2026-09-26. `npm audit` reports the same five advisories as `main`, all in existing dev tooling, and nothing new.
- **Size:** 294 changed lines of hand-written code (about 110 of them tests), within S's budget of 300. The generated files and the lock file don't count.

## Tests

- **Rust** (`tests/cli.rs`, 1 new): `--print-openapi` exits 0 with a document that has the stage's `move_relative`, and equals the committed `openapi.json`. Changing one description in the committed file fails it, with a message saying how to regenerate.
- **Web** (`tests/unit/api/things.spec.ts`, 2 new):
  - against the fake server, a typed stage reads its position, writes `backlash_steps`, runs `move_relative`, and receives `arrived`, with the values typed as `Position`;
  - at compile time, an unknown property, writing a read-only property, a value of the wrong type, and `save_from_memory` without a path are each errors (`@ts-expect-error`). Leaving out an optional input isn't.
- **Checked by breaking:**
  - making every property writable, making every input optional, or marking a valid read as an error each makes `npm run typecheck` fail;
  - an edited `schema.d.ts` fails `npm run api:types -- --check`.

## Follow-ups and known gaps

- **P20** builds the Control tab on `useThing('stage')` and `useThing('camera')`.
- **Hand-written types:** the P10–P18 code that predates the facades (the connection store, `LiveImage`) uses the untyped client where it reads little. It can move to the facades as it's next changed.
- **The upstream peer:** the `openapi-typescript` override goes once a release declares TypeScript 6.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. OpenFlexure's web app has no generated types; this is this project's own design.
