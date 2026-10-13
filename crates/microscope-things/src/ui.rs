//! Server-described UI (ADR-0024): panels that a Thing describes and the web
//! app draws, for the parts of the UI that depend on the hardware. A panel
//! is a list of [`UiElement`]s, served as JSON from the Thing's
//! `settings_ui` endpoint. [`ThingUi`] builds a Thing's controls, checking
//! the names it's given.

use std::sync::{Arc, OnceLock, Weak};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teta_wot::ThingHandle;
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

/// One element of a panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiElement {
    /// A heading.
    Header {
        /// What it says.
        text: RichText,
    },
    /// A paragraph.
    Text {
        /// What it says.
        text: RichText,
    },
    /// A bulleted list.
    Bullets {
        /// The items.
        items: Vec<RichText>,
    },
    /// An input for a property.
    PropertyControl(PropertyControl),
    /// A collapsible section, closed at first.
    Accordion {
        /// Its title, which opens and closes it.
        title: String,
        /// What it holds.
        children: Vec<UiElement>,
    },
    /// Elements laid out together.
    Container {
        /// How.
        layout: Layout,
        /// The elements.
        children: Vec<UiElement>,
    },
    /// A button that runs an action.
    ActionButton(ActionButton),
    /// A button that downloads a file.
    DownloadButton(DownloadButton),
}

/// How a container lays out its elements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// One below another.
    #[default]
    Column,
    /// Side by side, wrapping when there's no room.
    Row,
}

/// Text with a little emphasis, and links: a list of spans. `&str` converts
/// to plain text, and the methods add to it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RichText(Vec<Span>);

/// A run of text, and how it's shown.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Span {
    /// The text, without control characters.
    pub text: String,
    /// Shown bold.
    #[serde(skip_serializing_if = "is_default")]
    pub bold: bool,
    /// Shown in italics.
    #[serde(skip_serializing_if = "is_default")]
    pub italic: bool,
    /// A link, opened outside the app: only an `http` or `https` URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

impl RichText {
    /// Adds plain text.
    pub fn and(self, text: &str) -> Self {
        self.span(text, Span::default())
    }

    /// Adds bold text.
    pub fn bold(self, text: &str) -> Self {
        self.span(
            text,
            Span {
                bold: true,
                ..Span::default()
            },
        )
    }

    /// Adds italic text.
    pub fn italic(self, text: &str) -> Self {
        self.span(
            text,
            Span {
                italic: true,
                ..Span::default()
            },
        )
    }

    /// Adds a link to `href`, opened outside the app. Unless `href` is an
    /// `http` or `https` URL with a host, the text is added as plain text.
    pub fn link(self, text: &str, href: &str) -> Self {
        let href = safe_href(href);
        self.span(
            text,
            Span {
                href,
                ..Span::default()
            },
        )
    }

    fn span(mut self, text: &str, span: Span) -> Self {
        let text = text.chars().filter(|c| !c.is_control()).collect();
        self.0.push(Span { text, ..span });
        self
    }
}

impl From<&str> for RichText {
    fn from(text: &str) -> Self {
        RichText::default().and(text)
    }
}

/// `href`, if it's an `http` or `https` URL with a host.
fn safe_href(href: &str) -> Option<String> {
    let (scheme, rest) = href.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let web = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    let clean = !href.chars().any(|c| c.is_control() || c.is_whitespace());
    (web && clean && !host.is_empty()).then(|| href.to_owned())
}

/// An input for a property, drawn to suit its schema.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropertyControl {
    /// The Thing's name.
    pub thing: String,
    /// The property's name.
    pub property: String,
    /// What the input is labelled.
    pub label: String,
    /// Values to choose from, in this order, in place of free entry.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Choice>,
    /// The step of a number's input, in place of its schema's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// Read the property again this many ms after writing it, for hardware
    /// that adjusts what it's given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_back_ms: Option<u32>,
    /// How the value is entered.
    #[serde(skip_serializing_if = "is_default")]
    pub input: InputStyle,
    /// Whether the Thing lacks the property.
    #[serde(skip_serializing_if = "is_default")]
    pub broken: bool,
}

/// A value to choose, and its label.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Choice {
    /// What it's called.
    pub label: String,
    /// The value.
    pub value: Value,
}

/// How a property's value is entered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputStyle {
    /// As its schema suggests.
    #[default]
    Default,
    /// A number of seconds, as hours, minutes and seconds.
    DurationHms,
}

impl PropertyControl {
    /// Offers these values, with their labels, in place of free entry.
    pub fn options<'a, V: Into<Value>>(
        mut self,
        options: impl IntoIterator<Item = (&'a str, V)>,
    ) -> Self {
        self.options = options
            .into_iter()
            .map(|(label, value)| Choice {
                label: label.to_owned(),
                value: value.into(),
            })
            .collect();
        self
    }

    /// Steps a number's input by `step`.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Reads the property again `after_ms` after writing it.
    pub fn read_back(mut self, after_ms: u32) -> Self {
        self.read_back_ms = Some(after_ms);
        self
    }

