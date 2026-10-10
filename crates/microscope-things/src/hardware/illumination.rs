//! What other Things need from the illumination.

use teta_wot::prelude::*;

/// What other Things need from the light source, whatever drives it: to
/// switch it on for imaging, and off for background measurements.
#[teta_wot::interface]
pub trait IlluminationApi: Send + Sync {
    /// Switches the light on or off.
    fn set_led(&self, on: bool) -> BoxFuture<'_, Result<(), ActionError>>;

    /// Whether the light is on.
    fn led_on(&self) -> bool;
}
