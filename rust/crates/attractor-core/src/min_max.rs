//! A closed numeric range expressed declaratively as `min`/`max`.

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

/// A closed numeric range stored as `min`/`max`, so it can live in the JSON
/// catalog just like any other configuration value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinMax<T> {
    /// Lower bound (inclusive).
    pub min: T,
    /// Upper bound (inclusive).
    pub max: T,
}

impl<T> MinMax<T> {
    /// Build a range from its two bounds.
    pub const fn new(min: T, max: T) -> Self {
        Self { min, max }
    }
}

impl<T: Copy> MinMax<T> {
    /// The range as a native [`RangeInclusive`].
    pub fn closed_range(&self) -> RangeInclusive<T> {
        self.min..=self.max
    }
}
