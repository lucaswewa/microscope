# ADR-0015: Connection model

- Status: Accepted
- Date: 2026-10-09
- Phase: P11

## Context

- The web app is served in several ways, and must find its microscope from each:
  - by the microscope's own server (P12);
  - by Vite during development, which forwards `/api` to a server;
  - later, by the Tauri app.
- People also reach other microscopes on the local network by address. Those requests cross origins, which `teta-wot` allows (CORS from any origin).
- The rest of the app shouldn't need to know where the server is. It wants a WoT client (ADR-0014) and the Things' descriptions.
- Servers stop and start: they're restarted, the launcher restarts them (P44, P45), and computers sleep. The app must notice and recover, without losing the user's place.
- Pages depend on Things: a tab whose Things a server lacks shouldn't show (P07).

## Decision

- **Two connection profiles:**
  - **this server**, the origin that served the page;
  - **a remote microscope**, an origin people type as `lab-pc:5000` or `http://192.168.1.20:5000`. Without a scheme it's http, and any path is dropped.

  Both use the API root `/api/v1/` (ADR-0007). The Tauri app will add a third profile for the server it manages, behind the same store.
- **The page's address names the profile.** `?microscope=<address>`, before the hash, connects to a remote microscope; without it, the app connects to this server. Connecting updates the parameter, so reloading, bookmarking or sharing the page reconnects the same way. A parameter that isn't an address is ignored.
- **One Pinia store owns the connection:**
  - the client, the cached descriptions and the host name;
  - a state machine: idle → connecting → connected → lost → reconnecting.

  The app is **connected** only when the server lists the Things' descriptions, has the required Things (`system`, and `camera` from P17), and answers the host name. Connecting to a new profile starts a new attempt, and any older attempt's answers are ignored.
- **A heartbeat:** a connected app reads `/api/v1/health` every 5 s. A failure there, or in connecting, makes the connection **lost**. The app then tries again after 1 s, doubling each time up to 10 s, and the wait starts over once it's connected. Retry tries at once, and a reconnection reads the descriptions again, since a restarted server may have changed.
- **The descriptions are kept while the connection is lost,** so the tabs stay put. Tabs show only when their Things are in the descriptions. Before any are known, every tab shows.
- **Recent remote origins** are remembered in `localStorage`, five at most, with every read and write guarded.
- **The window's title** names the microscope: "lab-pc – Microscope".

## Consequences

- The rest of the app uses the store's client and descriptions, wherever the server is.
- **Tabs come and go with the server's Things.** Today's server has only `system`, so Slide Scan, Sequence and Gallery are hidden until their Things arrive.
- Losing a server is noticed within about 5 s, and recovery takes at most 10 s after it's back.
- The heartbeat costs one small request every 5 s.
- `/api/v1/health` is this project's own route, not a WoT affordance, so the heartbeat assumes a microscope server. It isn't for any `teta-wot` server.
- End-to-end tests need a running server. Playwright starts one, built from the checkout.

## Alternatives considered

- **Noticing a lost server only when a request fails.** An idle page would never notice. Pages showing live data need to know at once.
- **A stream as the heartbeat,** with the stream dropping meaning the server is lost. It would hold one of the browser's six connections (ADR-0014), and the `system` Thing has nothing to observe.
- **Remembering the last profile in `localStorage` and reconnecting to it.** A page served by one microscope could then quietly connect to another. The address parameter is explicit, and works for links and bookmarks.
- **Full URLs with any path** for remote servers. ADR-0007 fixes the prefix, so an origin is enough. A server behind a reverse proxy with a different prefix would need it, and can come later.
