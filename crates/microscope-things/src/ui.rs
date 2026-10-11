//! Server-described UI (ADR-0024): panels that a Thing describes and the web
//! app draws, for the parts of the UI that depend on the hardware. A panel
//! is a list of [`UiElement`]s, served as JSON from the Thing's
//! `settings_ui` endpoint. [`ThingUi`] builds a Thing's controls, checking
//! the names it's given.

use serde::Serialize;
use serde_json::Value;
use teta_wot::ThingHandle;

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

    /// A control for the property `name`, labelled `label`. If the Thing has
    /// no such property, the control is broken, and a warning is logged.
    pub fn property(&self, name: &str, label: &str) -> PropertyControl {
        let broken = self.thing.property(name).is_none();
        if broken {
            let thing = self.thing.name();
            tracing::warn!(
                thing,
                property = name,
                "a panel names a property the Thing lacks"
            );
        }
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
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
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
