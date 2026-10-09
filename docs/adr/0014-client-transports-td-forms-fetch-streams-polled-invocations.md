# ADR-0014: Client transports: TD forms, fetch streams, polled invocations

- Status: Accepted
- Date: 2026-10-09
- Phase: P10

## Context

- The web app talks to `teta-wot` servers: the one that served the page, and remote ones by URL, across origins (`teta-wot` allows CORS from any origin). The Tauri app will do the same later.
- Every affordance is described in its Thing Description (TD), with forms for each operation. `teta-wot` offers three transports:
  - plain HTTP;
  - server-sent events (SSE) for observing properties and subscribing to events;
  - a WebSocket per Thing.
- Actions run as invocations, resources at `{prefix}/action_invocations/{id}` that can be read and cancelled.
- Milestone 1 has no authentication (D8), but a token may come later. The browser's `EventSource` can't send headers, and nor can its WebSocket.
- `teta-wot` has two wire profiles. This app's server uses the default, `tetathing`, whose invocation records and error bodies differ from the `wot` profile's.
- OpenFlexure's web app follows actions by polling them, which has proved simple and robust.

## Decision

- **URLs come from the TDs.** The client finds each operation's form by its `op` and resolves its `href` against the TD's `base`, or failing that against the client's API root. Only two URLs are built by hand, since no TD operation covers them: the entry point, `{prefix}/thing_descriptions/`, and the `/reset` that `teta-wot` puts after a property's URL.
- **HTTP only, through `fetch`.**
  - Requests and responses are JSON.
  - Server-sent events are read from a `fetch` response with our own `text/event-stream` parser, never with `EventSource`.
  - WebSocket forms are ignored.
- **One header hook covers every request,** streams included, so a bearer token can be added later in one place.
- **Invocations are followed by polling.** The first poll is 100 ms after the action starts, and the interval grows by half each time to one per second, until the invocation finishes. Cancelling is a `DELETE` of its URL.
- **Observations reconnect by themselves:** after 1 s, then 2, 4 and so on up to 30 s, when a stream drops, the server can't be reached, or it answers with a 5xx. Any other error response ends the observation.
- **Every failure is an `ApiError`,** of one of these kinds:
  - `invalid`: a 422, with its validation list;
  - `lock-busy`: the global lock is taken;
  - `http`: any other error response;
  - `failed`: the action ended in an error;
  - `network`: no answer;
  - `cancelled`: the request was aborted, or the action was cancelled.

  Each carries the server's body for ErrorDetails.
- **`AbortSignal` everywhere:** every request, every stream, and the following of an invocation can be stopped with a signal. Stopping the following leaves the action running.
- **Only the default `tetathing` wire profile is supported.**

## Consequences

- Adding authentication is one hook, not a rewrite.
- A changed URL layout on the server needs no client change, as long as the TDs describe it.
- **Each stream holds an HTTP/1.1 connection,** and browsers allow only six per origin. The MJPEG stream (P18), a few observations and the polls must fit within that. Pages should prefer one stream for all of a Thing's properties (`observeProperties`) to one per property. If that isn't enough, a WebSocket per Thing is the next step.
- Polling costs at most one request per second for each running action, which is nothing on a local network. Updates arrive up to a second late.
- The SSE parser is ours to maintain. It is tested against events split anywhere between chunks.
- Running against a server in the `wot` profile would need its invocation and error shapes.

## Alternatives considered

- **`EventSource`.** It's built in and reconnects by itself, but it can't send headers (D8), and it reconnects on its own schedule.
- **The WebSocket per Thing,** for all observations and events. One connection would avoid the six-connection limit, but browsers can't set headers on it either, and it's more protocol to maintain. It remains the way out if streams run short.
- **Following invocations through the WebSocket's `actionStatus` messages.** Push instead of polling, but it would bring in the WebSocket for this one use.
- **A general WoT library such as node-wot.** It's large and written mainly for Node. It doesn't know `teta-wot`'s invocation resources or error shapes. Its licence (EPL-2.0 or the W3C licence) is also outside the plan's list.
- **A client generated from the OpenAPI document.** The TDs are what the server describes at run time. Types generated from OpenAPI may still help the typed facades of later phases.
