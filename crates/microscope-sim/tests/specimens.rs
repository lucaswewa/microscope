//! Specimens II and optical effects (P25): tissue and pictures on the
//! slide, the focal surface, vignetting, uneven lighting, and exposure and
//! gain. The benchmark is an ignored test:
//! `cargo test --release -p microscope-sim --test specimens -- --ignored --nocapture`.

use std::sync::Arc;
use std::time::Instant;

use image::RgbImage;
use microscope_sim::{
    BlobConfig, BlobSpecimen, Effects, FocalSurface, ImageSpecimen, Objective, Point, Sensor,
    SimState, Specimen, TissueConfig, TissueSpecimen, render_frame,
};

const SMALL: Sensor = Sensor {
    width: 200,
    height: 150,
    pixel_pitch_um: 4.48,
};

fn tissue(coverage: f64) -> TissueSpecimen {
    TissueSpecimen::new(TissueConfig {
        coverage,
        ..TissueConfig::default()
    })
}

/// A frame at 40× of `specimen`, with the stage at `stage_um`.
fn frame(specimen: Arc<dyn Specimen>, stage_um: [f64; 3], effects: Effects) -> RgbImage {
    render_frame(&SimState {
        specimen,
        objective: Objective::with_magnification(40.0).expect("offered"),
        sensor: SMALL,
        stage_um,
        focus_z_um: 0.0,
        noise: 0.0,
        frame: 0,
        effects,
    })
}

fn blobs() -> Arc<dyn Specimen> {
    Arc::new(BlobSpecimen::new(BlobConfig::default()))
}

/// The empty slide.
fn blank() -> Arc<dyn Specimen> {
    Arc::new(BlobSpecimen::new(BlobConfig {
        density_per_mm2: 0.0,
        ..BlobConfig::default()
    }))
}

/// How sharp a frame is: the mean squared difference between neighbours.
fn sharpness(frame: &RgbImage) -> f64 {
    let (w, h) = frame.dimensions();
    let pairs = (1..w).flat_map(|x| (0..h).map(move |y| (x, y)));
    let squares: f64 = pairs
        .map(|(x, y)| {
            let d = |p: &image::Rgb<u8>| f64::from(p.0[1]);
            (d(frame.get_pixel(x, y)) - d(frame.get_pixel(x - 1, y))).powi(2)
        })
        .sum();
    squares / f64::from((w - 1) * h)
}

/// The share of pixels that are the empty slide.
fn empty_share(frame: &RgbImage) -> f64 {
    let empty = frame.pixels().filter(|p| p.0.iter().all(|&c| c >= 236));
    empty.count() as f64 / f64::from(frame.width() * frame.height())
}

#[test]
fn tissue_is_the_same_wherever_it_is_drawn_from() {
    let specimen = tissue(0.7);
    let s = 0.5;
    let whole = specimen.render(Point::new(-300.0, 100.0), s, 400, 160);
    assert_eq!(
        whole,
        specimen.render(Point::new(-300.0, 100.0), s, 400, 160)
    );
    let right = specimen.render(Point::new(-300.0 + 200.0 * s, 100.0), s, 200, 160);
    let worst = (0..200)
        .flat_map(|x| (0..160).map(move |y| (x, y)))
        .flat_map(|(x, y)| {
            let (a, b) = (whole.get_pixel(x + 200, y).0, right.get_pixel(x, y).0);
            (0..3).map(move |c| a[c].abs_diff(b[c]))
        })
        .max();
    assert!(worst <= Some(1), "{worst:?}");
}

#[test]
fn tissue_has_gaps_and_its_coverage_sets_how_much() {
    let empty = |coverage: f64| {
        empty_share(&tissue(coverage).render(Point::new(-1000.0, -1000.0), 2.0, 1000, 1000))
    };
    let (sparse, dense) = (empty(0.3), empty(0.8));
    assert!((sparse - 0.7).abs() < 0.2, "{sparse}");
    assert!((dense - 0.2).abs() < 0.2, "{dense}");
    // No tissue, no nuclei either; and nothing beyond the 20 mm section.
    assert_eq!(empty(0.0), 1.0);
    let beyond = tissue(1.0).render(Point::new(10_100.0, 0.0), 2.0, 100, 100);
    assert_eq!(empty_share(&beyond), 1.0);
}

