# ADR-0024: Server-described UI elements

- Status: Accepted
- Date: 2026-10-11
- Phase: P24

## Context

- **What varies with the hardware:** some settings panels depend on what the microscope is made of, such as:
  - a camera's settings, which differ between the simulator and each real camera;
  - a scan workflow's options;
  - a background detector's parameters;
  - the calibration actions.
- **Who knows them:** the web app can't know every driver's settings, and milestone-2 drivers should bring their own. The Things know.
- **The idea:** OpenFlexure has Things describe such panels as a list of UI elements, which the web app draws generically. It's GPL-3.0 (ADR-0002), so the model here is our own; only the idea is shared.
- **The constraints:**
  - the panels must be safe to draw: text from a Thing must never become markup;
  - they must stay in step with the Things they control;
  - the web app's types come from the server's OpenAPI document (ADR-0021).

## Decision

- **Delivery:** a Thing with a panel serves it from a custom endpoint, `GET /{thing}/settings_ui` (`#[endpoint]`). The body is a JSON list of elements.
  - **Not a property:** the model is recursive, which a Thing Description's data schemas can't express.
  - **No fetching yet:** the first Thing to serve a panel (P25 or later) adds the web app's fetch. P24 provides the model and the renderers.
- **The model** (`microscope_things::ui`): an element is an object whose `kind` says what it is.

  | `kind` | What | Fields |
  |---|---|---|
  | `header` | A heading | `text` |
  | `text` | A paragraph | `text` |
  | `bullets` | A bulleted list | `items` |
  | `property_control` | An input for a property | `thing`, `property`, `label`, `options`, `step`, `read_back_ms`, `input`, `broken` |
  | `accordion` | A collapsible section | `title`, `children` |
  | `container` | Elements laid out together | `layout` (`column` or `row`), `children` |

  P24b adds `action_button` and `download_button`. Our own choices:
  - **options** are an ordered list of `{label, value}`, not a map;
  - **read-back** is one optional delay, `read_back_ms`;
  - **the duration input** is `input: "duration_hms"`, a number of seconds shown as hours, minutes and seconds;
  - **containers** have a layout from a short list, not CSS classes, so a Thing can't style the app.
- **Rich text, not HTML:** text is a list of spans, each with its `text` and, optionally, `bold`, `italic` or `href`.
  - **Drawn as elements:** the web app draws spans as elements, never through `v-html`. So there's no HTML to sanitise or parse, on either side.
  - **Links:** the allow-list is for links. An `href` is kept only if it's an `http` or `https` URL with a host.
  - **Checked twice:** the Rust builder checks links and removes control characters. The web app checks the scheme again before it makes a link. Links open outside the app, through the host (ADR-0010).
- **Names are checked when a description is built:** `ThingUi` builds controls for one Thing and checks each property's name against the Thing.
  - **A missing property** gives a control marked `broken`, and a warning in the log.
  - **The web app** also draws a control as broken if the Thing Description lacks its property.
- **How a property control behaves:**
  - **The input follows the property's schema:**
    - a list of options;
    - a switch for a boolean;
    - a number field with the schema's limits and step;
    - hours, minutes and seconds;
    - a text field;
    - a read-only value for anything else.
  - **Committing:** a committed edit is written at once, and the value is read back after `read_back_ms`, if given.
  - **Following:** an observable property's control follows its value, so two controls for it, or a change from elsewhere, stay in step.
    - **One stream per Thing:** every control of one Thing shares the Thing's single stream of property changes (`observeallproperties`).
    - **Why:** a browser opens only six connections to a server at a time over HTTP/1.1. A stream for each control blocked the page's other requests once a panel had a few.
  - **The highlight:** the label is highlighted while the value differs from the schema's default.
- **Types:**
  - **The web's types are written by hand** (`web/src/server-ui/types.ts`). Endpoint bodies aren't in the OpenAPI document, so they can't be generated (ADR-0021).
  - **Shared fixtures keep the two sides together:** the JSON files in `web/tests/fixtures/server-ui/` are both the Rust model's serialisation snapshots and the renderers' test inputs.
- **Forward compatibility:** the web app draws an element of an unknown kind as a short note, and draws the rest of the panel.

## Consequences

- **Drivers bring their own panels:** a milestone-2 camera describes its settings in Rust, and the web app needs no change.
- **Safe by construction:** a Thing can't inject markup or style, and links are limited to the web.
- **Two definitions of each type:** the Rust model and its TypeScript mirror. A change to one that the other misses fails the fixture tests on one side or the other, as long as the fixtures cover it.
- **Less expressive text:** there are no line breaks within a paragraph, and no tables or images in text. Separate elements cover what's needed.
- **Unknown elements degrade:** an older app draws a newer server's new elements as a note, rather than failing.

## Alternatives considered

- **A subset of HTML, sanitised (OpenFlexure's way).** It needs a sanitiser that's right in every case, and `v-html` in the app. Spans need neither.
- **A Markdown subset in strings.** It's nicer to write, but needs a parser in the web app, and escaping rules for literal asterisks. Spans are built by a few Rust helpers instead.
- **Hand-written panels for each Thing, in the web app.** That's the simplest for the simulator, but every new driver would need a web release.
- **UI hints on the property affordances themselves,** such as labels and steps in the Thing Description. They can't express layout, sections or text between controls.
- **A generic JSON-schema form library.** It's heavy, it doesn't cover actions with progress dialogs (P24b), and it would bring its own styles.
