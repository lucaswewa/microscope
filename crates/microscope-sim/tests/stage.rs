//! The stage model: timing against a manual clock, the latest command
//! winning, the zero offset, and property tests of backlash and travel
//! limits over random sequences of moves (seeded, so every run is the same).

use std::time::Duration;

use microscope_sim::clock::{Clock, ManualClock};
use microscope_sim::random::{Sequence, unit};
use microscope_sim::{MoveError, StageConfig, StageModel};

const SECOND: Duration = Duration::from_secs(1);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn moves_take_their_distance_over_the_speed_in_a_straight_line() {
    let clock = ManualClock::new();
    let mut stage = StageModel::new(StageConfig::default()); // 1,000 steps/s in x and y, 500 in z
    stage.move_to(clock.now(), [1000, 500, 250]);
    // Every axis needs 1 s, so they arrive together.
    assert_eq!(stage.arrival(), Some(SECOND));
    clock.advance(ms(500));
    assert_eq!(stage.reported(clock.now()), [500, 250, 125]);
    assert!(stage.is_moving(clock.now()));
    clock.advance(ms(500));
    assert!(!stage.is_moving(clock.now()));
    assert_eq!(stage.reported(clock.now()), [1000, 500, 250]);
    assert_eq!(stage.outcome(clock.now()), Ok(()));

    // A move limited by z takes z's time, and x keeps to the line.
    stage.move_to(clock.now(), [1500, 500, 750]);
    assert_eq!(stage.arrival(), Some(SECOND * 2));
    assert_eq!(stage.reported(SECOND + ms(500)), [1250, 500, 500]);
}

#[test]
fn the_latest_command_wins() {
    let mut stage = StageModel::new(StageConfig::default());
    stage.move_to(Duration::ZERO, [1000, 0, 0]);
    // A new move replaces it, from wherever the stage is.
    stage.move_to(ms(250), [0, 0, 0]);
    assert_eq!(stage.reported(ms(250)), [250, 0, 0]);
    assert_eq!(stage.arrival(), Some(ms(500)));
    // A relative move is from where the stage is now, not from the old target.
    stage.move_by(ms(400), [100, 0, 0]);
    assert_eq!(stage.reported(ms(400)), [100, 0, 0]);
    assert_eq!(stage.arrival(), Some(ms(500)));
    assert_eq!(stage.reported(SECOND), [200, 0, 0]);
}

#[test]
fn stopping_ends_a_move_on_whole_steps() {
    let mut stage = StageModel::new(StageConfig::default());
    stage.move_to(Duration::ZERO, [1000, 0, 0]);
    stage.stop(Duration::from_micros(333_700)); // 333.7 steps out
    assert!(!stage.is_moving(SECOND));
    assert_eq!(stage.reported(SECOND), [334, 0, 0]);
    assert_eq!(stage.true_position(SECOND), [334.0, 0.0, 0.0]);
}

#[test]
fn the_zero_offset_changes_what_is_reported_not_where_the_stage_is() {
    let mut stage = StageModel::new(StageConfig::default());
    stage.move_to(Duration::ZERO, [300, -200, 50]);
    stage.set_zero(SECOND);
    assert_eq!(stage.reported(SECOND), [0, 0, 0]);
    assert_eq!(stage.true_position(SECOND), [300.0, -200.0, 50.0]);
    stage.move_to(SECOND, [100, 0, 0]);
    assert_eq!(stage.reported(SECOND * 2), [100, 0, 0]);
    assert_eq!(stage.true_position(SECOND * 2), [400.0, -200.0, 50.0]);
}

/// Random moves: a target per axis in ±5,000 steps, sometimes interrupted.
fn random_moves(seed: u64, count: usize) -> Vec<([i64; 3], Option<f64>)> {
    let mut random = Sequence::new(seed);
    (0..count)
        .map(|_| {
            let target =
                std::array::from_fn(|_| (unit(random.next_u64()) * 10_000.0 - 5000.0) as i64);
            let interrupted = (unit(random.next_u64()) < 0.3).then(|| unit(random.next_u64()));
            (target, interrupted)
        })
        .collect()
}

/// Runs the moves, each to its end or interrupted part-way, calling
/// `check` after each with the stage, the time and the move.
fn run(
    stage: &mut StageModel,
    moves: &[([i64; 3], Option<f64>)],
    mut check: impl FnMut(&mut StageModel, Duration, [i64; 3]),
) {
    let mut now = Duration::ZERO;
    for &(target, interrupted) in moves {
        stage.move_to(now, target);
        let end = stage.arrival().expect("a move");
        now = match interrupted {
            Some(share) => now + (end - now).mul_f64(share),
            None => end,
        };
        check(stage, now, target);
        now += ms(10);
    }
}

