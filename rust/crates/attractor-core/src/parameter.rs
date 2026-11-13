//! Tunable coefficients of an attractor's ODE system.

use serde::{Deserialize, Serialize};

/// A tunable coefficient of an attractor's ODE system.
///
/// Each parameter is a named scalar that the formula strings reference as a
/// variable (e.g. `a`, `b`, `r`). At runtime the engine draws a concrete value
/// uniformly from the closed range `min..=max`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Variable name used inside the formula strings (e.g. `a`).
    pub name: String,
    /// Lower bound of the sampled range (inclusive).
    pub min: f64,
    /// Upper bound of the sampled range (inclusive).
    pub max: f64,
}

impl Parameter {
    /// Build a parameter from its name and closed range.
    pub fn new(name: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            name: name.into(),
            min,
            max,
        }
    }
}
