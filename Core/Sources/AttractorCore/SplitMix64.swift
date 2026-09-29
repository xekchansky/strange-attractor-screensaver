import Foundation

/// A small, deterministic, seedable pseudo-random number generator using the
/// SplitMix64 algorithm.
///
/// Unlike `SystemRandomNumberGenerator`, two instances initialized with the
/// same seed produce the exact same sequence of values, which makes it
/// suitable for reproducible selections and for seeding simulations.
public struct SplitMix64: RandomNumberGenerator, Sendable {
    /// Current internal state of the generator.
    public private(set) var state: UInt64

    /// Create a generator from a 64-bit seed.
    public init(seed: UInt64) {
        self.state = seed
    }

    /// Produce the next pseudo-random 64-bit value.
    public mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}