#[test]
fn the_carriage_stays_within_half_the_backlash_of_the_motors() {
    for seed in 0..50 {
        let backlash = std::array::from_fn(|axis| unit(seed * 3 + axis as u64 + 1) * 100.0);
        let mut stage = StageModel::new(StageConfig {
            backlash,
            ..StageConfig::default()
        });
        run(&mut stage, &random_moves(seed, 40), |stage, now, _| {
            let reported = stage.reported(now);
            let true_position = stage.true_position(now);
            for axis in 0..3 {
                let gap = (true_position[axis] - reported[axis] as f64).abs();
                // The reported position is rounded to a step.
                assert!(
                    gap <= backlash[axis] / 2.0 + 0.5 + 1e-9,
                    "seed {seed}, axis {axis}: {gap}"
                );
            }
        });
    }
}

#[test]
fn backlash_shows_as_hysteresis() {
    for backlash in [0.0, 10.0, 37.0, 80.0] {
        let config = StageConfig {
            backlash: [backlash, 0.0, 0.0],
            ..StageConfig::default()
        };
        // The same target, approached from below and from above.
        let mut from_below = StageModel::new(config.clone());
        from_below.move_to(Duration::ZERO, [-1000, 0, 0]);
        from_below.move_to(SECOND * 2, [500, 0, 0]);
        let mut from_above = StageModel::new(config);
        from_above.move_to(Duration::ZERO, [2000, 0, 0]);
        from_above.move_to(SECOND * 3, [500, 0, 0]);
        let later = SECOND * 10;
        assert_eq!(from_below.reported(later), from_above.reported(later));
        let difference = from_above.true_position(later)[0] - from_below.true_position(later)[0];
        assert!(
            (difference - backlash).abs() < 1e-9,
            "{backlash}: {difference}"
        );
        // Turning back by less than the play doesn't move the carriage.
        let before = from_below.true_position(later)[0];
        from_below.move_by(later, [-(backlash as i64) / 2, 0, 0]);
        assert!((from_below.true_position(later + SECOND)[0] - before).abs() < 1e-9);
    }
}

#[test]
fn without_backlash_the_carriage_is_where_the_motors_are() {
    let mut stage = StageModel::new(StageConfig::default());
    run(&mut stage, &random_moves(7, 60), |stage, now, _| {
        let reported = stage.reported(now).map(|steps| steps as f64);
        let true_position = stage.true_position(now);
        for axis in 0..3 {
            assert!((true_position[axis] - reported[axis]).abs() <= 0.5);
        }
    });
}

#[test]
fn the_stage_never_leaves_its_travel_and_says_when_it_hits_a_limit() {
    for seed in 0..50 {
        let limits = [Some((-3000, 2000)), Some((-1000, 4000)), None];
        let mut stage = StageModel::new(StageConfig {
            limits,
            ..StageConfig::default()
        });
        let mut random = Sequence::new(seed);
        let mut moves = random_moves(seed, 40);
        // Some moves run to the end, so their outcome is known.
        moves.iter_mut().for_each(|(_, interrupted)| {
            if unit(random.next_u64()) < 0.5 {
                *interrupted = None;
            }
        });
        run(&mut stage, &moves, |stage, now, target| {
            let reported = stage.reported(now);
            for (axis, limit) in limits.iter().enumerate() {
                if let Some((low, high)) = limit {
                    assert!(
                        (*low..=*high).contains(&reported[axis]),
                        "seed {seed}: {reported:?}"
                    );
                }
            }
            if stage.is_moving(now) {
                return;
            }
            let outside = (0..2).any(|axis| {
                let (low, high) = limits[axis].unwrap();
                !(low..=high).contains(&target[axis])
            });
            match stage.outcome(now) {
                Ok(()) => assert!(!outside, "seed {seed}: {target:?}"),
                Err(MoveError::LimitReached { axis }) => {
                    assert!(outside, "seed {seed}: {target:?}");
                    let (low, high) = limits[axis].unwrap();
                    assert!(
                        reported[axis] == low || reported[axis] == high,
                        "{reported:?}"
                    );
                }
            }
        });
    }
}

#[test]
fn imperfections_can_be_switched_at_runtime() {
    let mut stage = StageModel::new(StageConfig {
        backlash: [50.0, 0.0, 0.0],
        limits: [Some((-100, 100)), None, None],
        ..StageConfig::default()
    });
    stage.move_to(Duration::ZERO, [100, 0, 0]);
    assert_eq!(stage.true_position(SECOND)[0], 75.0);
    assert_eq!(stage.outcome(SECOND), Ok(()), "100 is within the travel");
    // Switching changes the next moves, and stops one in progress.
    stage.move_to(SECOND, [0, 0, 0]);
    stage.set_config(SECOND + ms(50), StageConfig::default());
    assert!(!stage.is_moving(SECOND + ms(50)));
    assert_eq!(stage.true_position(SECOND + ms(50)), [50.0, 0.0, 0.0]);
    stage.move_to(SECOND * 2, [500, 0, 0]);
    assert_eq!(stage.reported(SECOND * 3), [500, 0, 0]);
    assert_eq!(stage.outcome(SECOND * 3), Ok(()));
}
