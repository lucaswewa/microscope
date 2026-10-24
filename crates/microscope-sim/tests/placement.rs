//! Correlation placement (P12) on a scan the simulator draws with a drifting
//! stage: where the stage says the images are is pixels out, and their
//! overlaps put them back within a pixel or two of where they really are.

use std::sync::Arc;
use std::time::Duration;

use microscope_core::placement::{PlacementOptions, place};
use microscope_core::registration::Gray;
use microscope_sim::{
    BlobConfig, BlobSpecimen, Effects, Objective, Sensor, SimState, StageConfig, StageModel,
    render_frame,
};

/// How far a step moves the stage, in µm: the simulated stage's default.
const UM_PER_STEP: f64 = 0.1;

/// The largest distance between `placed` and `truth`, in pixels, once their
/// common offset is taken out.
fn residual(placed: &[[f64; 2]], truth: &[[f64; 2]]) -> f64 {
    let n = placed.len() as f64;
    let mean = |axis: usize| {
        (placed.iter().zip(truth))
            .map(|(p, t)| p[axis] - t[axis])
            .sum::<f64>()
            / n
    };
    let (mx, my) = (mean(0), mean(1));
    (placed.iter().zip(truth))
        .map(|(p, t)| (p[0] - t[0] - mx).hypot(p[1] - t[1] - my))
        .fold(0.0, f64::max)
}

#[test]
fn a_drifting_stage_s_scan_is_placed_within_a_pixel_or_two() {
    // The default sample, at 20×, as a slide is scanned.
    let specimen = Arc::new(BlobSpecimen::new(BlobConfig::default()));
    let objective = Objective::with_magnification(20.0).expect("20× is offered");
    let sensor = Sensor::PREVIEW;
    let um_per_px = sensor.pixel_pitch_um / objective.magnification;
    let px = |steps: f64| steps * UM_PER_STEP / um_per_px;
    // 1% of the steps lost, and a creep of a few steps a second.
    let mut stage = StageModel::new(StageConfig {
        lost_steps: [0.01, 0.01, 0.0],
        creep: [3.0, -2.0, 0.0],
        ..StageConfig::default()
    });

    // A 4 × 3 snake of images overlapping by 35%, each taken half a second
    // after the stage arrives.
    let step = [sensor.width, sensor.height]
        .map(|side| (f64::from(side) * 0.65 * um_per_px / UM_PER_STEP).round() as i64);
    let (mut tiles, mut expected, mut truth) = (vec![], vec![], vec![]);
    let mut now = Duration::ZERO;
    for row in 0..3 {
        for column in 0..4 {
            let column = if row % 2 == 0 { column } else { 3 - column };
            stage.move_to(now, [column * step[0], row * step[1], 0]);
            now = stage.arrival().unwrap_or(now) + Duration::from_millis(500);
            let (reported, real) = (stage.reported(now), stage.true_position(now));
            let frame = render_frame(&SimState {
                specimen: specimen.clone(),
                objective,
                sensor,
                stage_um: [real[0] * UM_PER_STEP, real[1] * UM_PER_STEP, 0.0],
                focus_z_um: 0.0,
                noise: 2.0,
                frame: tiles.len() as u64,
                effects: Effects::default(),
            });
            let (width, height) = (frame.width() as usize, frame.height() as usize);
            tiles.push(Gray::from_rgb8(width, height, frame.as_raw()));
            expected.push([px(reported[0] as f64), px(reported[1] as f64)]);
            truth.push([px(real[0]), px(real[1])]);
        }
    }

    let stage_off = residual(&expected, &truth);
    assert!(stage_off > 10.0, "the stage is only {stage_off} pixels out");
    let placement = place(&tiles, &expected, &PlacementOptions::default());
    let placed_off = residual(&placement.origins, &truth);
    assert!(
        placed_off < 1.5,
        "placed within {placed_off} pixels, from {stage_off}: {:?}",
        placement.pairs
    );
}
