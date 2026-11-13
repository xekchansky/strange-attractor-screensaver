//! The deterministic RNG contract.
//!
//! SplitMix64 must reproduce the Swift oracle bit for bit, because a seeded
//! selection has to survive the migration unchanged. The oracle vectors were
//! produced with Swift 6.4.2 (`SplitMix64` from `Core`, standard-library
//! `Int.random`/`Double.random`); `tests/fixtures/README.md` explains how.

use attractor_core::{RandomSource, SplitMix64};

#[test]
fn same_seed_produces_the_same_sequence() {
    let mut a = SplitMix64::new(42);
    let mut b = SplitMix64::new(42);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn different_seeds_produce_different_sequences() {
    let mut a = SplitMix64::new(1);
    let mut b = SplitMix64::new(2);
    let sequence_a: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
    let sequence_b: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
    assert_ne!(sequence_a, sequence_b);
}

#[test]
fn zero_seed_is_allowed_and_deterministic() {
    let mut a = SplitMix64::new(0);
    let mut b = SplitMix64::new(0);
    assert_eq!(a.next_u64(), b.next_u64());
}

#[test]
fn exposes_the_current_state() {
    let mut rng = SplitMix64::new(0);
    assert_eq!(rng.state(), 0);
    rng.next_u64();
    assert_eq!(rng.state(), 0x9E37_79B9_7F4A_7C15);
}

#[test]
fn sequence_matches_the_swift_oracle() {
    let mut seeded_42 = SplitMix64::new(42);
    let from_42: Vec<u64> = (0..5).map(|_| seeded_42.next_u64()).collect();
    assert_eq!(
        from_42,
        [
            0xbdd732262feb6e95,
            0x28efe333b266f103,
            0x47526757130f9f52,
            0x581ce1ff0e4ae394,
            0x09bc585a244823f2,
        ]
    );

    let mut seeded_0 = SplitMix64::new(0);
    let from_0: Vec<u64> = (0..3).map(|_| seeded_0.next_u64()).collect();
    assert_eq!(
        from_0,
        [0xe220a8397b1dcdaf, 0x6e789e6aa1b965f4, 0x06c45d188009454f,]
    );
}

#[test]
fn next_below_matches_the_swift_oracle() {
    let mut rng = SplitMix64::new(42);
    let draws: Vec<u64> = (0..5).map(|_| rng.next_below(12)).collect();
    assert_eq!(draws, [8, 1, 3, 4, 0]);
}

#[test]
fn integer_draws_match_the_swift_oracle() {
    let mut inclusive = SplitMix64::new(7);
    let draws: Vec<i64> = (0..5)
        .map(|_| inclusive.random_i64_inclusive(-3..=3))
        .collect();
    assert_eq!(draws, [-1, -3, 3, 1, 0]);

    let mut exclusive = SplitMix64::new(11);
    let picks: Vec<usize> = (0..8).map(|_| exclusive.random_index(4)).collect();
    assert_eq!(picks, [1, 1, 2, 2, 0, 2, 0, 3]);
}

#[test]
fn float_draws_match_the_swift_oracle() {
    let mut unit_range = SplitMix64::new(3);
    let draws: Vec<u64> = (0..5)
        .map(|_| unit_range.random_f64_inclusive(1.0..=2.0).to_bits())
        .collect();
    assert_eq!(
        draws,
        [
            0x3ff1d0b14e4db019,
            0x3ffb3466f8a7b81a,
            0x3ff9cebe8a6d050e,
            0x3ff12a764fb66abc,
            0x3ff37688dadcab7a,
        ]
    );

    let mut unit = SplitMix64::new(5);
    let draws: Vec<u64> = (0..5)
        .map(|_| unit.random_f64_inclusive(0.0..=1.0).to_bits())
        .collect();
    assert_eq!(
        draws,
        [
            0x3fd8c0cec328e270,
            0x3fe812e629b272e7,
            0x3fcdc969f80835e0,
            0x3fb96e4ec2da05b8,
            0x3fc80f13c7d500ac,
        ]
    );
}

#[test]
#[should_panic(expected = "upper bound must be positive")]
fn next_below_rejects_a_zero_bound() {
    let mut rng = SplitMix64::new(1);
    rng.next_below(0);
}
