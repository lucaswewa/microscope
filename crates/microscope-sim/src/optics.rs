//! Objectives and sensors: magnification, field of view, depth of field,
//! and the blur that defocus causes.

/// The wavelength light is modelled at: green, in µm.
pub const WAVELENGTH_UM: f64 = 0.55;

/// A microscope objective.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Objective {
    /// Its magnification, such as 40 for 40×.
    pub magnification: f64,
    /// Its numerical aperture.
    pub numerical_aperture: f64,
    /// The refractive index of what is between it and the sample: 1 for
    /// air, about 1.52 for immersion oil.
    pub immersion_index: f64,
}

/// The objectives the simulator offers: 4×, 10×, 20×, 40×, 60× and 100×
/// (oil), with typical numerical apertures.
pub const OBJECTIVES: [Objective; 6] = [
    Objective::dry(4.0, 0.10),
    Objective::dry(10.0, 0.25),
    Objective::dry(20.0, 0.40),
    Objective::dry(40.0, 0.65),
    Objective::dry(60.0, 0.80),
    Objective {
        magnification: 100.0,
        numerical_aperture: 1.25,
        immersion_index: 1.518,
    },
];

impl Objective {
    /// An objective used in air.
    pub const fn dry(magnification: f64, numerical_aperture: f64) -> Self {
        Self {
            magnification,
            numerical_aperture,
            immersion_index: 1.0,
        }
    }

    /// The objective with this magnification, if the simulator offers it.
    pub fn with_magnification(magnification: f64) -> Option<Self> {
        OBJECTIVES
            .into_iter()
            .find(|objective| objective.magnification == magnification)
    }

    /// The depth of field, in µm, on a sensor with this pixel pitch: the
    /// diffraction term λn/NA² plus the detector term n·e/(M·NA).
    pub fn depth_of_field_um(&self, pixel_pitch_um: f64) -> f64 {
        let (n, na) = (self.immersion_index, self.numerical_aperture);
        WAVELENGTH_UM * n / (na * na) + n * pixel_pitch_um / (self.magnification * na)
    }

    /// How blurred a point is, `defocus_um` from focus, as the standard
    /// deviation of a Gaussian in the sample plane, in µm. In focus it is
    /// the diffraction limit; away from it, the cone of light the objective
    /// collects spreads the point over a disc of radius |dz|·tan θ, where
    /// sin θ = NA / n.
    pub fn blur_sigma_um(&self, defocus_um: f64) -> f64 {
        let diffraction = 0.21 * WAVELENGTH_UM / self.numerical_aperture;
        let half_angle = (self.numerical_aperture / self.immersion_index)
            .min(0.999)
            .asin();
        let defocus = defocus_um.abs() * half_angle.tan() / 2.0;
        diffraction.hypot(defocus)
    }
}

/// A camera's sensor, as the simulator draws frames for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sensor {
    /// The frame's width, in pixels.
    pub width: u32,
    /// The frame's height, in pixels.
    pub height: u32,
    /// The distance between pixel centres, in µm.
    pub pixel_pitch_um: f64,
}

impl Sensor {
    /// A preview stream: 820 × 616 pixels, binned 4 × 4 from a 1.12 µm sensor.
    pub const PREVIEW: Sensor = Sensor {
        width: 820,
        height: 616,
        pixel_pitch_um: 4.48,
    };

    /// A capture: 1640 × 1232 pixels, binned 2 × 2 from the same sensor, so
    /// it shows the preview's field of view in twice the detail.
    pub const CAPTURE: Sensor = Sensor {
        width: 1640,
        height: 1232,
        pixel_pitch_um: 2.24,
    };

    /// The part of the sample a frame shows, through `objective`, in µm (width, height).
    pub fn field_of_view_um(&self, objective: &Objective) -> (f64, f64) {
        let um_per_px = self.pixel_pitch_um / objective.magnification;
        (
            f64::from(self.width) * um_per_px,
            f64::from(self.height) * um_per_px,
        )
    }
}
