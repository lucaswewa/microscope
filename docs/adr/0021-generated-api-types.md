# ADR-0021: Generated API types

- Status: Accepted
- Date: 2026-10-10
- Phase: P19

## Context

- The web app talks to the server through Thing Descriptions (ADR-0014). Until now, the types it used (a position, a move's input) were written by hand, and nothing kept them in step with the Rust structs they mirror.
- `teta-wot` writes an OpenAPI 3.1 document for the server's Things, with a schema per affordance:
  - `{thing}_{property}_value`;
  - `{thing}_{action}_input` and `{thing}_{action}_output`;
  - `{thing}_{event}_data`.

  It's generated from the same Rust types the server uses, and the server serves it at `/openapi.json`.
- Things change in almost every phase from here on, and the web app grows a view for each.

## Decision

- **The OpenAPI document is the source of the web app's API types.**
  - **Writing it:** `microscope-server -c configs/simulation.json --print-openapi` builds the configured Things, without starting them or listening, and prints the document the server would serve.
  - **Committing it:** it's committed as `web/src/api/generated/openapi.json`, so a change to the API shows in a pull request's diff.
- **`openapi-typescript`** turns it into `web/src/api/generated/schema.d.ts`, also committed.
  - **Defaults:** with `--default-non-nullable false`, an input with a default is optional, as the server treats it.
  - **Scripts:** `npm run api:openapi` refreshes the document from a debug build of the server, and `npm run api:types` refreshes the types.
  - **Formatting:** Prettier and ESLint leave the generated folder alone.
- **Two checks catch stale files:**
  - **The document:** the server's CLI tests compare `--print-openapi`'s output with the committed document, so `cargo test` fails when it's stale.
  - **The types:** CI's web job runs `npm run api:types -- --check`, which fails when the types don't match the committed document.

  Together, a change to a Thing can't be merged with types that disagree with it.
- **Typed facades** (`web/src/api/things`): `TypedThing<'stage'>` wraps the P10 client's `ConsumedThing`. Its `read`, `write`, `observe`, `invoke`, `run` and `subscribe` take affordance names and values whose types are looked up from the generated schemas by name.
  - **Writes:** only properties the server accepts a PUT for can be written.
  - **Inputs:** an action's input can be left out only when nothing in it is required.
  - **Getting one:** `useThing('stage')` gives the facade while connected.
- **TypeScript 6:** `openapi-typescript` 7.13.0 declares a peer dependency on TypeScript 5, and the app uses TypeScript 6. An npm `overrides` entry gives it the app's TypeScript. It generates the same output, and the type checks below pass.

## Consequences

- **Changing a Thing** means regenerating the two files (CLAUDE.md lists the commands) and fixing whatever no longer compiles. That's the point: the compiler finds the web code a server change affects.
- **Compile-time errors:** a misspelled property, writing a read-only setting, a value of the wrong shape, or an action called without a required input. A test file asserts each of these with `@ts-expect-error`.
- **Tied to the shipped configuration:** the types are generated from `configs/simulation.json`, so the schema names carry its Thing names (`stage`, `camera`) and its prefix (`/api/v1`). A Thing of the same class under another name uses the same facade type, `new TypedThing<'stage'>(…)`.
- **The document is large** (about 200 KB, and its types about 6,000 lines), and grows with each Thing. It's data, reviewed by its diff, and doesn't count toward phase budgets.
- **Responses are trusted:** `TypedThing` returns what the server sends, without validating it. The checks above keep the types and the server in step at build time.

## Alternatives considered

- **Hand-written types.** They drift silently. The point of this phase is that they can't.
- **Generating types from the Thing Descriptions.** TDs use JSON Schema too, but there's no mature generator for them, and OpenAPI is what the server already documents its HTTP API with.
- **A generated client** (openapi-fetch, or an OpenAPI code generator). The P10 client already speaks the WoT forms: SSE observation, polled invocations and the lock. A second, path-based client would duplicate it. Only the types are needed.
- **Generating at build time, without committing.** Every web build would need a Rust toolchain and a server build, and API changes wouldn't show in review.
- **Waiting for an `openapi-typescript` release that declares TypeScript 6,** or pinning the app to TypeScript 5. The override is one line and is removed when upstream catches up.
