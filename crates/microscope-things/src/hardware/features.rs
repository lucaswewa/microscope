//! An SDK camera's settings: the GenICam-style features its SDK reports,
//! and the settings panel they make (ADR-0043, ADR-0024).
//!
//! A Thing's properties are fixed, so every SDK camera's Thing has the same
//! ones for its settings ([`STANDARD_FEATURES`]). The features say which of them a
//! camera has, and how: a number, or a choice from the camera's own list.
//! Each is read back after it's written, since a camera rounds or clamps
//! what it's given.

use crate::ui::{Layout, ThingUi, UiElement};

/// The settings an SDK camera's Thing offers: each feature's GenICam name,
/// the Thing's property for it, and its label.
pub const STANDARD_FEATURES: [(&str, &str, &str); 5] = [
    ("ExposureTime", "exposure_time", "Exposure (µs)"),
    ("Gain", "gain", "Gain (dB)"),
    ("PixelFormat", "pixel_format", "Pixel format"),
    ("BalanceWhiteAuto", "white_balance", "White balance"),
    ("BinningHorizontal", "binning", "Binning"),
];

/// How long a camera may take to settle a value it's given, before it's
/// read back.
const READ_BACK_MS: u32 = 300;

/// One of a camera's features, as its SDK reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Its GenICam name, such as `ExposureTime`.
    pub name: String,
    /// What it takes.
    pub kind: FeatureKind,
}

/// What a feature takes.
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureKind {
    /// A number, in steps of `step` where the camera has one.
    Number {
        /// The step.
        step: Option<f64>,
    },
    /// One of the camera's own list, such as its pixel formats.
    Choice(Vec<String>),
}

/// The settings panel for `features`: a control for each standard one the
/// camera has, in [`STANDARD_FEATURES`]' order. Others aren't shown.
pub fn settings_panel(ui: &ThingUi, features: &[Feature]) -> Vec<UiElement> {
    let controls: Vec<UiElement> = (STANDARD_FEATURES.iter())
        .filter_map(|(name, property, label)| {
            let feature = features.iter().find(|feature| feature.name == *name)?;
            let control = ui.property(property, label).read_back(READ_BACK_MS);
            Some(match &feature.kind {
                FeatureKind::Number { step: Some(step) } => control.step(*step),
                FeatureKind::Number { step: None } => control,
                FeatureKind::Choice(options) => control.options(
                    options
                        .iter()
                        .map(|option| (option.as_str(), option.as_str())),
                ),
            })
        })
        .map(UiElement::from)
        .collect();
    if controls.is_empty() {
        let text = "This camera reports none of the settings shown here.".into();
        return vec![UiElement::Text { text }];
    }
    vec![UiElement::Container {
        layout: Layout::Column,
        children: controls,
    }]
}
