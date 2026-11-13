//! Random sources and range-drawing helpers.
//!
//! The helpers reproduce the Swift standard library's `random(in:using:)`
//! algorithms, so a seed selects the same values as the Swift oracle. They are
//! provided methods on [`RandomSource`] and only consume the source through
//! [`RandomSource::next_u64`], exactly like the generic Swift functions.

use std::ops::{Range, RangeInclusive};

/// A source of uniformly distributed 64-bit values.
///
/// Mirrors Swift's `RandomNumberGenerator`: implementors only provide
/// [`RandomSource::next_u64`], and every range helper is derived from it.
pub trait RandomSource {
    /// Produce the next uniformly distributed 64-bit value.
    fn next_u64(&mut self) -> u64;

    /// A uniformly distributed value in `0..upper_bound`.
    ///
    /// Uses Lemire's nearly divisionless method, the same algorithm as Swift's
    /// `RandomNumberGenerator.next(upperBound:)`.
    ///
    /// # Panics
    ///
    /// Panics when `upper_bound` is zero.
    fn next_below(&mut self, upper_bound: u64) -> u64 {
        assert!(upper_bound > 0, "upper bound must be positive");

        let mut random = self.next_u64();
        let mut product = u128::from(random) * u128::from(upper_bound);
        if (product as u64) < upper_bound {
            let threshold = upper_bound.wrapping_neg() % upper_bound;
            while (product as u64) < threshold {
                random = self.next_u64();
                product = u128::from(random) * u128::from(upper_bound);
            }
        }
        (product >> 64) as u64
    }

    /// A uniformly distributed index in `0..length`.
    ///
    /// # Panics
    ///
    /// Panics when `length` is zero.
    fn random_index(&mut self, length: usize) -> usize {
        assert!(length > 0, "upper bound must be positive");
        self.next_below(length as u64) as usize
    }

    /// A uniformly distributed integer in `range`.
    ///
    /// # Panics
    ///
    /// Panics when the range is empty.
    fn random_i64_in(&mut self, range: Range<i64>) -> i64 {
        assert!(range.start < range.end, "the range must not be empty");
        let width = (i128::from(range.end) - i128::from(range.start)) as u64;
        (i128::from(range.start) + i128::from(self.next_below(width))) as i64
    }

    /// A uniformly distributed integer in the closed `range`.
    ///
    /// # Panics
    ///
    /// Panics when the range is empty.
    fn random_i64_inclusive(&mut self, range: RangeInclusive<i64>) -> i64 {
        let (lower, upper) = range.into_inner();
        assert!(lower <= upper, "the range must not be empty");
        let width = (i128::from(upper) - i128::from(lower)) as u128 + 1;
        let draw = if width > u128::from(u64::MAX) {
            // The full 64-bit span has no representable width, so use every
            // bit of one draw, as Swift does for the equivalent edge case.
            self.next_u64()
        } else {
            self.next_below(width as u64)
        };
        (i128::from(lower) + i128::from(draw)) as i64
    }

    /// A uniformly distributed float in the closed `range`.
    ///
    /// Draws 53 bits of mantissa and scales them by the range width, matching
    /// Swift's `Double.random(in: ClosedRange<Double>, using:)`; the upper
    /// bound is reachable when the draw lands on the top of the range.
    ///
    /// # Panics
    ///
    /// Panics when the range is empty or infinite.
    fn random_f64_inclusive(&mut self, range: RangeInclusive<f64>) -> f64 {
        let (lower, upper) = range.into_inner();
        assert!(lower <= upper, "the range must not be empty");
        let delta = upper - lower;
        assert!(
            delta.is_finite(),
            "there is no uniform distribution on an infinite range"
        );

        const SIGNIFICAND_BITS: usize = 53;
        let draw = self.next_below((1 << SIGNIFICAND_BITS) + 1);
        if draw == 1 << SIGNIFICAND_BITS {
            return upper;
        }
        // `ulpOfOne / 2` is 2^-53, which turns the 53-bit draw into a value in
        // `[0, 1)`; `f64::EPSILON` is exactly `ulpOfOne`.
        let unit_random = draw as f64 * (f64::EPSILON / 2.0);
        delta * unit_random + lower
    }
}
