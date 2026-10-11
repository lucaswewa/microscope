//! A radix-2 fast Fourier transform, for the power-of-two squares that
//! image registration compares (ADR-0023).

use std::f64::consts::PI;
use std::ops::{Add, Mul, Sub};

/// A complex number.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Complex {
    /// The real part.
    pub re: f64,
    /// The imaginary part.
    pub im: f64,
}

impl Complex {
    /// `re + i·im`.
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// The complex conjugate.
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    /// The magnitude.
    pub fn norm(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// Divided by a real number.
    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.re * factor, self.im * factor)
    }
}

impl Add for Complex {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }
}

/// Transforms `data` in place: forward, or with `inverse`, backward without
/// the 1/n scaling. Its length must be a power of two.
pub fn fft(data: &mut [Complex], inverse: bool) {
    let n = data.len();
    assert!(
        n.is_power_of_two(),
        "an FFT's length must be a power of two"
    );
    // Reorder by bit-reversed index, then combine pairs, quads, and so on.
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        for k in 0..half {
            let angle = sign * 2.0 * PI * k as f64 / len as f64;
            let twiddle = Complex::new(angle.cos(), angle.sin());
            for start in (0..n).step_by(len) {
                let even = data[start + k];
                let odd = data[start + k + half] * twiddle;
                data[start + k] = even + odd;
                data[start + k + half] = even - odd;
            }
        }
        len *= 2;
    }
}

/// Transforms an `n` × `n` grid, stored by rows, in place: each row, then
/// each column.
pub fn fft2(data: &mut [Complex], n: usize, inverse: bool) {
    assert_eq!(data.len(), n * n, "the grid must be n × n");
    for row in data.chunks_mut(n) {
        fft(row, inverse);
    }
    let mut column = vec![Complex::default(); n];
    for x in 0..n {
        for (y, value) in column.iter_mut().enumerate() {
            *value = data[y * n + x];
        }
        fft(&mut column, inverse);
        for (y, value) in column.iter().enumerate() {
            data[y * n + x] = *value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transform by its definition, for comparison.
    fn dft(data: &[Complex]) -> Vec<Complex> {
        let n = data.len();
        (0..n)
            .map(|k| {
                data.iter()
                    .enumerate()
                    .fold(Complex::default(), |sum, (t, value)| {
                        let angle = -2.0 * PI * (k * t) as f64 / n as f64;
                        sum + *value * Complex::new(angle.cos(), angle.sin())
                    })
            })
            .collect()
    }

    #[test]
    fn matches_the_definition_and_inverts() {
        let data: Vec<Complex> = (0..16)
            .map(|i| Complex::new((i * 7 % 5) as f64, (i % 3) as f64 - 1.0))
            .collect();
        let mut transformed = data.clone();
        fft(&mut transformed, false);
        for (fast, slow) in transformed.iter().zip(dft(&data)) {
            assert!((*fast - slow).norm() < 1e-9, "{fast:?} against {slow:?}");
        }
        fft(&mut transformed, true);
        for (back, original) in transformed.iter().zip(&data) {
            assert!((back.scale(1.0 / 16.0) - *original).norm() < 1e-9);
        }
    }

    #[test]
    fn transforms_a_grid_by_rows_and_columns() {
        // A single point at (x, y) = (1, 2) has a transform of unit
        // magnitude everywhere, with the phase of that shift.
        let n = 8;
        let mut grid = vec![Complex::default(); n * n];
        grid[2 * n + 1] = Complex::new(1.0, 0.0);
        fft2(&mut grid, n, false);
        let (kx, ky) = (3, 5);
        let (x, y) = (1, 2);
        let angle = -2.0 * PI * (kx * x + ky * y) as f64 / n as f64;
        let expected = Complex::new(angle.cos(), angle.sin());
        assert!((grid[ky * n + kx] - expected).norm() < 1e-12);
        assert!(grid.iter().all(|value| (value.norm() - 1.0).abs() < 1e-12));
    }
}
