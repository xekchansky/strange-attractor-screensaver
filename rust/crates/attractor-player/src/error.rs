//! Failures while building or advancing the player.

use std::error::Error;
use std::fmt;

use attractor_core::formula::FormulaError;
use attractor_core::{AttractorModelError, RandomizerError, SimulationHealthIssue};
use attractor_gpu::GpuError;

/// A failure while building or advancing the player.
#[derive(Debug)]
pub enum PlayerError {
    /// The selection's formulas cannot be bound to its parameters.
    Model(AttractorModelError),
    /// A formula failed to evaluate during a step.
    Integration(FormulaError),
    /// The health monitor rejected the run and healing is disabled.
    Unhealthy(SimulationHealthIssue),
    /// The catalog could not produce a replacement selection.
    Randomizer(RandomizerError),
    /// The GPU device or renderer failed.
    Gpu(GpuError),
}

impl fmt::Display for PlayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(formatter, "cannot build the attractor model: {error}"),
            Self::Integration(error) => write!(formatter, "simulation step failed: {error}"),
            Self::Unhealthy(issue) => {
                write!(formatter, "simulation health check failed: {issue:?}")
            }
            Self::Randomizer(error) => write!(formatter, "cannot select an attractor: {error}"),
            Self::Gpu(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for PlayerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Model(error) => Some(error),
            Self::Integration(error) => Some(error),
            Self::Unhealthy(_) => None,
            Self::Randomizer(error) => Some(error),
            Self::Gpu(error) => Some(error),
        }
    }
}

impl From<AttractorModelError> for PlayerError {
    fn from(error: AttractorModelError) -> Self {
        Self::Model(error)
    }
}

impl From<FormulaError> for PlayerError {
    fn from(error: FormulaError) -> Self {
        Self::Integration(error)
    }
}

impl From<RandomizerError> for PlayerError {
    fn from(error: RandomizerError) -> Self {
        Self::Randomizer(error)
    }
}

impl From<GpuError> for PlayerError {
    fn from(error: GpuError) -> Self {
        Self::Gpu(error)
    }
}