#[test]
fn tissue_has_pink_stroma_and_purple_nuclei() {
    let section = tissue(1.0).render(Point::new(0.0, 0.0), 0.5, 400, 400);
    let count = |test: fn([i32; 3]) -> bool| {
        section
            .pixels()
            .filter(|p| test(p.0.map(i32::from)))
            .count()
    };
    let stroma = count(|[r, g, b]| r > g + 40 && r > b);
    let nuclei = count(|[r, _, b]| b > r + 30);
    assert!(stroma > 50_000, "{stroma}");
    assert!(nuclei > 500, "{nuclei}");
}

#[test]
fn a_picture_is_sampled_at_its_scale_and_averaged_when_coarser() {
    // 40 × 20 pixels, 2 µm each: 80 × 40 µm, centred on the origin.
    let picture = RgbImage::from_fn(40, 20, |x, y| {
        image::Rgb([(x * 6) as u8, (y * 12) as u8, 100])
    });
    let specimen = ImageSpecimen::new(picture.clone(), 2.0);
    assert_eq!(
        specimen.render(Point::new(-40.0, -20.0), 2.0, 40, 20),
        picture
    );

    // Around it, the empty slide.
    let around = specimen.render(Point::new(-60.0, -20.0), 2.0, 5, 1);
    assert_eq!(around.get_pixel(0, 0).0, [246, 244, 240]);

    // Pixels four times larger show the average of the four by four they cover.
    let coarse = specimen.render(Point::new(-40.0, -20.0), 8.0, 10, 5);
    for (x, y, pixel) in coarse.enumerate_pixels() {
        let block = (0..16).map(|i| picture.get_pixel(4 * x + i % 4, 4 * y + i / 4).0);
        let mean: Vec<f64> = (0..3)
            .map(|c| block.clone().map(|p| f64::from(p[c])).sum::<f64>() / 16.0)
            .collect();
        for c in 0..3 {
            assert!(
                (f64::from(pixel.0[c]) - mean[c]).abs() <= 2.0,
                "({x}, {y}): {pixel:?} {mean:?}"
            );
        }
    }
    // Every fourth column lit: sampling between columns would miss them all.
    let columns = RgbImage::from_fn(40, 20, |x, _| {
        image::Rgb([if x % 4 == 0 { 255 } else { 0 }; 3])
    });
    let coarse = ImageSpecimen::new(columns, 2.0).render(Point::new(-40.0, -20.0), 8.0, 10, 5);
    assert!(
        coarse.pixels().all(|p| p.0[0].abs_diff(64) <= 1),
        "{coarse:?}"
    );
}

