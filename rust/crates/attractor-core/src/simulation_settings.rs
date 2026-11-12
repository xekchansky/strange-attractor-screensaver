//! Concrete, sampled values of the simulation hyperparameters.

use crate::{SimulationHyperparameters, TraceColorMode};

/// Concrete, sampled values of the simulation and visualization
/// hyperparameters.
///
/// This is the "frozen" counterpart of [`SimulationHyperparameters`], which
/// holds ranges: each field is a single value drawn from its range by the
/// randomizer or pinned explicitly by the debug configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationSettings {
    /// Integration time step (accuracy) of the ODE solver, per simulation tick.
    pub step_size: f64,
    /// Multiplier of simulated time per rendered frame.
    pub simulation_speed: f64,
    /// Angular velocity of the orbiting camera, in radians per second.
    pub camera_rotation_speed: f64,
    /// How fast the trail's color gradient advances.
    pub trail_color_speed: f64,
    /// Number of points kept in the rendered trail buffer.
    pub trail_length: i64,
    /// How the trail is colored.
    pub color_mode: TraceColorMode,
}

impl SimulationSettings {
    /// Build concrete settings from explicit values.
    pub const fn new(
        step_size: f64,
        simulation_speed: f64,
        camera_rotation_speed: f64,
        trail_color_speed: f64,
        trail_length: i64,
        color_mode: TraceColorMode,
    ) -> Self {
        Self {
            step_size,
            simulation_speed,
            camera_rotation_speed,
            trail_color_speed,
            trail_length,
            color_mode,
        }
    }

    /// Sample the midpoint of every range in `hyperparameters`.
    ///
    /// Used by the debug configuration as the default when no explicit
    /// settings are supplied.
    pub fn midpoints_of(hyperparameters: &SimulationHyperparameters) -> Self {
        Self {
            step_size: midpoint(hyperparameters.step_size.min, hyperparameters.step_size.max),
            simulation_speed: midpoint(
                hyperparameters.simulation_speed.min,
                hyperparameters.simulation_speed.max,
            ),
            camera_rotation_speed: midpoint(
                hyperparameters.camera_rotation_speed.min,
                hyperparameters.camera_rotation_speed.max,
            ),
            trail_color_speed: midpoint(
                hyperparameters.trail_color_speed.min,
                hyperparameters.trail_color_speed.max,
            ),
            trail_length: (hyperparameters.trail_length.min + hyperparameters.trail_length.max) / 2,
            color_mode: TraceColorMode::Trail,
        }
    }

    /// Problems that would make the renderer misbehave.
    ///
    /// Used by the demo to reject nonsensical command-line overrides before
    /// they produce a frozen trace (non-positive step), an empty polyline
    /// (trail shorter than two points) or non-finite camera/color motion.
    /// A zero speed is allowed: it intentionally freezes one aspect for
    /// debugging.
    pub fn validation_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if !self.step_size.is_finite() || self.step_size <= 0.0 {
            issues.push(format!(
                "stepSize must be positive and finite (got {})",
                self.step_size
            ));
        }
        if !self.simulation_speed.is_finite() || self.simulation_speed < 0.0 {
            issues.push(format!(
                "simulationSpeed must be non-negative and finite (got {})",
                self.simulation_speed
            ));
        }
        if !self.camera_rotation_speed.is_finite() || self.camera_rotation_speed < 0.0 {
            issues.push(format!(
                "cameraRotationSpeed must be non-negative and finite (got {})",
                self.camera_rotation_speed
            ));
        }
        if !self.trail_color_speed.is_finite() || self.trail_color_speed < 0.0 {
            issues.push(format!(
                "trailColorSpeed must be non-negative and finite (got {})",
                self.trail_color_speed
            ));
        }
        if self.trail_length < 2 {
            issues.push(format!(
                "trailLength must keep at least 2 points (got {})",
                self.trail_length
            ));
        }

        issues
    }
}

fn midpoint(min: f64, max: f64) -> f64 {
    (min + max) / 2.0
}