    /// Enters a number of seconds as hours, minutes and seconds.
    pub fn duration_hms(mut self) -> Self {
        self.input = InputStyle::DurationHms;
        self
    }
}

impl From<PropertyControl> for UiElement {
    fn from(control: PropertyControl) -> Self {
        UiElement::PropertyControl(control)
    }
}

/// A button that runs an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActionButton {
    /// The Thing's name.
    pub thing: String,
    /// The action's name.
    pub action: String,
    /// What the button says.
    pub label: String,
    /// The action's input.
    #[serde(default = "empty_input", skip_serializing_if = "is_empty_input")]
    pub input: Value,
    /// A question to answer before it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm: Option<String>,
    /// Where its progress shows.
    #[serde(default, skip_serializing_if = "is_default")]
    pub progress: Progress,
    /// The camera whose small live stream the progress dialog shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Whether it can be cancelled while it runs.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub cancellable: bool,
    /// What to say when it succeeds: nothing, unless set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<Success>,
    /// Whether the panel should be described again once it succeeds,
    /// because the action changes what it shows.
    #[serde(default, skip_serializing_if = "is_default")]
    pub refresh: bool,
    /// Shown as a secondary button, rather than a primary one.
    #[serde(default, skip_serializing_if = "is_default")]
    pub secondary: bool,
    /// Whether the Thing lacks the action.
    #[serde(default, skip_serializing_if = "is_default")]
    pub broken: bool,
}

/// Where an action's progress shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Progress {
    /// Beside the button.
    #[default]
    Inline,
    /// In a dialog, for a long action to watch.
    Dialog,
}

/// What a button says when its action succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Success {
    /// This message.
    Message(String),
    /// The action's output.
    Output,
}

impl ActionButton {
    /// Runs the action with `input`.
    pub fn input(mut self, input: Value) -> Self {
        self.input = input;
        self
    }

    /// Asks `question` before running it.
    pub fn confirm(mut self, question: &str) -> Self {
        self.confirm = Some(question.to_owned());
        self
    }

    /// Shows its progress in a dialog, with the small live stream of the
    /// camera `stream`, if given.
    pub fn in_dialog(mut self, stream: Option<&str>) -> Self {
        self.progress = Progress::Dialog;
        self.stream = stream.map(str::to_owned);
        self
    }

    /// Doesn't offer to cancel it.
    pub fn not_cancellable(mut self) -> Self {
        self.cancellable = false;
        self
    }

    /// Says `success` when it succeeds.
    pub fn on_success(mut self, success: Success) -> Self {
        self.success = Some(success);
        self
    }

    /// Asks for the panel to be described again when it succeeds.
    pub fn refresh(mut self) -> Self {
        self.refresh = true;
        self
    }

    /// Shows it as a secondary button.
    pub fn secondary(mut self) -> Self {
        self.secondary = true;
        self
    }
}

/// A button that downloads a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadButton {
    /// What the button says.
    pub label: String,
    /// Where the file comes from.
    #[serde(flatten)]
    pub source: DownloadSource,
    /// The name it's saved under.
    pub filename: String,
    /// Shown as a secondary button, rather than a primary one.
    #[serde(default, skip_serializing_if = "is_default")]
    pub secondary: bool,
    /// Whether the Thing lacks the endpoint or the action.
    #[serde(default, skip_serializing_if = "is_default")]
    pub broken: bool,
}

/// Where a download comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum DownloadSource {
    /// A Thing's endpoint: `GET /{thing}/{path}`.
    Endpoint {
        /// The Thing's name.
        thing: String,
        /// The endpoint's path.
        path: String,
    },
    /// The file an action returns.
    Action {
        /// The Thing's name.
        thing: String,
        /// The action's name.
        action: String,
        /// The action's input.
        #[serde(default = "empty_input", skip_serializing_if = "is_empty_input")]
        input: Value,
    },
    /// A file in the data folder: `GET /data/{path}` (ADR-0026).
    Data {
        /// Its path in the data folder.
        path: String,
    },
}

impl DownloadButton {
    /// Runs the action with `input`, if the file comes from an action.
    pub fn input(mut self, value: Value) -> Self {
        if let DownloadSource::Action { input, .. } = &mut self.source {
            *input = value;
        }
        self
    }

    /// Shows it as a secondary button.
    pub fn secondary(mut self) -> Self {
        self.secondary = true;
        self
    }
}

impl From<ActionButton> for UiElement {
    fn from(button: ActionButton) -> Self {
        UiElement::ActionButton(button)
    }
}

impl From<DownloadButton> for UiElement {
    fn from(button: DownloadButton) -> Self {
        UiElement::DownloadButton(button)
    }
}

/// Builds the controls of one Thing's panel, checking the names it's given
/// against the Thing.
pub struct ThingUi<'a> {
    thing: &'a ThingHandle,
}

impl<'a> ThingUi<'a> {
    /// For `thing`.
    pub fn new(thing: &'a ThingHandle) -> Self {
        ThingUi { thing }
    }

