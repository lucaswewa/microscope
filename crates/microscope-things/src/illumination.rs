//! The `illumination` Thing in simulation: `SimulatedIllumination`, an LED
//! that the simulated camera's frames depend on. With it off, they are
//! black.

use teta_wot::prelude::*;

use crate::hardware::IlluminationApi;

/// A simulated LED, mirroring OpenFlexure's illumination.
#[derive(Thing)]
#[thing(interfaces(IlluminationApi))]
pub struct SimulatedIllumination {
    /// Whether the LED is on.
    #[property(default = true, readonly)]
    led_on: Prop<bool>,
}

#[thing_impl]
impl SimulatedIllumination {
    /// Switches the LED on or off.
    #[action]
    async fn set_led(&self, #[param(default = true)] led_on: bool) -> Result<(), ActionError> {
        self.led_on.set(led_on).map_err(ActionError::handled)
    }

    /// Flashes the LED: off for `dt` seconds, then on for `dt` seconds,
    /// `number_of_flashes` times. It ends on, even when cancelled.
    #[action]
    async fn flash(
        &self,
        ctx: ActionCtx,
        #[param(default = 10)] number_of_flashes: u32,
        #[param(default = 0.5)] dt: f64,
    ) -> Result<(), ActionError> {
        if !(0.0..=60.0).contains(&dt) {
            return Err(ActionError::handled("dt must be from 0 to 60 seconds"));
        }
        let half = std::time::Duration::from_secs_f64(dt);
        let mut flashed = Ok(());
        for _ in 0..number_of_flashes {
            self.led_on.set(false).map_err(ActionError::handled)?;
            flashed = ctx.sleep(half).await;
            self.led_on.set(true).map_err(ActionError::handled)?;
            if flashed.is_ok() {
                flashed = ctx.sleep(half).await;
            }
            if flashed.is_err() {
                break;
            }
        }
        Ok(flashed?)
    }
}

impl IlluminationApi for ThingRef<SimulatedIllumination> {
    fn set_led(&self, on: bool) -> BoxFuture<'_, Result<(), ActionError>> {
        SimulatedIlluminationActions::set_led(self, on)
    }

    fn led_on(&self) -> bool {
        ThingRef::thing(self).led_on.get()
    }
}
