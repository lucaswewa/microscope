//! Draws frames for review, as PNGs, and how long each took.
//!
//! ```text
//! cargo run --release -p microscope-sim --example render -- [folder]
//! ```
//!
//! The folder defaults to `target/sim-review`. The frames: every objective
//! in focus; 40× defocused by 0 to 40 µm; the edge of the finite sample; a
//! repeating sample; and sensor noise.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use microscope_sim::{
    BlobConfig, BlobSpecimen, Extent, OBJECTIVES, Objective, Sensor, SimState, render_frame,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/sim-review"), PathBuf::from);
    std::fs::create_dir_all(&folder)?;

    let blobs = Arc::new(BlobSpecimen::new(BlobConfig::default()));
    let forty = Objective::with_magnification(40.0).expect("40× is offered");
    let base = SimState {
        specimen: blobs.clone(),
        objective: forty,
        sensor: Sensor::PREVIEW,
        stage_um: [0.0, 0.0, 0.0],
        focus_z_um: 0.0,
        noise: 0.0,
        frame: 0,
    };

    let mut frames: Vec<(String, SimState)> = OBJECTIVES
        .iter()
        .map(|&objective| {
            let name = format!("objective-{}x", objective.magnification);
            (
                name,
                SimState {
                    objective,
                    ..base.clone()
                },
            )
        })
        .collect();
    for dz in [0.0, 2.0, 5.0, 10.0, 20.0, 40.0] {
        let state = SimState {
            stage_um: [0.0, 0.0, dz],
            ..base.clone()
        };
        frames.push((format!("defocus-40x-{dz}um"), state));
    }
    let ten = Objective::with_magnification(10.0).expect("10× is offered");
    frames.push((
        "edge-10x".into(),
        SimState {
            objective: ten,
            stage_um: [10_000.0, 0.0, 0.0],
            ..base.clone()
        },
    ));
    let repeating = BlobSpecimen::new(BlobConfig {
        extent: Extent::Repeating { period_um: 200.0 },
        ..BlobConfig::default()
    });
    frames.push((
        "repeating-10x".into(),
        SimState {
            specimen: Arc::new(repeating),
            objective: ten,
            ..base.clone()
        },
    ));
    frames.push((
        "noise-40x".into(),
        SimState {
            noise: 6.0,
            frame: 1,
            ..base.clone()
        },
    ));

    for (name, state) in &frames {
        let start = Instant::now();
        let frame = render_frame(state);
        let elapsed = start.elapsed();
        let path = folder.join(format!("{name}.png"));
        frame.save(&path)?;
        println!(
            "{name:>22}: {:6.1} ms  {}",
            elapsed.as_secs_f64() * 1e3,
            path.display()
        );
    }
    Ok(())
}
