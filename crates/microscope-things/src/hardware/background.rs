//! What the camera needs from a background detector (P27).

use image::RgbImage;
use teta_wot::prelude::*;

/// A way of telling sample from the empty slide, whatever its method. The
/// camera uses one, by its Thing's name.
#[teta_wot::interface]
pub trait BackgroundDetector: Send + Sync {
    /// Sets the background from `image`, a frame of the empty slide.
    fn set_background(&self, image: &RgbImage) -> Result<(), ActionError>;

    /// The percentage of `image` that holds sample.
    fn sample_coverage(&self, image: &RgbImage) -> Result<f64, ActionError>;

    /// The percentage above which an image holds sample.
    fn min_sample_coverage(&self) -> f64;

    /// Whether the background is set.
    fn ready(&self) -> bool;
}
