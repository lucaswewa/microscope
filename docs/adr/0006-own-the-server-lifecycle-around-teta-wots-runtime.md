# ADR-0006: Own the server lifecycle around `teta-wot`'s runtime

- Status: Accepted
- Date: 2026-10-09
- Phase: P02

## Context

The server needs routes of its own beside the Web of Things API: `/api/v1/health` now, and later the log and log-file downloads (P03), the gallery's data files (P31) and the web app at `/` (P12).

At `teta-wot` v0.1.0:

- `ThingServer::serve` and `serve_with` serve only `teta-wot`'s router, and there's no hook to add routes or a fallback to it.
- That router is a single fallback handler (`Router::new().fallback(dispatch)`) wrapped in `teta-wot`'s CORS middleware. Two routers that both have fallbacks can't be merged.
- What `serve_with` uses is public: `ThingServer::runtime()` (start, stop, broker, invocations), `ThingServer::router()`, `ThingServer::shutdown_grace()`, and `fallback_router(FallbackPage)` for the error page.
- The mDNS advertiser is private to `teta-wot`.

ADR-0004 rules out changing `teta-wot` during milestone 1.

## Decision

`microscope-server` builds its `ThingServer` as usual (`from_config` with the registry) but never calls `serve` or `serve_with`. Instead:

- **It composes an outer router:** the application's routes first, with `teta-wot`'s router as the fallback service, so every request the application doesn't answer reaches `teta-wot` unchanged (`lifecycle::compose`).
- **It runs `serve_with`'s lifecycle itself** (`lifecycle::serve`), in the same order: start the Things, serve until the shutdown signal, stop accepting connections, close the message broker (ending SSE and WebSocket streams), cancel invocations, wait up to the grace period for requests and invocations, then stop the Things in reverse order.
- **It keeps `teta-wot`'s command line:** it flattens `teta-wot`'s `CliArgs` into its own arguments and keeps the same messages, exit codes (0, 1, 2, 3) and `--fallback` page.
- **The application's routes carry their own CORS layer** (tower-http, every origin, method and header, with credentials and Private Network Access), the same policy as `teta-wot`'s, whose middleware is private and wraps only its own router.
- **mDNS isn't supported.** A configuration with `"mdns": true` is accepted, and the server warns that the key is ignored.

## Consequences

- Application routes sit beside the API with no change to `teta-wot`, and the web app can be served from the same origin.
- `lifecycle.rs` and `cli.rs` duplicate about 150 lines of `teta-wot`'s behaviour. They can drift when `teta-wot` changes. The modules say which version they copy. Tests pin the behaviour that matters: a shutdown cancels running invocations and stops the Things within the grace period; a Thing that can't start is a startup error; unmatched requests get exactly `teta-wot`'s answer; CORS headers appear once; and the exit codes and fallback page match.
- **Upgrading `teta-wot`** (ADR-0004) includes comparing `ThingServer::serve_with` and `cli::serve_until` with these modules.
- **Retirement:** if `teta-wot` gains a way to add routes, or a fallback, to its server, this ADR should be superseded and the copies deleted. That is on ADR-0004's list of upstream candidates.

## Alternatives considered

- **Add a router hook to `teta-wot`.** The cleanest fix, but the owner chose to keep `teta-wot` unchanged in milestone 1 (ADR-0004).
- **Serve the application's routes on a second port.** No duplication, but two origins means more CORS and connection handling in the web app, and two ports to configure, document and supervise.
- **Put a reverse proxy in front.** It needs another process or library, and still duplicates the routing decisions.
- **Use `teta-wot`'s `http::router` with a runtime built by hand.** It would duplicate more: the mapping from configuration to runtime, which `ThingServer::from_config` already does.
