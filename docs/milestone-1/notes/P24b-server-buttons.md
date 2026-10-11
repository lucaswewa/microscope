# P24b: Server-described action and download buttons

- Status: In review
- Pull request: [#34](https://github.com/lucaswewa/microscope/pull/34)
- ADRs: none new; it extends [ADR-0024](../../adr/0024-server-described-ui-elements.md)'s model
- Spec: [phases.md#p24b](../phases.md#p24b), split from P24, and re-sized from M to L at the owner's choice (plan Appendix D)

## Summary

A Thing's panel can now hold buttons that run its actions and download its files.

- **`ActionButton`:**
  - **Before:** it asks a question first, if the panel gives one.
  - **While it runs:** it shows its progress beside the button, or in a dialog with the log and a camera's small live stream. It can be cancelled, unless the panel says not.
  - **After:** it reports success with a message or the action's output, and asks for the panel to be refreshed if the panel says to. A failure is reported.
- **`DownloadButton`:** it saves the file a Thing's endpoint answers, or the file an action returns, under the panel's name for it.
- **Name checks:** both are checked when built, as property controls are. A button for an action or endpoint the Thing lacks says it isn't available.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/ui.rs` | `ActionButton`, `Progress`, `Success`, `DownloadButton`, `DownloadSource`, and `ThingUi::action`, `download_endpoint` and `download_action` |
| `crates/microscope-things/tests/ui.rs` | The buttons join the fixture's panel; a small Thing with a `GET` and a `POST` endpoint tests the endpoint check |
| `web/src/server-ui/ActionButton.vue`, `DownloadButton.vue` | The renderers |
| `web/src/server-ui/ServerInterface.vue`, `types.ts` | The two new kinds, and the `refresh` event, passed up through containers and accordions |
| `web/src/api/wot/client.ts` | `WotClient.download(url)`: a file, as a Blob, with the client's headers |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5000
npm --prefix web run dev
```

Open `http://localhost:5173/#/dev/components` and scroll to "Server-described UI":

- **Settle** asks first, then shows a dialog with the camera's small stream until the camera settles, and says "Settled." To watch the dialog longer, raise the camera's `settling_time`.
- **Capture** runs beside its button, can't be cancelled, and reports what the action returned: the capture's number.
- **Download a frame** saves `frame.jpg`.
- **Missing and Download settings** say they aren't available: the camera has no such action or endpoint.

## Design notes and deviations from the plan

- **Our own model, not OpenFlexure's:**
  - **The success message** is one field, `success`: `{"message": …}` or `"output"`. OpenFlexure has three flags.
  - **The dialog's stream** names its camera, rather than assuming there's one.
  - **A download's source** is a tagged choice, `source: "endpoint"` or `"action"`.
- **Endpoint downloads** are `GET /{thing}/{path}` under the API prefix: the Thing's own endpoints. OpenFlexure's are relative to a data folder.
- **Only GET endpoints:** `ThingUi::download_endpoint` accepts only an endpoint that answers `GET`; a `POST` endpoint at that path gives a broken button.
- **Files go through the client and the host:** a download is fetched with the client's headers (`WotClient.download`) and saved through the host (ADR-0010). That works the same in a browser and, later, in the Tauri app.
- **Cancelling:** a cancelled action closes its dialog without an error.
- **Refresh:** the panel's owner gets the `refresh` event. Nothing fetches panels yet (ADR-0024), so the first Thing to serve one wires it up.
- **Dialogs follow the app's theme:** in the components gallery, a dialog opened from the dark panel is in the app's theme, as every overlay is. Both themes were checked by switching the app's theme.
- **Size:** 782 changed lines of hand-written code (about 220 of them tests) against L's budget of 900. It was estimated at 750–880 and re-sized from M at the owner's choice.

## Tests

- **Rust:**
  - **`tests/ui.rs`** (2 tests):
    - the fixture now holds two action buttons with every option, and two downloads, one of each source;
    - the panel also has a button for an action the camera lacks, and a download from an endpoint it lacks, both broken;
    - an endpoint download needs an endpoint that answers `GET`.
- **`ServerInterface.spec.ts`** (6 new tests), against a fake camera:
  - **an action** runs with its input, offers no cancelling when it can't be, and reports its output;
  - **asking and the dialog:** declining the question runs nothing; the dialog shows the log and the stream, and cancelling sends `DELETE`, closes the dialog and reports nothing;
  - **success:** it's reported, and `refresh` passes up through a container;
  - **inline progress:** the button is disabled while running, with Cancel beside it, and a failure is reported;
  - **downloads:** an action's file and an endpoint's are saved under their names, and a failed download is reported;
  - **unavailable buttons:** each says what's missing.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in Rust:**
  - the action check;
  - the endpoint's method;
  - the endpoint check;
  - leaving out the default `cancellable`;
  - leaving out an empty input;
  - a download's input;
- **in the web:**
  - the confirmation;
  - the log;
  - the stream;
  - cancelling;
  - quiet cancelling;
  - the success message;
  - refresh, and its passing through containers;
  - not offering cancel;
  - disabling while running;
  - the endpoint's URL;
  - reporting a failed download;
  - an action the Thing Description lacks;
  - a broken download.

## Follow-ups and known gaps

- **The first panel** (P25 or later) adds `GET /{thing}/settings_ui`, the web app's fetching of it, and a refetch on `refresh`.
- **Progress values:** the dialog's bar runs without a value, since actions don't report a fraction done. A later phase could add one, if scanning needs it.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its idea:** buttons for actions and downloads in a Thing's panel, read from its source for behaviour only.
  - **Our own:** the model, its names, and the renderers.
