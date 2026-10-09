# P10: WoT client library

- Status: In review
- Pull request: [#14](https://github.com/lucaswewa/microscope/pull/14)
- ADRs: [ADR-0014](../../adr/0014-client-transports-td-forms-fetch-streams-polled-invocations.md)
- Spec: [phases.md#p10](../phases.md#p10)

## Summary

A small, typed client for `teta-wot` servers, in `web/src/api/wot/`. It lists the Things' descriptions, and through each description:

- reads, writes and resets properties;
- invokes actions and follows them to their output, by polling;
- observes properties and subscribes to events, over server-sent events read with `fetch`, reconnecting when a stream drops.

Every failure is a typed `ApiError`, every call takes an `AbortSignal`, and one hook adds headers to every request. A contract test runs the client against a real server in a new CI job.

## What was built

| Where | What |
|---|---|
| `web/src/api/wot/td.ts` | The TD 1.1 types this app uses, and `formUrl`, which finds an operation's HTTP form and resolves it against the TD's `base` |
| `web/src/api/wot/client.ts` | `WotClient`: `thingDescriptions()`, `consume(td)`, `request()` and `stream()`. `ConsumedThing`: `readProperty`, `writeProperty`, `resetProperty`, `invokeAction`, `ongoingInvocations`, `observeProperty`, `observeProperties` and `subscribeEvent` |
| `web/src/api/wot/invocation.ts` | `Invocation`: the latest `record`, `done`, `output()`, `cancel()`, polling with backoff, and the record and log types |
| `web/src/api/wot/sse.ts` | A `text/event-stream` parser, and a response body as text chunks |
| `web/src/api/wot/errors.ts` | `ApiError` and its kinds, and `describeError`, moved here from `src/ui/errors.ts` because error bodies belong with the API. ErrorDetails now imports it from here |
| `web/tests/unit/api/` | The unit tests, with a fake server (`fakeServer.ts`): a `fetch` routed by method and path, and scripted event streams |
| `web/tests/contract/system.spec.ts`, `web/vitest.contract.config.ts` | The contract test against a running server, `npm run test:contract` |
| `.github/workflows/ci.yml` | A **Contract** job: it builds and starts the server on port 5090, waits for `/api/v1/health`, runs the contract test, and prints the server's log if it fails |
| `README.md`, `CLAUDE.md` | How to run the contract test |

## How to try it

The client has no page of its own yet; P11's connection screen is its first user. From `web/`, with the server running (`cargo run -p microscope-server -- -c configs/simulation.json`, on port 5000):

```powershell
npm run test:contract
```

Or in the browser console on `npm run dev`:

```js
const { WotClient } = await import('/src/api/wot/client.ts')
const client = new WotClient({ baseUrl: '/api/v1/' })
const system = client.consume((await client.thingDescriptions()).system)
await system.readProperty('version_data')
```

## Design notes and deviations from the plan

ADR-0014 records the transport decisions. The details:

- **Using a Thing:** `client.consume(td)` returns a `ConsumedThing`, after the WoT Scripting API, whose methods take affordance names. The client keeps no state; P11's connection store will hold the descriptions.
- **Resolving form URLs:** `teta-wot` writes form `href`s as paths (`/api/v1/system/hostname`) and `base` as the origin the request came to. So a page served through Vite's proxy, or a remote server reached by its address, resolves to the right origin. A description without `base` resolves against the client's API root.
- **Writing:** a property write sends any JSON value, `false`, `0`, `null` and `""` included. (Testing for a body's presence, not its truthiness, is what makes that work.) A reset is a `POST` to the property's URL with `/reset` after it.
- **Invoking:** an action called without input is sent with no body, which `teta-wot` accepts for actions without parameters. The answer is the invocation's record, which the `Invocation` then follows:
  - **Polling:** the first poll is 100 ms after the action starts, then each is half as long again after the last, up to one per second. `onUpdate` gets each new record.
  - **`output()`** rejects with `cancelled` or `failed`, or with `lock-busy` when the action couldn't take the global lock (409, `GlobalLockBusyError`).
  - **A signal stops the following, not the action.** `cancel()` asks the server to cancel it.
- **Ongoing invocations:** `ongoingInvocations(action)` reads the action's invocations and follows those still pending or running. This is how a page can pick up an action after a reload.
- **Observing:**
  - **One stream for all properties:** `observeProperties` uses the Thing's `observeallproperties` stream, whose events are named after the properties, so one connection covers them all.
  - **Reconnecting:** a dropped stream, a network failure or a 5xx is retried after 1 s, doubling to 30 s, and the wait resets once a stream connects. A 4xx, such as a property that isn't observable, ends the observation. `onError` is told each time.
- **`ApiError`** adds one kind to the spec's list. `failed` covers an action that ran and ended in an error, which isn't an HTTP failure. Its message comes from the server's body, through `describeError`: the `detail`, "N invalid inputs" for a 422, or the status text.
- **The SSE parser follows the HTML standard:**
  - it accepts CRLF, CR and LF line endings, including a CRLF split between chunks, and a CR that ends the stream;
  - it handles multi-line `data`, `event` and `id`, comments, and a byte-order mark;
  - it drops an event the stream ends in the middle of.

  Testing found two bugs, both fixed: a CR ending the stream lost the last event, and `formUrl` could pick a WebSocket form.
- **The contract test** uses the `system` Thing, the only one so far. It checks the listing and the base, property reads, and the server's 405 and 404 as ApiErrors. It runs in Vitest's Node environment against `MICROSCOPE_API_URL`. Locally it passed against a server on port 5091. In CI, the server is started in the same step as the tests. On Windows, the runner stops the processes a step started when that step ends, and the first CI run failed because the server started in a step of its own was gone by the next.
- **Size:** 1,325 changed lines of hand-written code (688 in `src`, 583 in tests, 54 in configuration and CI) against L's budget of 900, above my estimate of about 1,100. About 124 of them are `describeError` moving from `src/ui/` to `src/api/wot/`, counted once deleted and once added.

## Tests

234 unit tests and 3 contract tests, and the 14 end-to-end and visual tests, all passing locally. New in this phase:

- **The SSE parser** (6):
  - names, ids and multi-line data;
  - events split at every possible position;
  - CRLF, CR and LF, a CRLF split between chunks, and a CR at the end;
  - comments, events without data and a byte-order mark;
  - an unfinished event dropped;
  - UTF-8 split between chunks.
- **The client** (12):
  - finding forms (HTTP only, SSE when asked) and resolving them;
  - listing descriptions;
  - resolving without a base;
  - reading, writing every JSON value, and resetting;
  - refusing an affordance the description lacks;
  - the header hook;
  - `ApiError` for a 422, a lock taken, other error responses, no answer, and an aborted request.
- **Invocations** (8):
  - posting the input and following to the output, with the polls slowing on schedule;
  - no body without input;
  - `output()` rejecting when the action failed, found the lock taken, or was cancelled;
  - cancelling with `DELETE`;
  - a signal stopping the polls without cancelling the action;
  - following ongoing invocations.
- **Observation** (4):
  - values split between chunks, and stopping;
  - every property by name, and an event's data;
  - reconnecting after a dropped stream, a 503 and no answer, waiting 1, 2 and 4 s;
  - giving up on a 403.
- **Contract** (3, against a running server): the listing, property reads, and the server's errors.

Mutation checks: each of these, removed, fails a test:

- the end-of-stream CR;
- HTTP-only forms;
- sending falsy bodies;
- giving up on 4xx;
- the polling backoff.

## Follow-ups and known gaps

- P11 (connection management) creates the client for the chosen server, holds the descriptions, and shows `ApiError`s in toasts with their details.
- **Streams and connections:** each stream holds one of the six HTTP/1.1 connections a browser allows per origin (ADR-0014). The live image (P18) and the pages that observe properties must budget for that.
- The `wot` wire profile isn't supported.
- The contract test covers only the `system` Thing. It grows as Things with writable properties, actions and events arrive (P13 onwards).

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure. The client follows `teta-wot`'s documented HTTP binding and its source (MIT). The SSE parser follows the HTML standard.
