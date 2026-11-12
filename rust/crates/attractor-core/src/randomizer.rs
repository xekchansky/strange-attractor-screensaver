//! Chooses a random attractor together with concrete parameter values and
//! hyperparameters for a simulation run.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::{
    AttractorDescriptor, AttractorSelection, RandomSource, SimulationHyperparameters,
    SimulationSettings, State, TraceColorMode,
};

/// Error thrown when a catalog or a descriptor cannot produce a run.
#[derive(Debug, Clone, PartialEq)]
pub enum RandomizerError {
    /// The catalog handed to the randomizer is empty.
    EmptyCatalog,
    /// A sampling range is malformed: non-finite bounds or `min > max`.
    InvalidRange {
        /// Name of the offending field, e.g. `"parameters.a"`.
        field: String,
        /// Declared lower bound.
        min: f64,
        /// Declared upper bound.
        max: f64,
    },
}

impl fmt::Display for RandomizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCatalog => write!(formatter, "the attractor catalog is empty"),
            Self::InvalidRange { field, min, max } => {
                write!(formatter, "invalid range for {field}: min {min}, max {max}")
            }
        }
    }
}

impl Error for RandomizerError {}

/// Pick a random attractor, parameter values, and hyperparameters from
/// `catalog`.
///
/// Parameter values are drawn uniformly from each declared `[min, max]` range;
/// hyperparameters are drawn from the descriptor's simulation ranges. The
/// sequence of draws matches the Swift oracle, so the same seed reproduces the
/// same selection bit for bit.
pub fn select_from_catalog<R: RandomSource>(
    catalog: &[AttractorDescriptor],
    rng: &mut R,
) -> Result<AttractorSelection, RandomizerError> {
    if catalog.is_empty() {
        return Err(RandomizerError::EmptyCatalog);
    }
    let index = rng.random_index(catalog.len());
    select_attractor(&catalog[index], rng)
}

/// Pick parameter values, hyperparameters, and an initial state for a *fixed*
/// attractor, sampling everything from the descriptor's declared ranges.
///
/// The ranges are validated for chaotic behaviour across the whole declared
/// parameter span, so a sampled run stays healthy; the midpoint defaults of the
/// debug configuration are not — several catalog attractors (Thomas, Chua,
/// Moore–Spiegel) settle into fixed points or limit cycles exactly at the
/// midpoints.
pub fn select_attractor<R: RandomSource>(
    descriptor: &AttractorDescriptor,
    rng: &mut R,
) -> Result<AttractorSelection, RandomizerError> {
    validate_descriptor(descriptor)?;

    let mut parameters = HashMap::with_capacity(descriptor.parameters.len());
    for parameter in &descriptor.parameters {
        parameters.insert(
            parameter.name.clone(),
            rng.random_f64_inclusive(parameter.min..=parameter.max),
        );
    }

    let simulation = &descriptor.simulation;
    // Draw into locals so the consumption order is explicit and independent of
    // any argument-evaluation rules.
    let step_size = rng.random_f64_inclusive(simulation.step_size.min..=simulation.step_size.max);
    let simulation_speed =
        rng.random_f64_inclusive(simulation.simulation_speed.min..=simulation.simulation_speed.max);
    let camera_rotation_speed = rng.random_f64_inclusive(
        simulation.camera_rotation_speed.min..=simulation.camera_rotation_speed.max,
    );
    let trail_color_speed = rng
        .random_f64_inclusive(simulation.trail_color_speed.min..=simulation.trail_color_speed.max);
    let trail_length =
        rng.random_i64_inclusive(simulation.trail_length.min..=simulation.trail_length.max);
    let color_mode = TraceColorMode::ALL[rng.random_index(TraceColorMode::ALL.len())];
    let settings = SimulationSettings::new(
        step_size,
        simulation_speed,
        camera_rotation_speed,
        trail_color_speed,
        trail_length,
        color_mode,
    );

    let range = &descriptor.initial_state;
    let x = rng.random_f64_inclusive(range.x.min..=range.x.max);
    let y = rng.random_f64_inclusive(range.y.min..=range.y.max);
    let z = rng.random_f64_inclusive(range.z.min..=range.z.max);
    let initial_state = State::new(x, y, z);

    Ok(AttractorSelection::new(
        descriptor.clone(),
        parameters,
        settings,
        initial_state,
    ))
}

fn validate_descriptor(descriptor: &AttractorDescriptor) -> Result<(), RandomizerError> {
    for parameter in &descriptor.parameters {
        validate_range(
            parameter.min,
            parameter.max,
            format!("parameters.{}", parameter.name),
        )?;
    }
    validate_simulation(&descriptor.simulation)?;
    validate_range(
        descriptor.initial_state.x.min,
        descriptor.initial_state.x.max,
        "initialState.x".into(),
    )?;
    validate_range(
        descriptor.initial_state.y.min,
        descriptor.initial_state.y.max,
        "initialState.y".into(),
    )?;
    validate_range(
        descriptor.initial_state.z.min,
        descriptor.initial_state.z.max,
        "initialState.z".into(),
    )
}

fn validate_simulation(simulation: &SimulationHyperparameters) -> Result<(), RandomizerError> {
    validate_range(
        simulation.step_size.min,
        simulation.step_size.max,
        "stepSize".into(),
    )?;
    validate_range(
        simulation.simulation_speed.min,
        simulation.simulation_speed.max,
        "simulationSpeed".into(),
    )?;
    validate_range(
        simulation.camera_rotation_speed.min,
        simulation.camera_rotation_speed.max,
        "cameraRotationSpeed".into(),
    )?;
    validate_range(
        simulation.trail_color_speed.min,
        simulation.trail_color_speed.max,
        "trailColorSpeed".into(),
    )?;
    if simulation.trail_length.min > simulation.trail_length.max {
        return Err(RandomizerError::InvalidRange {
            field: "trailLength".into(),
            min: simulation.trail_length.min as f64,
            max: simulation.trail_length.max as f64,
        });
    }
    Ok(())
}

fn validate_range(min: f64, max: f64, field: String) -> Result<(), RandomizerError> {
    if min.is_finite() && max.is_finite() && min <= max {
        Ok(())
    } else {
        Err(RandomizerError::InvalidRange { field, min, max })
    }
}
