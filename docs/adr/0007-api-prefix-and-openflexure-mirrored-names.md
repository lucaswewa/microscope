# ADR-0007: API prefix `/api/v1` and OpenFlexure-mirrored names

- Status: Accepted
- Date: 2026-10-09
- Phase: P02

## Context

- OpenFlexure serves its API under `/api/v3`, with Things named `camera`, `stage`, `autofocus`, `camera_stage_mapping`, `smart_scan`, `gallery`, `system`, … Planning decided to mirror those names where they fit, without promising compatibility ([plan, D7](../milestone-1/implementation-plan.md#3-decisions-made-while-planning)).
- The web app will be served at `/` (P12), so the API needs a prefix to keep clear of it.
- `teta-wot` puts the Things, `/thing_descriptions/`, `/things/`, `/action_invocations` and `/directory/things` under the configured `api_prefix`. It always serves the docs pages (`/docs`, `/redoc`), the OpenAPI document (`/openapi.json`) and discovery (`/.well-known/wot`) at the root.
- Configuration files name each Thing's class. LabThings uses Python import strings, and `teta-wot` accepts any name (`a.b:C`, `a.b.C` and `a::b::C` are equivalent) as well as the short Rust type name.

## Decision

- **The API prefix is `/api/v1`.** Every configuration this project ships sets `"api_prefix": "/api/v1"`. The application's own routes go under the same prefix (`/api/v1/health`, later `/api/v1/log/`, `/api/v1/logfile/`, `/api/v1/data/…`).
- **`v1` is this project's API version,** not OpenFlexure's. A breaking change to the API moves it to `/api/v2`.
- **Thing names mirror OpenFlexure's** where the concept is the same: `system`, `camera`, `stage`, `illumination`, `autofocus`, `camera_stage_mapping`, `stage_measure`, `gallery`, `smart_scan`, the workflow Things and the background detectors. Affordance names follow the same rule (`position`, `move_absolute`, `fast_autofocus`, `sample_scan`, …), in snake_case. Where our concept differs, the name differs too. The planned case is `sequence` for OpenFlexure's `timelapse` (P41). Each phase's notes list the deviations it makes.
- **Class names** in configuration files look like import strings, `microscope.<module>:<Type>` (for example `microscope.system:MicroscopeSystem`), and are registered in `microscope_things::registry()`.
- **There is no compatibility promise.** OpenFlexure's clients may work where the names and shapes happen to match, but nothing is tested against them.

## Consequences

- The web app, the API and `teta-wot`'s root-level pages don't collide.
- OpenFlexure users and documentation map easily onto this API.
- The prefix lives in the configuration file, not in code. The server builds its own routes from the configured prefix, so a configuration without `/api/v1` would move them along with the Things, and the web app, which will look for the API under `/api/v1` (P10), wouldn't find it. A test checks that the shipped configuration uses `/api/v1`.

## Alternatives considered

- **`/api/v3`, as OpenFlexure uses.** It would suggest a compatibility we don't promise.
- **No prefix.** It would clash with the web app at `/`, and with any future root-level routes.
- **Our own names throughout.** They'd be cleaner in places, but we'd lose the familiarity that helps both the parity work and OpenFlexure users.
