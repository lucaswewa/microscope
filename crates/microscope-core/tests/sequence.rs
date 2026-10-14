//! Sequences (P41): the plan's time points, and when each runs, with a fake
//! clock: on time, a little late within its interval, or skipped once its
//! interval has passed.

use std::time::Duration;

use microscope_core::sequence::{Due, PlanError, SequencePlan, Step};

fn seconds(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

fn plan(interval: f64, duration: f64) -> SequencePlan {
    SequencePlan::timelapse(seconds(interval), seconds(duration), None)
}

/// Runs `plan` on a fake clock, its steps at each point taking `takes(k)`:
/// each point run, and when, and the points skipped.
fn run(plan: &SequencePlan, takes: impl Fn(u64) -> f64) -> (Vec<(u64, f64)>, u64) {
    let (mut now, mut next, mut skipped) = (Duration::ZERO, 0, 0);
    let mut ran = Vec::new();
    loop {
        match plan.due(now, next) {
            Due::Wait(wait) => now += wait,
            Due::Run {
                index,
                skipped: late,
            } => {
                ran.push((index, now.as_secs_f64()));
                skipped += late;
                now += seconds(takes(index));
                next = index + 1;
            }
            Due::Done { skipped: late } => return (ran, skipped + late),
        }
    }
}

#[test]
fn points_are_at_the_start_then_each_interval_within_the_duration() {
    assert_eq!(plan(10.0, 60.0).points(), 7);
    assert_eq!(plan(10.0, 65.0).points(), 7);
    assert_eq!(plan(10.0, 5.0).points(), 1);
    assert_eq!(plan(10.0, 0.0).points(), 1);
    // Tenths of a second, counted exactly.
    assert_eq!(plan(0.1, 1.0).points(), 11);
    assert_eq!(plan(10.0, 60.0).at(3), seconds(30.0));
}

#[test]
fn on_time_every_point_runs_at_its_time() {
    let (ran, skipped) = run(&plan(10.0, 30.0), |_| 2.0);
    assert_eq!(ran, [(0, 0.0), (1, 10.0), (2, 20.0), (3, 30.0)]);
    assert_eq!(skipped, 0);
    assert_eq!(
        plan(10.0, 30.0).due(seconds(13.0), 2),
        Due::Wait(seconds(7.0))
    );
}

#[test]
fn a_late_point_runs_within_its_interval_and_later_ones_are_skipped() {
    // Point 2 takes 25 s: point 3's interval passes, and point 4 starts
    // late, at 45 s, within its own.
    let (ran, skipped) = run(&plan(10.0, 60.0), |k| if k == 2 { 25.0 } else { 1.0 });
    assert_eq!(
        ran,
        [
            (0, 0.0),
            (1, 10.0),
            (2, 20.0),
            (4, 45.0),
            (5, 50.0),
            (6, 60.0)
        ]
    );
    assert_eq!(skipped, 1);
    // Points never run back to back to catch up.
    let (ran, skipped) = run(&plan(10.0, 60.0), |_| 15.0);
    let indices: Vec<u64> = ran.iter().map(|(k, _)| *k).collect();
    assert_eq!(indices, [0, 1, 3, 4, 6]);
    assert_eq!(skipped, 2);
}

#[test]
fn points_whose_intervals_pass_at_the_end_are_skipped() {
    // The first point takes past the end and the last point's interval.
    let (ran, skipped) = run(&plan(10.0, 20.0), |_| 35.0);
    assert_eq!(ran, [(0, 0.0)]);
    assert_eq!(skipped, 2);
}

#[test]
fn a_timelapse_autofocuses_first_if_asked() {
    let focused = SequencePlan::timelapse(seconds(1.0), seconds(1.0), Some(2000));
    assert_eq!(focused.steps, [Step::Autofocus { dz: 2000 }, Step::Capture]);
    assert_eq!(plan(1.0, 1.0).steps, [Step::Capture]);
}

#[test]
fn only_supported_plans_run() {
    assert_eq!(plan(1.0, 10.0).check(), Ok(()));
    assert_eq!(plan(0.0, 10.0).check(), Err(PlanError::NoInterval));
    let mut stack = plan(1.0, 10.0);
    stack.steps.push(Step::ZStack { images: 3, dz: 50 });
    assert_eq!(stack.check(), Err(PlanError::Unsupported));
    let mut positions = plan(1.0, 10.0);
    positions.positions.push([0, 0, 0]);
    assert_eq!(positions.check(), Err(PlanError::Unsupported));
}
