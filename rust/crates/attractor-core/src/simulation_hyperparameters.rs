//! Declared ranges for the simulation and visualization hyperparameters.

use serde::{Deserialize, Serialize};

use crate::MinMax;

/// Ranges that govern the *simulation and visualization* rather than the
/// physical constants of the ODE (which [`Parameter`] models).
///
/// Every field is a `min`/`max` sampling range. At startup the randomizer
/// draws a concrete value from each range; the debug configuration can pin
/// them explicitly.
///
/// [`Parameter`]: crate::Parameter
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationHyperparameters {
    /// Integration time step (accuracy) of the ODE solver, per simulation tick.
    pub step_size: MinMax<f64>,
    /// Multiplier of simulated time per rendered frame (overall motion speed).
    pub simulation_speed: MinMax<f64>,
    /// Angular velocity of the orbiting camera, in radians per second.
    pub camera_rotation_speed: MinMax<f64>,
    /// How fast the trail's color gradient advances along the trace.
    pub trail_color_speed: MinMax<f64>,
    /// Number of points kept in the rendered trail buffer.
    pub trail_length: MinMax<i64>,
}

impl Default for SimulationHyperparameters {
    fn default() -> Self {
        Self {
            step_size: MinMax::new(0.004, 0.01),
            simulation_speed: MinMax::new(1.0, 3.0),
            camera_rotation_speed: MinMax::new(0.1, 0.4),
            trail_color_speed: MinMax::new(0.05, 0.3),
            trail_length: MinMax::new(2000, 8000),
        }
    }
}
