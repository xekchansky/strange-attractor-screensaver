//! A complete, concrete set of choices for one simulation run.

use std::collections::HashMap;

use crate::attractor_model::AttractorModelError;
use crate::{AttractorDescriptor, AttractorModel, SimulationSettings, State};

/// A complete, concrete set of choices for one simulation run: the attractor
/// descriptor together with sampled parameter values and concrete
/// hyperparameters.
///
/// This is what [`crate::randomizer`] produces at startup (and the debug
/// configuration will build by hand): everything the engine needs to construct
/// and run a simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct AttractorSelection {
    /// The chosen attractor definition.
    pub descriptor: AttractorDescriptor,
    /// Concrete parameter values keyed by parameter name. Covers every
    /// parameter declared in [`Self::descriptor`].
    pub parameters: HashMap<String, f64>,
    /// Concrete simulation/visualization hyperparameters.
    pub settings: SimulationSettings,
    /// The concrete initial state of the simulation.
    pub initial_state: State,
}

impl AttractorSelection {
    /// Assemble a selection from its concrete parts.
    pub fn new(
        descriptor: AttractorDescriptor,
        parameters: HashMap<String, f64>,
        settings: SimulationSettings,
        initial_state: State,
    ) -> Self {
        Self {
            descriptor,
            parameters,
            settings,
            initial_state,
        }
    }

    /// Turn the selection into an evaluable attractor model.
    ///
    /// Fails when a formula cannot be parsed or a declared parameter is absent
    /// from [`Self::parameters`].
    pub fn build_model(&self) -> Result<AttractorModel, AttractorModelError> {
        AttractorModel::new(self.descriptor.clone(), self.parameters.clone())
    }
}
