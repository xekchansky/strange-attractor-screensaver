//! Metadata and formula definitions for a single attractor.

use serde::{Deserialize, Serialize};

use crate::{Parameter, SimulationHyperparameters, StateRange};

/// Metadata and formula definitions for a single attractor, loaded from
/// `attractors.json`.
///
/// Formulas are plain strings parsed at runtime; their variables are the state
/// coordinates (`x`, `y`, `z`) and the declared [`Parameter`] names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttractorDescriptor {
    /// Stable unique identifier (e.g. `"lorenz"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Formula for dX/dt as a string.
    pub dx: String,
    /// Formula for dY/dt as a string.
    pub dy: String,
    /// Formula for dZ/dt as a string.
    pub dz: String,
    /// Tunable ODE coefficients together with their sampled ranges.
    pub parameters: Vec<Parameter>,
    /// Ranges governing the simulation and visualization.
    pub simulation: SimulationHyperparameters,
    /// Ranges for the initial state coordinates.
    pub initial_state: StateRange,
}
