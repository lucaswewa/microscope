//! Moves a simulated stage and prints where it is over time: the motors,
//! the carriage (the true position) and the reported position, in steps.
//!
//! ```text
//! cargo run -p microscope-sim --example trajectory
//! ```
//!
//! The x axis has 40 steps of backlash and a limit at 3,000 steps. It moves
//! out to 2,000, with the carriage trailing the motors by half the backlash;
//! back to 0, where the carriage waits until the motors have taken up all 40
//! steps of play; then towards 5,000, stopping at its limit.

use std::time::Duration;

use microscope_sim::{StageConfig, StageModel};

fn main() {
    let mut stage = StageModel::new(StageConfig {
        backlash: [40.0, 0.0, 0.0],
        limits: [Some((-3000, 3000)), None, None],
        ..StageConfig::default()
    });
    let mut now = Duration::ZERO;
    println!(
        "{:>6}  {:>9}  {:>9}  {:>8}  move",
        "time", "reported", "carriage", "moving"
    );
    for (label, target) in [("out", 2000), ("back", 0), ("too far", 5000)] {
        stage.move_to(now, [target, 0, 0]);
        loop {
            let moving = stage.is_moving(now);
            println!(
                "{:>5.2}s  {:>9}  {:>9.1}  {:>8}  {label}",
                now.as_secs_f64(),
                stage.reported(now)[0],
                stage.true_position(now)[0],
                moving,
            );
            if !moving {
                break;
            }
            now += Duration::from_millis(250);
        }
        if let Err(error) = stage.outcome(now) {
            println!("        stopped: {error}");
        }
    }
}
