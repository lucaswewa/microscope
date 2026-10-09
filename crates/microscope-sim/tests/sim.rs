//! The simulator's behaviour: determinism, the frame's geometry and scale,
//! blur growing with defocus, the specimen's extent, levels of detail and
//! noise. Benchmarks are ignored tests:
//! `cargo test --release -p microscope-sim -- --ignored --nocapture`.

use std::sync::Arc;
use std::time::Instant;

use image::RgbImage;
use microscope_sim::units::um_per_px;
use microscope_sim::{
    BlobConfig, BlobSpecimen, Extent, OBJECTIVES, Objective, Point, Sensor, SimState, Specimen,
    StageScale, render_frame,
};

const SMALL: Sensor = Sensor {
    width: 200,
    height: 150,
    pixel_pitch_um: 4.48,
};

fn objective(magnification: f64) -> Objective {
    Objective::with_magnification(magnification).expect("offered")
}

fn state(specimen: Arc<dyn Specimen>, magnification: f64, stage_um: [f64; 3]) -> SimState {
    SimState {
        specimen,
        objective: objective(magnification),
        sensor: SMALL,
        stage_um,
        focus_z_um: 0.0,
        noise: 0.0,
        frame: 0,
    }
}

fn blobs(extent: Extent) -> Arc<BlobSpecimen> {
    Arc::new(BlobSpecimen::new(BlobConfig {
        extent,
        ..BlobConfig::default()
    }))
}

/// Dark vertical stripes, `period_um` apart, half a period wide.
struct Stripes(f64);

impl Specimen for Stripes {
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, _| {
            let um = origin.x + (f64::from(x) + 0.5) * um_per_px;
            let dark = (um / (self.0 / 2.0)).floor().rem_euclid(2.0) == 0.0;
            image::Rgb(if dark { [0; 3] } else { [255; 3] })
        })
    }
}

/// A dark disc of 2 µm radius at a point.
struct Dot(Point);

impl Specimen for Dot {
    fn render(&self, origin: Point, um_per_px: f64, width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| {
            let dx = origin.x + (f64::from(x) + 0.5) * um_per_px - self.0.x;
            let dy = origin.y + (f64::from(y) + 0.5) * um_per_px - self.0.y;
            image::Rgb(if dx.hypot(dy) < 2.0 { [0; 3] } else { [255; 3] })
        })
    }
}

/// The mean of the darkest pixels' coordinates.
fn darkest(frame: &RgbImage) -> (f64, f64) {
    let min = frame.pixels().map(|p| p.0[0]).min().expect("pixels");
    let dark: Vec<_> = frame
        .enumerate_pixels()
        .filter(|(_, _, p)| p.0[0] == min)
        .map(|(x, y, _)| (f64::from(x), f64::from(y)))
        .collect();
    let n = dark.len() as f64;
    (
        dark.iter().map(|p| p.0).sum::<f64>() / n,
        dark.iter().map(|p| p.1).sum::<f64>() / n,
    )
}

/// How sharp a frame is: the mean squared difference between neighbours.
fn sharpness(frame: &RgbImage) -> f64 {
    let (w, h) = frame.dimensions();
    let mut sum = 0.0;
    for y in 0..h {
        for x in 1..w {
            let d =
                f64::from(frame.get_pixel(x, y).0[1]) - f64::from(frame.get_pixel(x - 1, y).0[1]);
            sum += d * d;
        }
    }
    sum / f64::from((w - 1) * h)
}

#[test]
fn the_same_state_draws_the_same_frame() {
    let a = render_frame(&state(
        blobs(BlobConfig::default().extent),
        40.0,
        [30.0, -20.0, 3.0],
    ));
    let b = render_frame(&state(
        blobs(BlobConfig::default().extent),
        40.0,
        [30.0, -20.0, 3.0],
    ));
    assert_eq!(a, b);
    let other = Arc::new(BlobSpecimen::new(BlobConfig {
        seed: 2,
        ..BlobConfig::default()
    }));
    assert_ne!(a, render_frame(&state(other, 40.0, [30.0, -20.0, 3.0])));
}

