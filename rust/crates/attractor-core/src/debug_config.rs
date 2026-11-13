//! Builds a concrete [`AttractorSelection`] by hand for debugging.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::{AttractorDescriptor, AttractorSelection, MinMax, SimulationSettings, State};

/// Error produced while building a debug configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugConfigError {
    /// No descriptor with the requested id exists in the catalog.
    UnknownAttractor(String),
}

impl fmt::Display for DebugConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAttractor(id) => write!(formatter, "unknown attractor '{id}'"),
        }
    }
}

impl Error for DebugConfigError {}

/// Builds a concrete [`AttractorSelection`] by hand for debugging.
///
/// Unlike [`crate::randomizer`] (which samples everything randomly), this lets
/// the demo app pin specific parameter and hyperparameter values and fills any
/// omitted value with the midpoint of its declared range.
pub struct DebugConfig;

impl DebugConfig {
    /// Small deterministic offset applied to the initial state so it never
    /// sits exactly on the range midpoint.
    ///
    /// For center-symmetric ranges (Thomas, Lorenz) the midpoint is an
    /// equilibrium point where every derivative is zero, so a start at the
    /// exact midpoint produces no motion at all.
    const INITIAL_STATE_OFFSET: State = State::new(0.01, -0.01, 0.01);

    /// Build a selection for the attractor with `id`, using midpoint
    /// parameters and midpoint hyperparameters.
    pub fn selection(
        id: &str,
        catalog: &[AttractorDescriptor],
    ) -> Result<AttractorSelection, DebugConfigError> {
        Self::selection_with(id, catalog, HashMap::new(), None)
    }

    /// Build a selection for the attractor with `id`, overriding some values.
    ///
    /// Explicit `parameters` win over the range midpoints; omitted parameters
    /// default to the midpoint of their declared range. When `settings` is
    /// `None`, every hyperparameter defaults to the midpoint of its range.
    pub fn selection_with(
        id: &str,
        catalog: &[AttractorDescriptor],
        mut parameters: HashMap<String, f64>,
        settings: Option<SimulationSettings>,
    ) -> Result<AttractorSelection, DebugConfigError> {
        let descriptor = catalog
            .iter()
            .find(|descriptor| descriptor.id == id)
            .ok_or_else(|| DebugConfigError::UnknownAttractor(id.to_string()))?;

        for parameter in &descriptor.parameters {
            parameters
                .entry(parameter.name.clone())
                .or_insert_with(|| (parameter.min + parameter.max) / 2.0);
        }

        let settings =
            settings.unwrap_or_else(|| SimulationSettings::midpoints_of(&descriptor.simulation));
        // The offset breaks the symmetry at the midpoint, but it must not push
        // a start outside a narrow declared range: clamp it back into bounds.
        let offset = Self::INITIAL_STATE_OFFSET;
        let initial_state = State::new(
            offset_within(
                midpoint(&descriptor.initial_state.x),
                offset.x,
                &descriptor.initial_state.x,
            ),
            offset_within(
                midpoint(&descriptor.initial_state.y),
                offset.y,
                &descriptor.initial_state.y,
            ),
            offset_within(
                midpoint(&descriptor.initial_state.z),
                offset.z,
                &descriptor.initial_state.z,
            ),
        );

        Ok(AttractorSelection::new(
            descriptor.clone(),
            parameters,
            settings,
            initial_state,
        ))
    }
}

fn midpoint(range: &MinMax<f64>) -> f64 {
    (range.min + range.max) / 2.0
}

/// Offset `value` by `delta` and clamp the result into `range`.
fn offset_within(value: f64, delta: f64, range: &MinMax<f64>) -> f64 {
    (value + delta).clamp(range.min, range.max)
}
