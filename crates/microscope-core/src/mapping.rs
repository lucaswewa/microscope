//! Camera–stage mapping's arithmetic (P23): 2 × 2 matrices, fitting one to
//! measured moves, and estimating backlash from a run of moves.

/// A 2 × 2 matrix, by rows.
pub type Matrix2 = [[f64; 2]; 2];

/// `m` times `v`.
pub fn apply(m: &Matrix2, v: [f64; 2]) -> [f64; 2] {
    [
        m[0][0] * v[0] + m[0][1] * v[1],
        m[1][0] * v[0] + m[1][1] * v[1],
    ]
}

/// The inverse of `m`, unless it is singular.
pub fn invert(m: &Matrix2) -> Option<Matrix2> {
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if det.abs() < 1e-12 {
        return None;
    }
    Some([
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ])
}

/// The matrix A that best maps each pair's `from` to its `to` (`to` ≈
/// A·`from`), by least squares. None if the `from`s don't span both
/// dimensions, or if the residuals' RMS length is more than `max_error`
/// times the `to`s' mean length.
pub fn fit(pairs: &[([f64; 2], [f64; 2])], max_error: f64) -> Option<Matrix2> {
    // A = (Σ to·fromᵀ)(Σ from·fromᵀ)⁻¹.
    let mut to_from = [[0.0; 2]; 2];
    let mut from_from = [[0.0; 2]; 2];
    for (from, to) in pairs {
        for i in 0..2 {
            for j in 0..2 {
                to_from[i][j] += to[i] * from[j];
                from_from[i][j] += from[i] * from[j];
            }
        }
    }
    let inverse = invert(&from_from)?;
    let a: Matrix2 = std::array::from_fn(|i| {
        std::array::from_fn(|j| to_from[i][0] * inverse[0][j] + to_from[i][1] * inverse[1][j])
    });
    let squares: f64 = pairs
        .iter()
        .map(|(from, to)| {
            let fitted = apply(&a, *from);
            (fitted[0] - to[0]).powi(2) + (fitted[1] - to[1]).powi(2)
        })
        .sum();
    let mean = pairs.iter().map(|(_, to)| to[0].hypot(to[1])).sum::<f64>() / pairs.len() as f64;
    ((squares / pairs.len() as f64).sqrt() <= max_error * mean).then_some(a)
}

/// What a run of equal moves, starting with a reversal, says about backlash.
#[derive(Debug, Clone, PartialEq)]
pub struct RunBacklash {
    /// How far the stage moved without the image following, in steps; None
    /// unless every move was tracked.
    pub steps: Option<f64>,
    /// Which moves fell short: the leading moves, up to the first tracked
    /// one that went at least 90% of the typical move.
    pub short: Vec<bool>,
}

/// The backlash in a run of moves of `step` steps each, from how far the
/// image moved on each (`moves`, in pixels; None where it couldn't be
/// tracked): the run's first moves move it less while the play is taken
/// up. The typical move is the median along the run's direction. The
/// leading moves' shortfall from it, in moves, times `step`, is the
/// backlash; once a move goes the typical distance, the play is taken up,
/// and later moves don't count.
pub fn run_backlash(moves: &[Option<[f64; 2]>], step: f64) -> RunBacklash {
    let tracked: Vec<[f64; 2]> = moves.iter().flatten().copied().collect();
    let direction = tracked
        .iter()
        .fold([0.0, 0.0], |sum, m| [sum[0] + m[0], sum[1] + m[1]]);
    let norm = direction[0].hypot(direction[1]).max(1e-12);
    let along = |m: &[f64; 2]| (m[0] * direction[0] + m[1] * direction[1]) / norm;
    let mut lengths: Vec<f64> = tracked.iter().map(|m| m[0].hypot(m[1])).collect();
    lengths.sort_by(f64::total_cmp);
    let typical = lengths.get(lengths.len() / 2).copied().unwrap_or(0.0);
    if typical <= 0.0 {
        return RunBacklash {
            steps: None,
            short: vec![false; moves.len()],
        };
    }
    let leading = moves
        .iter()
        .take_while(|m| m.is_none_or(|m| along(&m) < 0.9 * typical))
        .count();
    let shortfall: f64 = moves[..leading]
        .iter()
        .flatten()
        .map(|m| (typical - along(m)).max(0.0))
        .sum();
    RunBacklash {
        steps: (tracked.len() == moves.len()).then(|| shortfall / typical * step.abs()),
        short: (0..moves.len()).map(|i| i < leading).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_a_rotation_and_scale_exactly_and_inverts_it() {
        let truth: Matrix2 = [[1.2, -0.4], [0.3, 0.9]];
        let pairs: Vec<_> = [[10.0, 0.0], [0.0, 7.0], [-3.0, 5.0], [8.0, -2.0]]
            .into_iter()
            .map(|from| (from, apply(&truth, from)))
            .collect();
        let fitted = fit(&pairs, 1e-9).expect("spanned and exact");
        for i in 0..2 {
            for j in 0..2 {
                assert!((fitted[i][j] - truth[i][j]).abs() < 1e-12);
            }
        }
        // A pair far from the rest is more error than a fit allows.
        let mut stray = pairs.clone();
        stray.push(([5.0, 5.0], [-20.0, 30.0]));
        assert!(fit(&stray, 0.1).is_none() && fit(&stray, 10.0).is_some());
        let back = apply(
            &invert(&truth).expect("invertible"),
            apply(&truth, [2.0, -5.0]),
        );
        assert!((back[0] - 2.0).abs() < 1e-12 && (back[1] + 5.0).abs() < 1e-12);
        let unspanned = [([1.0, 0.0], [1.0, 1.0]), ([2.0, 0.0], [2.0, 2.0])];
        assert!(fit(&unspanned, 10.0).is_none());
        assert!(invert(&[[1.0, 2.0], [2.0, 4.0]]).is_none());
    }

    #[test]
    fn backlash_is_the_shortfall_of_a_runs_first_moves() {
        // Moves of 100 steps that move the image 50 px, the first only 20
        // and the second 40: 30 + 10 = 40 px short, or 80 steps.
        let moves = [
            Some([-20.0, 1.0]),
            Some([-40.0, 0.0]),
            Some([-50.0, 0.5]),
            Some([-50.0, -0.5]),
            Some([-50.0, 0.0]),
        ];
        let backlash = run_backlash(&moves, 100.0);
        assert!(
            (backlash.steps.expect("tracked") - 80.0).abs() < 0.5,
            "{backlash:?}"
        );
        assert_eq!(backlash.short, [true, true, false, false, false]);
        let even = run_backlash(&[Some([30.0, 0.0]); 4], -60.0);
        assert_eq!(even.steps, Some(0.0));
        // A stray short move later in the run isn't backlash.
        let stray = [50.0, 50.0, 10.0, 50.0].map(|x| Some([x, 0.0]));
        let stray = run_backlash(&stray, 100.0);
        assert_eq!((stray.steps, stray.short), (Some(0.0), vec![false; 4]));
        // An untracked move leaves the backlash unknown, and doesn't end
        // the leading moves.
        let untracked = run_backlash(
            &[
                None,
                Some([40.0, 0.0]),
                Some([50.0, 0.0]),
                Some([50.0, 0.0]),
            ],
            100.0,
        );
        assert_eq!(
            (untracked.steps, untracked.short),
            (None, vec![true, true, false, false])
        );
    }
}
