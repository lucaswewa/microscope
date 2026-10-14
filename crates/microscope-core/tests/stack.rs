//! Z-stacks (P36): the counts made odd and consistent, the images saved
//! about the sharpest, and how a window is judged, by each rule, along focus
//! curves measured on the simulator, and on its noise.

use microscope_core::stack::{Counts, Verdict, judge};

/// The simulator's sharpness, its preview frames' JPEG sizes, every 50 steps
/// from 500 below focus to 500 above, on a dense sample of blobs.
const DENSE: [f64; 21] = [
    5750.0, 5738.0, 5724.0, 5749.0, 6291.0, 7279.0, 8691.0, 10230.0, 11892.0, 14051.0, 20394.0,
    14074.0, 11914.0, 10246.0, 8682.0, 7288.0, 6298.0, 5726.0, 5724.0, 5726.0, 5763.0,
];
/// The same, from 700 below focus to 400 above, on the default sample.
const DEFAULT: [f64; 23] = [
    4556.0, 4552.0, 4567.0, 4541.0, 4545.0, 4565.0, 4552.0, 4557.0, 4810.0, 5220.0, 5699.0, 6201.0,
    6471.0, 6604.0, 7699.0, 6616.0, 6468.0, 6187.0, 5710.0, 5228.0, 4809.0, 4553.0, 4546.0,
];

#[test]
fn counts_are_odd_and_no_more_are_saved_than_tested() {
    let counts = |test, save| {
        let Counts { test, save } = Counts::new(test, save);
        (test, save)
    };
    assert_eq!(counts(9, 1), (9, 1));
    assert_eq!(counts(8, 4), (9, 5));
    assert_eq!(counts(5, 7), (5, 5));
    assert_eq!(counts(6, 9), (7, 7));
    // Within 3 to 9 tested, and at least 1 saved.
    assert_eq!(counts(1, 0), (3, 1));
    assert_eq!(counts(12, 12), (9, 9));
}

#[test]
fn the_images_saved_are_centred_on_the_sharpest() {
    let five = Counts::new(9, 5);
    assert_eq!(five.each_side(), 2);
    assert_eq!(five.to_save(6, 13), 4..9);
    // Fewer, where the stack is too short.
    assert_eq!(five.to_save(1, 13), 0..4);
    assert_eq!(five.to_save(11, 13), 9..13);
    assert_eq!(Counts::new(9, 1).to_save(3, 9), 3..4);
}

#[test]
fn three_images_succeed_only_with_the_middle_sharpest() {
    assert_eq!(judge(&[1.0, 3.0, 2.0]), Verdict::Focused(1));
    assert_eq!(judge(&[3.0, 2.0, 1.0]), Verdict::Restart);
    assert_eq!(judge(&[1.0, 2.0, 3.0]), Verdict::Continue);
    // A tie goes to the lower image.
    assert_eq!(judge(&[3.0, 3.0, 1.0]), Verdict::Restart);
}

#[test]
fn a_window_climbing_or_falling_throughout_has_its_peak_beyond_it() {
    assert_eq!(judge(&[1.0, 2.0, 2.0, 4.0, 5.0]), Verdict::Continue);
    assert_eq!(judge(&[5.0, 4.0, 4.0, 2.0, 1.0]), Verdict::Restart);
    // Falling ever more slowly, so no parabola peaks.
    assert_eq!(judge(&[9.0, 5.0, 3.0, 2.0, 1.5]), Verdict::Restart);
    // Levelling off isn't a peak either: it may rise again above.
    let level = [0.0, 5.0, 8.0, 9.5, 10.0, 10.0, 10.0, 10.0, 10.0];
    assert_eq!(judge(&level), Verdict::Continue);
}

#[test]
fn a_peak_must_rise_enough_and_be_away_from_the_edges() {
    let dome = |centre: f64| -> Vec<f64> {
        (0..9)
            .map(|i| 100.0 - (i as f64 - centre).powi(2))
            .collect()
    };
    assert_eq!(judge(&dome(4.0)), Verdict::Focused(4));
    assert_eq!(judge(&dome(2.2)), Verdict::Focused(2));
    assert_eq!(judge(&dome(6.2)), Verdict::Focused(6));
    // Rising by less than 3% is noise.
    let faint: Vec<f64> = dome(4.0).iter().map(|y| y + 1000.0).collect();
    assert_eq!(judge(&faint), Verdict::Continue);
    // In the first two places, or the last two.
    assert_eq!(judge(&dome(1.2)), Verdict::Restart);
    assert_eq!(judge(&dome(6.8)), Verdict::Continue);
    // The fit peaks inside, but the sharpest image is at an edge.
    let mut late = dome(5.0);
    late[7] = 100.5;
    assert_eq!(judge(&late), Verdict::Continue);
    let mut early = dome(3.0);
    early[1] = 100.5;
    assert_eq!(judge(&early), Verdict::Restart);
    // A valley isn't a peak.
    let valley: Vec<f64> = (0..9).map(|i| (i as f64 - 4.0).powi(2)).collect();
    assert_eq!(judge(&valley), Verdict::Continue);
}

#[test]
fn two_peaks_are_no_focus_but_noise_is_ignored() {
    let twin = [1.0, 5.0, 3.0, 6.0, 9.0, 6.0, 3.0, 5.0, 1.0];
    assert_eq!(judge(&twin), Verdict::Continue);
    let wobble = [1.0, 4.0, 6.0, 8.0, 9.0, 8.02, 8.0, 6.0, 4.0];
    assert_eq!(judge(&wobble), Verdict::Focused(4));
}

#[test]
fn climbing_measured_curves_from_far_below_succeeds_first_at_focus() {
    for (curve, focus) in [(&DENSE[..], 10), (&DEFAULT[..], 14)] {
        for test in [3, 5, 7, 9] {
            let found = (0..=curve.len() - test).find_map(|start| {
                match judge(&curve[start..start + test]) {
                    Verdict::Focused(i) => Some(start + i),
                    Verdict::Continue => None,
                    Verdict::Restart => panic!("{test} images restarted at {start}"),
                }
            });
            assert_eq!(found, Some(focus), "{test} images");
        }
    }
}

#[test]
fn noise_far_from_focus_is_no_peak() {
    // Frames far from focus on the default sample, at one z. Some windows
    // have the middle image sharpest, under a parabola curving down.
    let noise = [
        4869.0, 4887.0, 4890.0, 4900.0, 4864.0, 4886.0, 4879.0, 4896.0, 4872.0, 4892.0, 4895.0,
        4880.0, 4867.0, 4903.0, 4891.0, 4880.0,
    ];
    for test in [3, 5, 7, 9] {
        for window in noise.windows(test) {
            assert_eq!(judge(window), Verdict::Continue, "{window:?}");
        }
    }
}
