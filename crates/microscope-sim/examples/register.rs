//! Measures how far the image moves between two rendered frames, by phase
//! correlation (ADR-0023), against how far the stage moved.
//!
//! ```text
//! cargo run --release -p microscope-sim --example register
//! ```

use std::sync::Arc;
use std::time::Instant;

use microscope_core::registration::{Gray, Options, register};
use microscope_sim::{
    BlobConfig, BlobSpecimen, Effects, Objective, Sensor, SimState, render_frame,
};

fn main() {
    let objective = Objective::with_magnification(40.0).expect("40× is offered");
    let state = SimState {
        specimen: Arc::new(BlobSpecimen::new(BlobConfig::default())),
        objective,
        sensor: Sensor::PREVIEW,
        stage_um: [0.0, 0.0, 0.0],
        focus_z_um: 0.0,
        noise: 2.0,
        frame: 0,
        effects: Effects::default(),
    };
    let um_per_px = Sensor::PREVIEW.pixel_pitch_um / objective.magnification;
    let gray = |state: &SimState| {
        let frame = render_frame(state);
        Gray::from_rgb8(
            frame.width() as usize,
            frame.height() as usize,
            frame.as_raw(),
        )
    };
    let before = gray(&state);
    println!("40×, {um_per_px:.3} µm a pixel; the image moves against the stage.");
    for (index, (x_um, y_um)) in [(10.0, 5.0), (-3.3, 7.25), (20.0, -12.0)]
        .into_iter()
        .enumerate()
    {
        let after = gray(&SimState {
            stage_um: [x_um, y_um, 0.0],
            frame: index as u64 + 1,
            ..state.clone()
        });
        let started = Instant::now();
        let shift = register(&before, &after, &Options::default());
        let took = started.elapsed();
        println!(
            "stage ({x_um:6.2}, {y_um:6.2}) µm: expected ({:7.2}, {:7.2}) px, measured ({:7.2}, {:7.2}) px, confidence {:.2}, in {:.1} ms",
            -x_um / um_per_px,
            -y_um / um_per_px,
            shift.dx,
            shift.dy,
            shift.confidence,
            took.as_secs_f64() * 1e3
        );
    }
}
