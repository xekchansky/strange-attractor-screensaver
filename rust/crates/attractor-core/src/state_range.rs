//! Sampling ranges for the coordinates of an initial state.

use serde::{Deserialize, Serialize};

use crate::MinMax;

/// A range for each coordinate of a simulation's initial state point.
///
/// Stored declaratively so the randomizer can draw a concrete [`State`] from
/// it at startup, and the debug configuration can pin one explicitly.
///
/// [`State`]: crate::State
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StateRange {
    /// Sampling range for the `x` coordinate.
    pub x: MinMax<f64>,
    /// Sampling range for the `y` coordinate.
    pub y: MinMax<f64>,
    /// Sampling range for the `z` coordinate.
    pub z: MinMax<f64>,
}

impl StateRange {
    /// Build a state range from the three coordinate ranges.
    pub const fn new(x: MinMax<f64>, y: MinMax<f64>, z: MinMax<f64>) -> Self {
        Self { x, y, z }
    }
}
