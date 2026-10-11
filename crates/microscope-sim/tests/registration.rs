//! Phase correlation (P22) on frames the simulator draws: the image moves
//! against the stage by the stage's move in pixels, and an empty slide
//! gives nothing to go by.

use std::sync::Arc;

use microscope_core::registration::{Gray, Options, register};
use microscope_sim::{BlobConfig, BlobSpecimen, Objective, Sensor, SimState, render_frame};

fn frame(specimen: &Arc<BlobSpecimen>, x_um: f64, y_um: f64, number: u64) -> Gray {
    let frame = render_frame(&SimState {
        specimen: specimen.clone(),
        objective: Objective::with_magnification(40.0).expect("40× is offered"),
        sensor: Sensor::PREVIEW,
        stage_um: [x_um, y_um, 0.0],
        focus_z_um: 0.0,
        noise: 2.0,
        frame: number,
    });
    Gray::from_rgb8(
        frame.width() as usize,
        frame.height() as usize,
        frame.as_raw(),
    )
}

#[test]
fn the_image_moves_against_the_stage_by_its_move_in_pixels() {
    let blobs = Arc::new(BlobSpecimen::new(BlobConfig::default()));
    let um_per_px = Sensor::PREVIEW.pixel_pitch_um / 40.0;
    let shift = register(
        &frame(&blobs, 0.0, 0.0, 0),
        &frame(&blobs, 10.0, -3.3, 1),
        &Options::default(),
    );
    let (dx, dy) = (-10.0 / um_per_px, 3.3 / um_per_px);
    assert!(
        (shift.dx - dx).hypot(shift.dy - dy) < 0.5,
        "{shift:?} against ({dx}, {dy})"
    );
    assert!(shift.confidence > 0.5, "{shift:?}");
}

#[test]
fn an_empty_slide_gives_nothing_to_go_by() {
    let empty = Arc::new(BlobSpecimen::new(BlobConfig {
        density_per_mm2: 0.0,
        ..BlobConfig::default()
    }));
    let shift = register(
        &frame(&empty, 0.0, 0.0, 0),
        &frame(&empty, 10.0, 0.0, 1),
        &Options::default(),
    );
    assert!(shift.confidence < 0.3, "{shift:?}");
}
