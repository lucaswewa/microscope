//! Drawing what the camera sees.

use std::sync::Arc;

use image::RgbImage;

use crate::filters::{add_noise, gaussian_blur};
use crate::noise::value_noise;
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
/// Seeds the focal surface's waviness.
const WAVE_SEED: u64 = 0x5A7E_0001;
/// The size of the waviness's features, in µm.
const WAVE_UM: f64 = 2000.0;

/// Imperfections of the optics, and the sensor's response. By default
/// there are none, at the exposure and gain that show the empty slide at its
/// own brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effects {
    /// Where the sample is in focus across the slide.
    pub focal_surface: FocalSurface,
    /// The share of the light lost in the frame's corners, and less towards
    /// its centre.
    pub vignetting: f64,
    /// The share of the light lost at the frame's most dimly lit corner,
    /// bottom right, as if the lamp were off centre.
    pub uneven_illumination: f64,
    /// The exposure, relative to the one that shows the empty slide at its
    /// own brightness.
    pub exposure: f64,
    /// The sensor's gain, which brightens the frame and its noise alike.
    pub gain: f64,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            focal_surface: FocalSurface::default(),
            vignetting: 0.0,
            uneven_illumination: 0.0,
            exposure: 1.0,
            gain: 1.0,
        }
    }
}

/// The height of the slide's surface, where the sample is in focus: a tilt,
/// plus a gentle waviness.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FocalSurface {
    /// The tilt, in µm of height per mm, in x and y.
    pub tilt_um_per_mm: [f64; 2],
    /// How far the waviness goes up and down, in µm.
    pub waviness_um: f64,
}

impl FocalSurface {
    /// The surface's height at `(x, y)` on the slide, in µm.
    pub fn height_um(&self, x_um: f64, y_um: f64) -> f64 {
        let [tilt_x, tilt_y] = self.tilt_um_per_mm;
        let wave = 2.0 * value_noise(WAVE_SEED, x_um / WAVE_UM, y_um / WAVE_UM) - 1.0;
        (tilt_x * x_um + tilt_y * y_um) / 1000.0 + self.waviness_um * wave
    }
}

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
    /// The optics' imperfections, and the exposure and gain.
    pub effects: Effects,
}

/// What the camera sees: the specimen around the stage position, blurred by
/// its distance from the focal surface, lit and exposed as `effects` say,
/// with sensor noise. The same state draws the same frame.
pub fn render_frame(state: &SimState) -> RgbImage {
    let Sensor { width, height, .. } = state.sensor;
    let scale = um_per_px(state.sensor.pixel_pitch_um, state.objective.magnification);
    let [x, y, z] = state.stage_um;
    let surface = state.effects.focal_surface.height_um(x, y);
    let defocus = z + surface - state.focus_z_um;
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
    expose(
        &mut samples,
        (w as usize, h as usize),
        pad as usize,
        &state.effects,
    );
    if state.noise > 0.0 {
        let noise = state.noise * state.effects.gain as f32;
        add_noise(&mut samples, noise, hash(&[NOISE_SALT, state.frame]));
    }

    RgbImage::from_fn(width, height, |x, y| {
        let p = (((y + pad) * w + x + pad) * 3) as usize;
        image::Rgb(std::array::from_fn(|c| {
            samples[p + c].round().clamp(0.0, 255.0) as u8
        }))
    })
}

/// Scales each sample by the light that reaches its pixel: the exposure and
/// gain, less what vignetting and uneven lighting take. The frame is the
/// middle of the samples, inside a margin of `pad` pixels.
fn expose(samples: &mut [f32], (w, h): (usize, usize), pad: usize, effects: &Effects) {
    let exposed = (effects.exposure * effects.gain) as f32;
    if exposed == 1.0 && effects.vignetting == 0.0 && effects.uneven_illumination == 0.0 {
        return;
    }
    let (frame_w, frame_h) = ((w - 2 * pad) as f64, (h - 2 * pad) as f64);
    for y in 0..h {
        // Across the frame, from 0 to 1.
        let v = (y as f64 - pad as f64 + 0.5) / frame_h;
        for x in 0..w {
            let u = (x as f64 - pad as f64 + 0.5) / frame_w;
            let corner = 2.0 * ((u - 0.5).powi(2) + (v - 0.5).powi(2));
            let vignette = 1.0 - effects.vignetting * corner;
            let lamp = 1.0 - effects.uneven_illumination * (u + v) / 2.0;
            let factor = exposed * (vignette * lamp).max(0.0) as f32;
            for sample in &mut samples[(y * w + x) * 3..][..3] {
                *sample *= factor;
            }
        }
    }
}

/// The objective's standard deviation of blur at this defocus, in pixels of
/// `sensor`, before the cap.
pub fn blur_px(objective: &Objective, sensor: &Sensor, defocus_um: f64) -> f64 {
    objective.blur_sigma_um(defocus_um) / um_per_px(sensor.pixel_pitch_um, objective.magnification)
}