#[test]
fn a_region_drawn_in_two_halves_matches_it_drawn_whole() {
    let specimen = BlobSpecimen::new(BlobConfig::default());
    let s = 0.5;
    let whole = specimen.render(Point::new(-100.0, -40.0), s, 400, 160);
    let left = specimen.render(Point::new(-100.0, -40.0), s, 200, 160);
    let right = specimen.render(Point::new(0.0, -40.0), s, 200, 160);
    for (x, y, pixel) in whole.enumerate_pixels() {
        let half = if x < 200 {
            left.get_pixel(x, y)
        } else {
            right.get_pixel(x - 200, y)
        };
        assert_eq!(pixel, half, "({x}, {y})");
    }
}

#[test]
fn a_pixel_covers_the_pixel_pitch_over_the_magnification() {
    for magnification in [10.0, 40.0] {
        let frame = render_frame(&state(Arc::new(Stripes(20.0)), magnification, [0.0; 3]));
        // Count the dark–light edges along a row.
        let row: Vec<u8> = (0..SMALL.width)
            .map(|x| frame.get_pixel(x, 75).0[0])
            .collect();
        let edges = row
            .windows(2)
            .filter(|pair| (pair[0] < 128) != (pair[1] < 128))
            .count();
        let width_um = f64::from(SMALL.width) * um_per_px(SMALL.pixel_pitch_um, magnification);
        let expected = width_um / 10.0; // an edge every half period
        assert!(
            (edges as f64 - expected).abs() <= 1.5,
            "{magnification}×: {edges} edges, expected {expected}"
        );
    }
    let (width, height) = Sensor::PREVIEW.field_of_view_um(&objective(40.0));
    assert!((width - 91.84).abs() < 1e-9 && (height - 68.992).abs() < 1e-9);
}

#[test]
fn the_stage_position_is_the_sample_point_at_the_centre() {
    let dot = Arc::new(Dot(Point::new(10.0, -5.0)));
    let (cx, cy) = darkest(&render_frame(&state(dot.clone(), 40.0, [10.0, -5.0, 0.0])));
    assert!(
        (cx - 99.5).abs() < 1.0 && (cy - 74.5).abs() < 1.0,
        "({cx}, {cy})"
    );
    // With the stage at the origin, the dot is right of centre and above it.
    let (cx, cy) = darkest(&render_frame(&state(dot, 40.0, [0.0; 3])));
    let s = um_per_px(SMALL.pixel_pitch_um, 40.0);
    assert!((cx - (99.5 + 10.0 / s)).abs() < 1.0, "{cx}");
    assert!((cy - (74.5 - 5.0 / s)).abs() < 1.0, "{cy}");
}

#[test]
fn blur_grows_with_defocus_either_way() {
    let specimen = blobs(BlobConfig::default().extent);
    let at = |dz: f64| {
        sharpness(&render_frame(&state(
            specimen.clone(),
            40.0,
            [0.0, 0.0, dz],
        )))
    };
    let series: Vec<f64> = [0.0, 1.0, 3.0, 6.0, 12.0].map(at).into();
    assert!(
        series.windows(2).all(|pair| pair[0] > pair[1]),
        "{series:?}"
    );
    assert!((at(5.0) - at(-5.0)).abs() < 1e-9);
    // Moving the focus is the same as moving the sample.
    let refocused = SimState {
        focus_z_um: 5.0,
        ..state(specimen.clone(), 40.0, [0.0; 3])
    };
    assert_eq!(sharpness(&render_frame(&refocused)), at(-5.0));
}

#[test]
fn the_depth_of_field_is_shallower_at_higher_magnification() {
    let depths: Vec<f64> = OBJECTIVES
        .iter()
        .map(|o| o.depth_of_field_um(4.48))
        .collect();
    assert!(
        depths.windows(2).all(|pair| pair[0] > pair[1]),
        "{depths:?}"
    );
}