#[test]
fn pictures_load_from_png_jpeg_and_tiff_files() {
    let picture = RgbImage::from_fn(32, 16, |x, y| {
        image::Rgb([(x * 8) as u8, (y * 16) as u8, 60])
    });
    let folder = std::env::temp_dir().join(format!("microscope-pictures-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("made");
    for (name, tolerance) in [("a.png", 0), ("a.tiff", 0), ("a.jpg", 16)] {
        let path = folder.join(name);
        picture.save(&path).expect("saved");
        let drawn = ImageSpecimen::open(&path, 1.0).expect("loaded").render(
            Point::new(-16.0, -8.0),
            1.0,
            32,
            16,
        );
        let worst = drawn
            .pixels()
            .zip(picture.pixels())
            .flat_map(|(a, b)| (0..3).map(move |c| a.0[c].abs_diff(b.0[c])))
            .max();
        assert!(worst <= Some(tolerance), "{name}: {worst:?}");
    }
    std::fs::remove_dir_all(&folder).expect("removed");
}

#[test]
fn the_focal_surface_changes_sharpness_across_x_and_y() {
    for tilt in [[20.0, 0.0], [0.0, 20.0]] {
        let effects = Effects {
            focal_surface: FocalSurface {
                tilt_um_per_mm: tilt,
                waviness_um: 0.0,
            },
            ..Effects::default()
        };
        // 1 mm along the tilt, the surface is 20 µm higher: out of focus.
        let far = [tilt[0] / 20.0 * 1000.0, tilt[1] / 20.0 * 1000.0];
        let here = sharpness(&frame(blobs(), [0.0, 0.0, 0.0], effects));
        let there = sharpness(&frame(blobs(), [far[0], far[1], 0.0], effects));
        let refocused = sharpness(&frame(blobs(), [far[0], far[1], -20.0], effects));
        assert!(there < here / 4.0, "{tilt:?}: {here} then {there}");
        assert!(
            refocused > there * 4.0,
            "{tilt:?}: {there} then {refocused}"
        );
    }
    let wavy = FocalSurface {
        tilt_um_per_mm: [0.0, 0.0],
        waviness_um: 5.0,
    };
    let heights: Vec<f64> = (0..20)
        .map(|i| wavy.height_um(f64::from(i) * 700.0, f64::from(i) * 300.0))
        .collect();
    assert!(heights.iter().all(|h| h.abs() <= 5.0));
    let spread = heights.iter().copied().fold(f64::MIN, f64::max)
        - heights.iter().copied().fold(f64::MAX, f64::min);
    assert!(spread > 3.0, "{heights:?}");
}

#[test]
fn vignetting_and_uneven_lighting_darken_the_edges() {
    let level = |frame: &RgbImage, x: u32, y: u32| f64::from(frame.get_pixel(x, y).0[1]);
    let vignetted = frame(
        blank(),
        [0.0; 3],
        Effects {
            vignetting: 0.4,
            ..Effects::default()
        },
    );
    let (centre, corner) = (level(&vignetted, 100, 75), level(&vignetted, 0, 0));
    assert!(
        (centre - 244.0).abs() < 2.0 && (corner / centre - 0.6).abs() < 0.03,
        "{centre} {corner}"
    );

    let uneven = frame(
        blank(),
        [0.0; 3],
        Effects {
            uneven_illumination: 0.4,
            ..Effects::default()
        },
    );
    let (top_left, bottom_right) = (level(&uneven, 0, 0), level(&uneven, 199, 149));
    assert!(
        top_left > 240.0 && (bottom_right / 244.0 - 0.6).abs() < 0.03,
        "{top_left} {bottom_right}"
    );
}

#[test]
fn exposure_and_gain_brighten_the_frame_and_gain_its_noise() {
    let state = |exposure: f64, gain: f64| SimState {
        specimen: blank(),
        objective: Objective::with_magnification(40.0).expect("offered"),
        sensor: SMALL,
        stage_um: [0.0; 3],
        focus_z_um: 0.0,
        noise: 2.0,
        frame: 0,
        effects: Effects {
            exposure,
            gain,
            ..Effects::default()
        },
    };
    let stats = |frame: &RgbImage| {
        let values: Vec<f64> = frame.pixels().map(|p| f64::from(p.0[1])).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        (mean, variance.sqrt())
    };
    let (normal, normal_noise) = stats(&render_frame(&state(1.0, 1.0)));
    let (half, _) = stats(&render_frame(&state(0.5, 1.0)));
    let (gained, gained_noise) = stats(&render_frame(&state(0.25, 4.0)));
    let (saturated, _) = stats(&render_frame(&state(2.0, 1.0)));
    assert!((half / normal - 0.5).abs() < 0.02, "{normal} {half}");
    assert!((gained / normal - 1.0).abs() < 0.02, "{normal} {gained}");
    assert!(
        (gained_noise / normal_noise - 4.0).abs() < 0.5,
        "{normal_noise} {gained_noise}"
    );
    assert!(saturated > 254.0, "{saturated}");
}

/// A preview frame of tissue at 4×, 10× and 40×, in focus, in a release build.
#[test]
#[ignore = "a benchmark: run with --release -- --ignored --nocapture"]
fn benchmark_a_tissue_preview_frame() {
    let specimen: Arc<dyn Specimen> = Arc::new(tissue(0.7));
    for magnification in [4.0, 10.0, 40.0] {
        let state = SimState {
            specimen: Arc::clone(&specimen),
            objective: Objective::with_magnification(magnification).expect("offered"),
            sensor: Sensor::PREVIEW,
            stage_um: [0.0; 3],
            focus_z_um: 0.0,
            noise: 2.0,
            frame: 0,
            effects: Effects::default(),
        };
        render_frame(&state);
        let start = Instant::now();
        for frame in 0..10 {
            render_frame(&SimState {
                frame,
                ..state.clone()
            });
        }
        let ms = start.elapsed().as_secs_f64() * 1e3 / 10.0;
        println!("tissue, 820×616 at {magnification}×: {ms:.1} ms");
    }
}
