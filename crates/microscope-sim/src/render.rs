//! Drawing what the camera sees.

use std::sync::Arc;

use image::RgbImage;

use crate::filters::{add_noise, gaussian_blur};
use crate::optics::{Objective, Sensor};
use crate::random::hash;
use crate::specimen::Specimen;
use crate::units::{Point, um_per_px};

/// Blur is capped at this standard deviation, in pixels: a frame blurred
/// more is a featureless wash anyway, and drawing the margin the blur needs
/// would be slow.
pub const MAX_BLUR_PX: f64 = 64.0;

/// Mixed with the frame's number to seed its noise.
const NOISE_SALT: u64 = 0x6E01_5E00;

/// Everything that decides a frame.
#[derive(Clone)]
pub struct SimState {
    /// What is on the slide.
    pub specimen: Arc<dyn Specimen>,
    /// The objective in use.
    pub objective: Objective,
    /// The sensor the frame is drawn for.
    pub sensor: Sensor,
    /// Where the stage is, in µm: x and y are the sample point at the
    /// centre of the field of view; z is the sample's height.
    pub stage_um: [f64; 3],
    /// The z at which the sample is in focus, in µm.
    pub focus_z_um: f64,
    /// Sensor noise, as a standard deviation in grey levels (0–255); 0 for none.
    pub noise: f32,
    /// The frame's number, which seeds its noise.
    pub frame: u64,
}

/// What the camera sees: the specimen around the stage position, blurred by
/// defocus, with sensor noise. The same state draws the same frame.
pub fn render_frame(state: &SimState) -> RgbImage {
    let Sensor { width, height, .. } = state.sensor;
    let scale = um_per_px(state.sensor.pixel_pitch_um, state.objective.magnification);
    let defocus = state.stage_um[2] - state.focus_z_um;
    let sigma = (state.objective.blur_sigma_um(defocus) / scale).min(MAX_BLUR_PX);

    // Draw a margin of two blur widths, so the frame's edges blur like its
    // middle; beyond it, the blur repeats the margin's outermost pixels.
    let pad = if sigma < 0.3 {
        0
    } else {
        (2.0 * sigma).ceil() as u32
    };
    let (w, h) = (width + 2 * pad, height + 2 * pad);
    let origin = Point::new(
        state.stage_um[0] - f64::from(w) / 2.0 * scale,
        state.stage_um[1] - f64::from(h) / 2.0 * scale,
    );
    let drawn = state.specimen.render(origin, scale, w, h);
    let mut samples: Vec<f32> = drawn.into_raw().into_iter().map(f32::from).collect();
    gaussian_blur(&mut samples, w as usize, h as usize, sigma);
    if state.noise > 0.0 {
        add_noise(&mut samples, state.noise, hash(&[NOISE_SALT, state.frame]));
    }

    RgbImage::from_fn(width, height, |x, y| {
        let p = (((y + pad) * w + x + pad) * 3) as usize;
        image::Rgb(std::array::from_fn(|c| {
            samples[p + c].round().clamp(0.0, 255.0) as u8
        }))
    })
}

/// The objective's standard deviation of blur at this defocus, in pixels of
/// `sensor`, before the cap.
pub fn blur_px(objective: &Objective, sensor: &Sensor, defocus_um: f64) -> f64 {
    objective.blur_sigma_um(defocus_um) / um_per_px(sensor.pixel_pitch_um, objective.magnification)
}
