# P11: Connection management

- Status: In review
- Pull request: [#15](https://github.com/lucaswewa/microscope/pull/15)
- ADRs: [ADR-0015](../../adr/0015-connection-model.md)
- Spec: [phases.md#p11](../phases.md#p11)

## Summary

The app now connects to a microscope:

- by default, the server that served the page;
- otherwise, another microscope by its address, from a Connect screen or `?microscope=` in the page's address.

A connection store holds the WoT client, the Things' descriptions and the host name, so the rest of the app doesn't need to know where the server is. It checks that the server is still there, covers the page with an offline notice when it isn't, and reconnects when it's back. Tabs show only when the server has the Things they need. An end-to-end test connects across origins, stops the server, and watches the app recover.

## What was built

| Where | What |
|---|---|
| `web/src/connection/profiles.ts` | Profiles (this server, or a remote origin), their API roots, reading typed addresses, the `?microscope=` parameter, and recent connections in `localStorage` |
| `web/src/connection/store.ts` | `useConnectionStore`: the state machine (idle, connecting, connected, lost, reconnecting), the heartbeat, retries with backoff, Retry, the required Things, Thing availability, recent connections and the window title |
| `web/src/connection/ConnectView.vue` | The Connect screen at `/#/connect`: this server, another microscope by address with validation, and the recent ones (each can be forgotten). A successful connection goes to View |
| `web/src/connection/ConnectionIndicator.vue` | The foot of the rail: a dot (green, red, grey, or a spinner) and the microscope's name, linking to the Connect screen, with a tooltip saying where |
| `web/src/connection/OfflineOverlay.vue` | Covers the page while the microscope can't be reached: why, whether it's trying, Retry, and Choose a microscope. The Connect screen stays uncovered |
| `web/src/main.ts` | Provides Thing availability from the store, and connects once the app is mounted |
| `web/src/app/AppShell.vue`, `NavRail.vue`, `web/src/router/index.ts` | The overlay over the page, the indicator in the rail, and the `/connect` route |
| `web/playwright.config.ts` | Builds and starts a microscope server on port 5098. The preview forwards `/api` to it |
| `web/tests/e2e/connection.spec.ts` | The app on one origin, a server on another (port 5099) that stops and starts again |
| `web/tests/e2e/smoke.spec.ts`, `shell.spec.ts`, `web/tests/visual/` | Updated for a connected app: the gated tabs, the title, and new baselines with the host name masked |

## How to try it

Start a server on another port, then the development server:

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5001
cd web
npm run dev
```

1. Open <http://localhost:5173/>. It connects to the server on port 5000 through Vite's forwarding, if one is running. The rail shows the host name, and the window title is "<host> – Microscope".
2. Open <http://localhost:5173/?microscope=localhost:5001#/view>. It connects across origins.
3. Stop the server on 5001. Within about 5 s, "Connection lost" covers the page. Start it again: the app reconnects by itself, or at once with Retry.
4. Click the indicator for the Connect screen. Try an address such as `ftp://x` for the error, connect to `localhost:5001`, and find it under Recent.

## Design notes and deviations from the plan

ADR-0015 records the connection model. The details:

- **Connected means usable:** the descriptions are listed, the required `system` Thing is there, and its `hostname` reads. A server without `system` counts as lost, with the reason shown.
- **The timings:**
  - the heartbeat reads `/api/v1/health` every 5 s;
  - retries wait 1 s, then 2, 4, 8 and 10;
  - Retry tries at once and restarts the waits.

  In Chromium, a stopped server was noticed in 5.3 s, and its return in 0.8 s.
- **Each `connect()` is a new attempt.** Answers to an older attempt that arrive late are ignored, so choosing a second microscope while the first is still answering can't end up connected to the first.
- **The page's address** gains `?microscope=lab-pc:5000` for a remote connection and loses it for this server, through `history.replaceState`, keeping the route.
- **The two error headings:** the overlay says "Connection lost" when the app had been connected, and "Can't reach the microscope" when it never was. It shows the error through ErrorDetails.
- **Tabs and Things:** today's server has only `system`, so connecting hides Slide Scan, Sequence and Gallery. That's as planned: they return with their Things (P31, P35, P41). Before any descriptions are known, every tab shows, so nothing flickers away while connecting.
- **Unit tests don't connect by themselves.** The app connects from `main.ts`, after mounting, and availability is provided there too. The tests mount `App` without either, so they stay offline unless a test connects the store against a fake server.
- **End-to-end tests now need a server.** Playwright builds and starts one from the checkout (`cargo run`, so the first run takes a minute or so). The reconnect test starts and stops its own, on port 5099. The manual-check ports in CLAUDE.md are now 5090–5097.
- **Visual baselines** now show the connected rail. The indicator's label is masked, since it names the machine running the tests. The old baselines still passed, because the change was under the 1% pixel tolerance, so I replaced them on purpose (`--update-snapshots=all`).
- **Size:** 1,111 changed lines of hand-written code (609 in `src`, 476 in tests, 26 in configuration) against M's budget of 600, above my estimate of 950. The project owner chose to keep it as one pull request.

## Tests

256 unit tests, and 15 end-to-end and visual tests, all passing locally. New in this phase:

- **Profiles** (7):
  - reading typed addresses, and saying what's wrong with bad ones;
  - `?microscope=` and its fallback;
  - the page's address following the profile;
  - five recent connections without repeats, read back safely, and forgotten when storage fails.
- **The store** (10):
  - connecting (descriptions, host name, title);
  - lost and retried with growing waits;
  - a lost connection noticed by the heartbeat, its Things kept, and recovered;
  - Retry;
  - a missing required Thing;
  - an abandoned attempt's late answer ignored;
  - remote microscopes remembered and put in the address;
  - starting from the address;
  - availability before and after connecting;
  - disconnecting.
- **The interface** (5): the indicator's states, name and link; the overlay and Retry; the overlay leaving the Connect screen uncovered; address validation; connecting from the Connect screen, then the recent list and forgetting.
- **End to end:**
  - connecting across origins (title, gated tabs), losing the server, and recovering after its restart;
  - the smoke test, now connected.

Mutation checks: each of these, removed, fails a test:

- the guard against stale attempts (which first needed a better test: the abandoned attempt must succeed late, not fail);
- the heartbeat;
- the growing waits;
- the overlay's exception for the Connect screen.

## Follow-ups and known gaps

- P12 serves the app from the backend, which makes "this server" the usual profile, and adds the end-to-end tests to CI.
- P17 adds `camera` to the required Things. P44 shows a managed server's status.
- The Tauri app adds a third profile for the server it manages.
- A server behind a reverse proxy with a different API prefix isn't supported.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. OpenFlexure Connect, its discovery app, isn't mirrored: this app connects by address instead.