#[test]
fn a_finite_sample_has_an_empty_margin() {
    let config = BlobConfig {
        extent: Extent::Finite {
            width_um: 1000.0,
            height_um: 1000.0,
        },
        ..BlobConfig::default()
    };
    let background = config.background;
    let specimen = BlobSpecimen::new(config);
    // Far away, and from just beyond the largest radius past the right-hand edge.
    for origin in [Point::new(5000.0, 0.0), Point::new(500.0 + 8.0, -300.0)] {
        let region = specimen.render(origin, 0.5, 200, 1200);
        assert!(region.pixels().all(|p| p.0 == background), "{origin:?}");
    }
    // Inside the edge, there are blobs.
    let inside = specimen.render(Point::new(400.0, -300.0), 0.5, 200, 1200);
    assert!(inside.pixels().any(|p| p.0 != background));
}

#[test]
fn a_repeating_sample_repeats() {
    let specimen = BlobSpecimen::new(BlobConfig {
        extent: Extent::Repeating { period_um: 100.0 },
        ..BlobConfig::default()
    });
    let period = specimen.period_um().expect("repeating");
    assert!((period - 96.0).abs() < 1e-9, "{period}"); // three 32 µm cells
    let here = specimen.render(Point::new(-50.0, -50.0), 0.5, 100, 100);
    let there = specimen.render(
        Point::new(-50.0 + 3.0 * period, -50.0 - period),
        0.5,
        100,
        100,
    );
    assert_eq!(here, there);
}

#[test]
fn blobs_smaller_than_a_pixel_still_darken_it() {
    let specimen = BlobSpecimen::new(BlobConfig::default());
    // 20 µm per pixel: every blob is smaller than a pixel.
    let overview = specimen.render(Point::new(-2000.0, -2000.0), 20.0, 200, 200);
    let background = BlobConfig::default().background;
    let darkened = overview.pixels().filter(|p| p.0 != background).count();
    assert!(darkened > 1000, "{darkened}");
}

#[test]
fn noise_differs_from_frame_to_frame() {
    let quiet = state(blobs(BlobConfig::default().extent), 40.0, [0.0; 3]);
    let noisy = |frame| {
        render_frame(&SimState {
            noise: 5.0,
            frame,
            ..quiet.clone()
        })
    };
    assert_eq!(noisy(1), noisy(1));
    assert_ne!(noisy(1), noisy(2));
    let clean = render_frame(&quiet);
    let differences: Vec<f64> = noisy(1)
        .as_raw()
        .iter()
        .zip(clean.as_raw())
        .map(|(a, b)| f64::from(*a) - f64::from(*b))
        .collect();
    let n = differences.len() as f64;
    let mean = differences.iter().sum::<f64>() / n;
    let sd = (differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n).sqrt();
    assert!(
        mean.abs() < 0.2 && (sd - 5.0).abs() < 0.5,
        "mean {mean}, sd {sd}"
    );
}

#[test]
fn steps_and_um_convert_both_ways() {
    let scale = StageScale {
        um_per_step: [0.1, 0.1, 0.05],
    };
    assert_eq!(scale.to_um([10, -20, 30]), [1.0, -2.0, 1.5]);
    assert_eq!(scale.to_steps([1.0, -2.0, 1.5]), [10, -20, 30]);
    assert_eq!(scale.to_steps([0.149, 0.151, -0.024]), [1, 2, 0]);
}

/// A preview frame at 40×, in focus and defocused, in a release build.
#[test]
#[ignore = "a benchmark: run with --release -- --ignored --nocapture"]
fn benchmark_a_preview_frame() {
    let specimen = blobs(BlobConfig::default().extent);
    for dz in [0.0, 5.0, 20.0, 40.0] {
        let state = SimState {
            sensor: Sensor::PREVIEW,
            ..state(specimen.clone(), 40.0, [0.0, 0.0, dz])
        };
        render_frame(&state);
        let start = Instant::now();
        let runs = 20;
        for frame in 0..runs {
            render_frame(&SimState {
                frame,
                ..state.clone()
            });
        }
        let ms = start.elapsed().as_secs_f64() * 1e3 / runs as f64;
        println!("820×616 at 40×, {dz:>4} µm from focus: {ms:.1} ms");
    }
}
