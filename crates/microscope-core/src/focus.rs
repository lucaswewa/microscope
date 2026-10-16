//! Where focus is, from a curve of sharpness against z (P54, ADR-0037).
//!
//! Near focus, sharpness has a narrow peak, about a micrometre wide at
//! 40×, on a broad hump whose sides are symmetric about it. A sweep's
//! frames come some micrometres apart, so they can miss the peak and make
//! a side of the hump the sharpest. The hump's centroid, above half its
//! height, is the focus between the samples, whether the peak was caught
//! or not. A curve that hardly rises has nothing in view to focus on.

/// How much sharper than the least sharp sample the sharpest must be, as a
/// share, for the curve to show focus. On the simulator, sweeps over
/// empty fields rose by 2.9% at most, and over anything else by 4.2% or
/// more.
pub const MIN_RISE: f64 = 0.03;

/// The z of the curve's focus: the centroid of its samples above half its
/// height, each weighted by how far above. None if it hardly rises, or has
/// no samples. The curve's samples are `(z, sharpness)`, in any order.
pub fn peak(curve: &[(f64, f64)]) -> Option<f64> {
    let low = curve.iter().map(|&(_, s)| s).fold(f64::INFINITY, f64::min);
    let high = curve
        .iter()
        .map(|&(_, s)| s)
        .fold(f64::NEG_INFINITY, f64::max);
    if curve.is_empty() || high < low * (1.0 + MIN_RISE) {
        return None;
    }
    let cut = (low + high) / 2.0;
    let above = curve.iter().filter(|&&(_, s)| s > cut);
    let (moment, weight) = above.fold((0.0, 0.0), |(m, w), &(z, s)| {
        (m + z * (s - cut), w + (s - cut))
    });
    Some(moment / weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The simulator's preview about focus, as measured with the stage
    /// still (P54): a ring of sides 130 steps out and a narrow peak at
    /// `focus`, on a level, sampled every `step` from `first`.
    fn curve(focus: f64, first: f64, step: f64) -> Vec<(f64, f64)> {
        (0..20)
            .map(|i| {
                let z = first + step * f64::from(i);
                let d = z - focus;
                let ring = 500.0 * (-((d.abs() - 130.0) / 80.0).powi(2)).exp();
                let spike = 700.0 * (-(d / 12.0).powi(2)).exp();
                (z, 4300.0 + ring + spike)
            })
            .collect()
    }

    #[test]
    fn finds_focus_between_samples() {
        let found = peak(&curve(37.0, -480.0, 50.0)).expect("a focus");
        assert!((found - 37.0).abs() < 5.0, "{found}");
    }

    #[test]
    fn finds_focus_where_the_samples_miss_the_peak() {
        // Samples 25 steps either side of focus, and every 50 beyond: the
        // sharpest are the ring's, about 125 steps away.
        let samples = curve(25.0, -450.0, 50.0);
        let sharpest = samples
            .iter()
            .copied()
            .fold((0.0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
        assert!((sharpest.0 - 25.0_f64).abs() > 75.0, "{sharpest:?}");
        let found = peak(&samples).expect("a focus");
        assert!((found - 25.0).abs() < 10.0, "{found}");
    }

    #[test]
    fn finds_nothing_on_a_flat_curve_or_none() {
        let flat: Vec<(f64, f64)> = (0..20)
            .map(|i| (f64::from(i) * 50.0, 2760.0 + f64::from(i % 3)))
            .collect();
        assert_eq!(peak(&flat), None);
        assert_eq!(peak(&[]), None);
    }

    #[test]
    fn takes_samples_in_any_order() {
        let mut samples = curve(-60.0, -500.0, 50.0);
        let in_order = peak(&samples).expect("a focus");
        samples.reverse();
        let reversed = peak(&samples).expect("a focus");
        assert!(
            (reversed - in_order).abs() < 1e-9,
            "{reversed} and {in_order}"
        );
    }
}
