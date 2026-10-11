# P24: Server-described UI elements

- Status: Done
- Pull request: [#33](https://github.com/lucaswewa/microscope/pull/33)
- ADRs: [ADR-0024](../../adr/0024-server-described-ui-elements.md)
- Spec: [phases.md#p24](../phases.md#p24). P24 was split, at the owner's choice: the action and download buttons are now P24b (plan Appendix D).

## Summary

Things can now describe the hardware-dependent parts of the UI, and the web app draws them generically. ADR-0024 records the design.

- **The Rust model** (`microscope_things::ui`):
  - **elements:** headers, paragraphs, bullets, property controls, accordions and containers;
  - **rich text** as spans, with only web links allowed;
  - **`ThingUi`,** which builds a Thing's controls and marks one broken if the Thing lacks its property.
- **The web renderers** (`web/src/server-ui/`):
  - **`ServerInterface`** draws a panel.
  - **`PropertyControl`** gives each property an input that suits its schema, and writes committed values. It reads back when asked, follows the live value, and highlights a value that differs from its default.
- **One fixture for both sides:** the Rust model's serialisation snapshot is the renderers' test input.

No Thing serves a panel yet; the first is P25 or later. Until then, the components gallery draws the fixture against the live camera.

## What was built

| Where | What |
|---|---|
| `crates/microscope-things/src/ui.rs` | `UiElement`, `RichText`, `PropertyControl` and its builder methods, `Layout`, `InputStyle`, and `ThingUi` |
| `crates/microscope-things/tests/ui.rs` | A panel for the simulated camera, compared with the fixture (`BLESS=1` rewrites it) |
| `web/tests/fixtures/server-ui/panel.json` | The shared fixture |
| `web/src/server-ui/types.ts` | The model's TypeScript mirror |
| `web/src/server-ui/ServerInterface.vue`, `RichText.vue` | The panel and its text |
| `web/src/server-ui/PropertyControl.vue`, `DurationField.vue`, `duration.ts` | Property inputs, and durations as hours, minutes and seconds |
| `web/src/server-ui/follow.ts` | `followProperty`: one shared stream of property changes per Thing |
| `web/src/views/dev/ComponentsView.vue` | The fixture's panel in the components gallery, in both themes |

## How to try it

```powershell
cargo run -p microscope-server -- -c configs/simulation.json --port 5000
npm --prefix web run dev
```

Open `http://localhost:5173/#/dev/components`, and scroll to "Server-described UI" in either theme. The panel's controls are bound to the simulated camera:

- **Following:** change the exposure in one theme's panel, and the other follows it.
- **The highlight:** while a value differs from its default, its label is highlighted, with "Changed from its default." below.
- **The accordion** ("More") holds a duration, entered as hours, minutes and seconds, and a control for a property the camera lacks, which says it isn't available.

The settings you change stay changed; put them back afterwards (exposure 10000, not repeating, objective 40×, settling time 0.2 s).

## Design notes and deviations from the plan

ADR-0024 records the design. The details:

- **Our own model, not OpenFlexure's:**
  - **Rich text:** text is a list of spans rather than an HTML subset, so nothing is parsed or drawn with `v-html`.
  - **Options and read-back:** options are an ordered list, and read-back is one optional delay.
  - **Layout:** containers choose a layout rather than CSS classes.
- **Following live values came from looking in a browser.**
  - **The first version** read each property once. Two controls for one property, as in the gallery's two themes, then disagreed.
  - **A stream per control** blocked the page instead. A browser opens only six connections to a server at a time over HTTP/1.1, and the gallery's two panels wanted ten.
  - **Now** every control of a Thing shares the Thing's one stream of property changes, which stops when its last control goes, as P18's shared MJPEG streams do.
- **The highlight uses the default in the Thing Description:** `teta-wot` puts each setting's default there. A property without one is never highlighted.
- **Choosing the input:** the input follows the value's type, with the schema's `enum` as a list unless the panel gives options. Anything that isn't a boolean, number or string is shown read-only.
- **Committing:**
  - **A text field** writes on Enter or when it loses focus, once.
  - **A number field** writes when its edit is committed (P09).
  - **A refused write** is undone in the field and reported.
- **Unknown elements:** an element of a kind the app doesn't know shows a note, and the rest of the panel still draws.
- **Links** open through the host (ADR-0010), and only `http` and `https` links are links, checked on both sides.
- **No fetching yet:** `GET /{thing}/settings_ui` comes with the first Thing that serves a panel.
- **Size:** 1,123 changed lines of hand-written code (about 330 of them tests) against L's budget of 900. My estimate after the split was 700–800. The extra came from:
  - following live values;
  - the shared-fixture tests;
  - field documentation, which the Rust model needs throughout.

  The owner chose to keep it as one pull request.

## Tests

- **Rust:**
  - **`tests/ui.rs`** (1 test): a panel for the simulated camera serialises to the shared fixture. That includes a control for a property the camera lacks, which is broken.
  - **`ui` unit test:** links keep only `http` and `https` URLs with a host, and control characters are removed.
- **`ServerInterface.spec.ts`** (9 tests), drawing the fixture against a fake camera:
  - **text and structure:** a heading, emphasis, a link opened through the host, bullets, a row container, and an accordion that opens to its children;
  - **writing:** committed values are written, and read back after `read_back_ms`;
  - **following:** two controls share one stream and follow it, and the stream stops when they go;
  - **lists:** the panel's options, and the schema's enum;
  - **text:** it's written once, when committed;
  - **durations:** hours, minutes and seconds, with exact sums;
  - **the default highlight;**
  - **a refused write,** undone and reported;
  - **odd cases:** read-only values, a property the Thing Description lacks, a non-web link, and an unknown kind of element.

**Mutation checks:** I broke each of the following, and a test failed every time:

- **in Rust:**
  - the link schemes;
  - control characters;
  - the name check;
  - leaving out false flags;
- **in the web:**
  - web links only;
  - emphasis;
  - read-back;
  - undoing a refused write;
  - the default highlight;
  - the schema's enum;
  - writing once;
  - following;
  - sharing the stream;
  - stopping it;
  - the duration sum;
  - the row layout;
  - unknown elements;
  - a property the Thing Description lacks.

## Follow-ups and known gaps

- **P24b:** action and download buttons, in the model and the web.
- **The first panel** (P25 or later) adds `GET /{thing}/settings_ui`, and the web app's fetching of it.
- **Validation messages:** a number outside the schema's limits marks the field invalid (P09), but a text that breaks the schema's `pattern` is only refused by the server, with a toast.

## Independent implementation

- [x] No code, styles, assets or text copied from OpenFlexure.
  - **Its idea:** a list of UI elements served by a Thing, and a highlight for changed settings, read from its source for behaviour only.
  - **Our own:** the model, its names, the rich text and the renderers.
