//! The deterministic, seedable pseudo-random number generator.

use crate::random::RandomSource;

/// A small, deterministic, seedable pseudo-random number generator using the
/// SplitMix64 algorithm.
///
/// Two instances created with the same seed produce the exact same sequence,
/// which makes reproducible selections and seeded simulations possible. The
/// sequence is bit-identical to the Swift oracle's `SplitMix64`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create a generator from a 64-bit seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Current internal state of the generator.
    pub const fn state(&self) -> u64 {
        self.state
    }
}

impl RandomSource for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
