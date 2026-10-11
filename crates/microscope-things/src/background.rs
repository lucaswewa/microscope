//! The background detectors (P27): two Things that tell sample from the
//! empty slide, by each pixel's colour (`bg_color_channels_luv`) or by each
//! region's texture (`bg_channel_deviations_luv`). The camera uses one, by
//! name; the behaviour spec opens the P27 notes.

use image::RgbImage;
use microscope_core::background::{BlankImage, ColourStatistics, Luv, TextureStatistics};
use teta_wot::http::axum::Json;
use teta_wot::prelude::*;

use crate::hardware::BackgroundDetector;
use crate::ui::{OwnEntry, ThingUi, UiElement};

/// Compares each pixel's colour, its u* and v*, with the background's.
#[derive(Thing)]
#[thing(interfaces(BackgroundDetector))]
pub struct ColourChannelDetector {
    /// How many standard deviations from the background's mean a pixel's
    /// u* or v* must be, to count as sample.
    #[setting(default = 7.0, ge = 0.0)]
    channel_tolerance: Prop<f64>,
    /// The percentage of the image that must be sample for it to hold
    /// sample.
    #[setting(default = 25.0, ge = 0.0, le = 100.0)]
    min_sample_coverage: Prop<f64>,
    /// The background's mean L*, u* and v*, once it's set.
    #[setting(default = None, readonly)]
    background_means: Prop<Option<[f64; 3]>>,
    /// Their standard deviations, once it's set.
    #[setting(default = None, readonly)]
    background_deviations: Prop<Option<[f64; 3]>>,
    entry: OwnEntry,
}

#[thing_impl]
impl ColourChannelDetector {
    /// Whether the background is set.
    #[property]
    async fn ready(&self) -> bool {
        self.statistics().is_some()
    }

    /// The settings, described for the settings panel (ADR-0024).
    #[endpoint(get, "settings_ui")]
    async fn settings_ui(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(panel)
    }

    /// Keeps the Thing's registry entry, for its panel's names.
    #[on_start]
    async fn keep_entry(&self, ctx: ThingCtx) {
        self.entry.keep::<Self>(&ctx);
    }
}

impl ColourChannelDetector {
    fn statistics(&self) -> Option<ColourStatistics> {
        Some(ColourStatistics {
            means: self.background_means.get()?,
            deviations: self.background_deviations.get()?,
        })
    }
}

impl BackgroundDetector for ThingRef<ColourChannelDetector> {
    fn set_background(&self, image: &RgbImage) -> Result<(), ActionError> {
        let thing = ThingRef::thing(self);
        let statistics = ColourStatistics::of(&luv(image)).map_err(blank)?;
        let means = thing.background_means.set(Some(statistics.means));
        let deviations = thing.background_deviations.set(Some(statistics.deviations));
        means.and(deviations).map_err(ActionError::handled)
    }

    fn sample_coverage(&self, image: &RgbImage) -> Result<f64, ActionError> {
        let thing = ThingRef::thing(self);
        let statistics = thing.statistics().ok_or_else(unset)?;
        Ok(statistics.sample_coverage(&luv(image), thing.channel_tolerance.get()))
    }

    fn min_sample_coverage(&self) -> f64 {
        ThingRef::thing(self).min_sample_coverage.get()
    }

    fn ready(&self) -> bool {
        ThingRef::thing(self).statistics().is_some()
    }
}

/// Compares each region's texture, the deviation of its colours, with the
/// background's, over an 8 × 8 grid.
#[derive(Thing)]
#[thing(interfaces(BackgroundDetector))]
pub struct ChannelDeviationDetector {
    /// How many times the background's deviation a region's must be, in any
    /// channel, to count as sample.
    #[setting(default = 7.0, ge = 0.0)]
    channel_tolerance: Prop<f64>,
    /// The percentage of the image that must be sample for it to hold
    /// sample.
    #[setting(default = 25.0, ge = 0.0, le = 100.0)]
    min_sample_coverage: Prop<f64>,
    /// The background's median deviations of L*, u* and v*, once it's set.
    #[setting(default = None, readonly)]
    background_deviations: Prop<Option<[f64; 3]>>,
    entry: OwnEntry,
}

#[thing_impl]
impl ChannelDeviationDetector {
    /// Whether the background is set.
    #[property]
    async fn ready(&self) -> bool {
        self.background_deviations.get().is_some()
    }

    /// The settings, described for the settings panel (ADR-0024).
    #[endpoint(get, "settings_ui")]
    async fn settings_ui(&self) -> Json<Vec<UiElement>> {
        self.entry.panel(panel)
    }

    /// Keeps the Thing's registry entry, for its panel's names.
    #[on_start]
    async fn keep_entry(&self, ctx: ThingCtx) {
        self.entry.keep::<Self>(&ctx);
    }
}

impl BackgroundDetector for ThingRef<ChannelDeviationDetector> {
    fn set_background(&self, image: &RgbImage) -> Result<(), ActionError> {
        let statistics = TextureStatistics::of(&luv(image)).map_err(blank)?;
        let deviations = Some(statistics.deviations);
        let thing = ThingRef::thing(self);
        thing
            .background_deviations
            .set(deviations)
            .map_err(ActionError::handled)
    }

    fn sample_coverage(&self, image: &RgbImage) -> Result<f64, ActionError> {
        let thing = ThingRef::thing(self);
        let deviations = thing.background_deviations.get().ok_or_else(unset)?;
        let statistics = TextureStatistics { deviations };
        Ok(statistics.sample_coverage(&luv(image), thing.channel_tolerance.get()))
    }

    fn min_sample_coverage(&self) -> f64 {
        ThingRef::thing(self).min_sample_coverage.get()
    }

    fn ready(&self) -> bool {
        ThingRef::thing(self).background_deviations.get().is_some()
    }
}

fn luv(image: &RgbImage) -> Luv {
    Luv::from_rgb8(
        image.width() as usize,
        image.height() as usize,
        image.as_raw(),
    )
}

fn blank(_: BlankImage) -> ActionError {
    ActionError::handled("the image is blank: check the camera is running and the light is on")
}

fn unset() -> ActionError {
    ActionError::handled("the background isn't set: calibrate the camera first")
}

/// A detector's settings panel: its two settings.
fn panel(ui: &ThingUi) -> Vec<UiElement> {
    vec![
        ui.property("channel_tolerance", "Channel tolerance").into(),
        ui.property("min_sample_coverage", "Sample coverage required (%)")
            .into(),
    ]
}
