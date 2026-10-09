//! Deterministic randomness, without a dependency: SplitMix64, as a hash of
//! several numbers (for the specimen, which must look the same wherever it
//! is rendered from) and as a sequence (for sensor noise).

/// SplitMix64's finaliser: a well-mixed 64-bit hash of `x`.
pub fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A hash of several numbers, in order.
pub fn hash(parts: &[u64]) -> u64 {
    parts.iter().fold(0, |acc, &part| mix(acc ^ part))
}

/// A number in [0, 1) from a hash.
pub fn unit(hash: u64) -> f64 {
    (hash >> 11) as f64 / (1u64 << 53) as f64
}

/// A sequence of pseudo-random numbers from a seed.
#[derive(Debug, Clone)]
pub struct Sequence(u64);

impl Sequence {
    /// The sequence for `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// The next number from a distribution close to the standard normal: the
    /// sum of four uniforms, which is quick and good enough for sensor noise.
    pub fn next_normal(&mut self) -> f32 {
        let bits = self.next_u64();
        let sum: u32 = (0..4).map(|i| ((bits >> (16 * i)) & 0xFFFF) as u32).sum();
        // Four uniforms on [0, 1) have a mean of 2 and a variance of 1/3.
        (sum as f32 / 65_536.0 - 2.0) * 3.0f32.sqrt()
    }
}