    /// The Thing's name.
    pub fn thing_name(&self) -> &'a str {
        self.thing.name()
    }

    /// A control for the property `name`, labelled `label`. If the Thing has
    /// no such property, the control is broken, and a warning is logged.
    pub fn property(&self, name: &str, label: &str) -> PropertyControl {
        let broken = self.lacks(self.thing.property(name).is_none(), "property", name);
        PropertyControl {
            thing: self.thing.name().to_owned(),
            property: name.to_owned(),
            label: label.to_owned(),
            options: Vec::new(),
            step: None,
            read_back_ms: None,
            input: InputStyle::Default,
            broken,
        }
    }

    /// A button that runs the action `name`, labelled `label`. If the Thing
    /// has no such action, the button is broken, and a warning is logged.
    pub fn action(&self, name: &str, label: &str) -> ActionButton {
        ActionButton {
            thing: self.thing.name().to_owned(),
            action: name.to_owned(),
            label: label.to_owned(),
            input: Value::Object(Map::new()),
            confirm: None,
            progress: Progress::Inline,
            stream: None,
            cancellable: true,
            success: None,
            refresh: false,
            secondary: false,
            broken: self.lacks(self.thing.action(name).is_none(), "action", name),
        }
    }

    /// A button that downloads what the Thing's endpoint at `path` answers
    /// to `GET`, as `filename`. Without such an endpoint, it's broken.
    pub fn download_endpoint(&self, path: &str, label: &str, filename: &str) -> DownloadButton {
        let mut endpoints = self.thing.endpoints();
        let found = endpoints.any(|endpoint| endpoint.method() == "GET" && endpoint.path() == path);
        let source = DownloadSource::Endpoint {
            thing: self.thing.name().to_owned(),
            path: path.to_owned(),
        };
        self.download(
            source,
            label,
            filename,
            self.lacks(!found, "endpoint", path),
        )
    }

    /// A button that downloads the file the action `name` returns, as
    /// `filename`. Without such an action, it's broken.
    pub fn download_action(&self, name: &str, label: &str, filename: &str) -> DownloadButton {
        let source = DownloadSource::Action {
            thing: self.thing.name().to_owned(),
            action: name.to_owned(),
            input: Value::Object(Map::new()),
        };
        let broken = self.lacks(self.thing.action(name).is_none(), "action", name);
        self.download(source, label, filename, broken)
    }

    fn download(
        &self,
        source: DownloadSource,
        label: &str,
        filename: &str,
        broken: bool,
    ) -> DownloadButton {
        DownloadButton {
            label: label.to_owned(),
            source,
            filename: filename.to_owned(),
            secondary: false,
            broken,
        }
    }

    /// `missing`, after logging a warning if the Thing lacks the `kind`
    /// called `name`.
    fn lacks(&self, missing: bool, kind: &str, name: &str) -> bool {
        if missing {
            let thing = self.thing.name();
            tracing::warn!(thing, kind, name, "a panel names something the Thing lacks");
        }
        missing
    }
}

/// A Thing's own registry entry, for its panels to check names against.
/// It's kept weakly, so that it doesn't keep the Thing alive.
#[derive(Debug, Default)]
pub struct OwnEntry(OnceLock<Weak<ThingHandle>>);

impl OwnEntry {
    /// Keeps the entry of the Thing that `ctx` is for, a `T`, as it starts.
    pub fn keep<T: Thing>(&self, ctx: &ThingCtx) {
        if let Some(thing) = ctx.server().thing_ref::<T>(ctx.name()) {
            let _ = self.0.set(Arc::downgrade(ThingRef::handle(&thing)));
        }
    }

    /// The panel `build` makes for the Thing; empty before it starts.
    pub fn panel(&self, build: impl FnOnce(&ThingUi) -> Vec<UiElement>) -> Json<Vec<UiElement>> {
        let entry = self.0.get().and_then(Weak::upgrade);
        Json(entry.map_or_else(Vec::new, |entry| build(&ThingUi::new(&entry))))
    }
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_empty_input(input: &Value) -> bool {
    input.as_object().is_some_and(Map::is_empty)
}

fn yes() -> bool {
    true
}

fn empty_input() -> Value {
    Value::Object(Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_web_links_and_drops_control_characters() {
        let text = RichText::from("a\u{7}b")
            .link("web", "https://example.org/a?b#c")
            .link("plain", "HTTP://example.org")
            .link("script", "javascript:alert(1)")
            .link("files", "ftp://example.org")
            .link("hostless", "https:///path")
            .link("spaced", "https://example.org/a b");
        let hrefs: Vec<_> = text.0.iter().map(|span| span.href.as_deref()).collect();
        assert_eq!(text.0[0].text, "ab");
        assert_eq!(
            hrefs,
            [
                None,
                Some("https://example.org/a?b#c"),
                Some("HTTP://example.org"),
                None,
                None,
                None,
                None
            ]
        );
    }
}
