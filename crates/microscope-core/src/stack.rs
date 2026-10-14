//! Z-stacks (P36): whether a stack's latest images have found focus, by
//! their sharpness, and which of them to keep. The behaviour spec opens the
//! P36 notes.

use std::ops::Range;

/// The fewest images a window can judge.
pub const MIN_TEST: u32 = 3;
/// The most images a window can judge.
pub const MAX_TEST: u32 = 9;
/// How many images in from either end of the window the peak must be.
const EDGE: usize = 2;
/// How much sharper than a window's least sharp image its sharpest must be,
/// as a share; less is noise. On the simulator, frames far from focus vary
/// by up to 1.2%, and a window about focus rises by 16% or more.
const MIN_RISE: f64 = 0.03;

/// How a window of images was judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The image at this index of the window is in focus.
    Focused(usize),
    /// The peak is above: take another image, higher up.
    Continue,
    /// The peak is below: start again.
    Restart,
}

/// A stack's counts, odd and consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// How many images a window judges.
    pub test: usize,
    /// How many images are saved.
    pub save: usize,
}

impl Counts {
    /// `test` within 3 to 9, and `save` at least 1 and no more than `test`,
    /// each raised by one if it's even.
    pub fn new(test: u32, save: u32) -> Counts {
        let odd = |n: u32| n + u32::from(n.is_multiple_of(2));
        let test = odd(test.clamp(MIN_TEST, MAX_TEST));
        Counts {
            test: test as usize,
            save: odd(save.max(1)).min(test) as usize,
        }
    }

    /// How many images are saved either side of the sharpest.
    pub fn each_side(self) -> usize {
        (self.save - 1) / 2
    }

    /// Which of a stack's `len` images to save, the sharpest being at
    /// `sharpest`: as many as there are, centred on it.
    pub fn to_save(self, sharpest: usize, len: usize) -> Range<usize> {
        let side = self.each_side();
        sharpest.saturating_sub(side)..(sharpest + side + 1).min(len)
    }
}

/// The index of the sharpest image, the lowest of any tied; 0 if none.
pub fn sharpest(sharpness: &[f64]) -> usize {
    (0..sharpness.len())
        .max_by(|&a, &b| sharpness[a].total_cmp(&sharpness[b]).then(b.cmp(&a)))
        .unwrap_or(0)
}

/// Judges a window's sharpnesses, lowest image first.
pub fn judge(sharpness: &[f64]) -> Verdict {
    let (n, sharpest) = (sharpness.len(), sharpest(sharpness));
    if n < 2 {
        return Verdict::Focused(0);
    }
    let least = sharpness.iter().copied().fold(f64::INFINITY, f64::min);
    if sharpness[sharpest] < least * (1.0 + MIN_RISE) {
        return Verdict::Continue;
    }
    if n == 3 {
        return match sharpest {
            1 => Verdict::Focused(1),
            0 => Verdict::Restart,
            _ => Verdict::Continue,
        };
    }
    if sharpness.windows(2).all(|pair| pair[0] <= pair[1]) {
        return Verdict::Continue;
    }
    if sharpness.windows(2).all(|pair| pair[0] >= pair[1]) {
        return Verdict::Restart;
    }
    let Some(peak) = peak(sharpness) else {
        return Verdict::Continue;
    };
    if peak < EDGE as f64 - 0.5 || sharpest < EDGE {
        return Verdict::Restart;
    }
    if peak > (n - EDGE) as f64 - 0.5 || sharpest >= n - EDGE {
        return Verdict::Continue;
    }
    if turning_points(sharpness) > 1 {
        return Verdict::Continue;
    }
    Verdict::Focused(sharpest)
}

/// Where a parabola fitted to the sharpnesses peaks, by index, if it curves
/// downwards.
fn peak(y: &[f64]) -> Option<f64> {
    // The normal equations of y ≈ a·x² + b·x + c, at x = 0, 1, …
    let mut xtx = [[0.0; 3]; 3];
    let mut xty = [0.0; 3];
    for (i, &yi) in y.iter().enumerate() {
        let row = [(i * i) as f64, i as f64, 1.0];
        for j in 0..3 {
            xty[j] += row[j] * yi;
            for k in 0..3 {
                xtx[j][k] += row[j] * row[k];
            }
        }
    }
    let inverse = invert3(&xtx)?;
    let coefficient = |j: usize| (0..3).map(|k| inverse[j][k] * xty[k]).sum::<f64>();
    let (a, b) = (coefficient(0), coefficient(1));
    (a < 0.0).then(|| -b / (2.0 * a))
}

/// The inverse of a 3 × 3 matrix, if it has one.
fn invert3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
        let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let determinant: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
    if determinant.abs() < 1e-12 {
        return None;
    }
    // The adjugate is the transposed cofactors.
    Some(std::array::from_fn(|r| {
        std::array::from_fn(|c| cofactor(c, r) / determinant)
    }))
}

/// How often the sharpnesses turn, counting only steps more than half the
/// mean step, so noise doesn't count.
fn turning_points(y: &[f64]) -> usize {
    let steps: Vec<f64> = y.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let mean = steps.iter().map(|d| d.abs()).sum::<f64>() / steps.len().max(1) as f64;
    if mean == 0.0 {
        return 0;
    }
    let prominent: Vec<f64> = steps.into_iter().filter(|d| d.abs() / mean > 0.5).collect();
    prominent
        .windows(2)
        .filter(|pair| pair[0] * pair[1] < 0.0)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parabola_peaks_where_it_should_and_a_valley_has_no_peak() {
        let y: Vec<f64> = (0..9).map(|i| 100.0 - (i as f64 - 4.3).powi(2)).collect();
        assert!((peak(&y).expect("a peak") - 4.3).abs() < 1e-6);
        let valley: Vec<f64> = (0..9).map(|i| (i as f64 - 4.0).powi(2)).collect();
        assert_eq!(peak(&valley), None);
    }

    #[test]
    fn only_prominent_turns_count() {
        assert_eq!(turning_points(&[1.0, 2.0, 3.0, 2.0, 1.0]), 1);
        assert_eq!(turning_points(&[1.0, 3.0, 1.0, 3.0, 1.0]), 3);
        // A tiny wobble on the way up doesn't count.
        assert_eq!(turning_points(&[1.0, 2.0, 1.99, 3.0, 4.0, 3.0]), 1);
        assert_eq!(turning_points(&[2.0, 2.0, 2.0]), 0);
    }
}
