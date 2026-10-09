# ADR-0010: Rules for a Tauri-ready web app

- Status: Accepted
- Date: 2026-10-09
- Phase: P04

## Context

The same web app will run in three places:

- served by the microscope server, in a browser tab;
- from Vite during development;
- later, inside a Tauri desktop app, loaded from the app's own files through a custom protocol, while it talks to a local sidecar or a remote microscope ([plan §4.2](../milestone-1/implementation-plan.md#42-a-tauri-ready-web-app)).

Code that assumes a browser tab, a same-origin server or browser-only APIs would have to be found and rewritten later. The rules are cheapest to follow from the first line of code.

## Decision

1. **Routes live in the URL's hash** (`/#/control`), with Vue Router's `createWebHashHistory`. That works under Tauri's custom protocol and on any static host. It also means the server only serves `/` and `/assets/…`, which keeps clear of `teta-wot`'s routes at the root (`/docs`, `/openapi.json`, `/.well-known/wot`; ADR-0007).
2. **Every server URL comes from the active connection:** its base URL plus the forms in the Thing Descriptions. The page's own location is used only to propose the default "this server" connection (P11).
3. **Platform services go through the `HostAdapter`** (`src/host/`): app information, opening links, saving files, and control of a local microscope service where the host manages one. `main.ts` provides the browser implementation, and components use `useHost()`. The Tauri app will provide its own implementation, and nothing else changes.
4. **Every asset is bundled:** no fonts, icons or scripts from a CDN, so the app works offline in Tauri.
5. **Streams and server-sent events are read with `fetch`,** not with `<img src>` for MJPEG or `EventSource` for SSE, so that headers such as authentication can be added later (ADR-0014, P10).
6. **The app's code uses no Node APIs.** Node runs only the config files and the end-to-end tests.

**Enforcement.** ESLint (`app/tauri-ready` in `web/eslint.config.js`) rejects the following outside `src/host/`:

- `createWebHistory`, against rule 1;
- `EventSource`, against rule 5;
- `window.open`, against rule 3.

Review enforces the other rules. The pull-request template's checklist covers it.

## Consequences

- The Tauri milestone adds a host implementation and a connection profile, without searching the app for browser assumptions.
- URLs look like `http://127.0.0.1:5000/#/control` rather than `/control`.
- Downloads, external links and service control are behind one small interface, which tests can replace.
- Reading MJPEG and SSE with `fetch` takes more code than `<img>` and `EventSource`. P10 and P18 write that once.

## Alternatives considered

- **HTML5 history routing** (`/control`). The URLs are nicer, but every server has to answer unknown paths with `index.html`, which conflicts with the fallback router that serves `teta-wot` (ADR-0006), and Tauri needs extra setup.
- **Calling Tauri APIs directly behind `if (window.__TAURI__)` checks.** The checks would spread through the app, and nothing would keep them complete.
- **Leaving portability to the Tauri milestone.** By then the assumptions would be everywhere.
