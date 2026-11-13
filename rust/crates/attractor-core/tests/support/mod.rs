//! Shared descriptors for the engine, model, debug-config and health-monitor
//! tests.

// Each test binary includes this module and uses a different subset.
#![allow(dead_code)]

use attractor_core::{
    AttractorDescriptor, MinMax, Parameter, SimulationHyperparameters, StateRange,
};

/// The reference Lorenz descriptor with classic numeric constants.
pub fn lorenz_descriptor() -> AttractorDescriptor {
    AttractorDescriptor {
        id: "lorenz".into(),
        name: "Lorenz".into(),
        dx: "a * (y - x)".into(),
        dy: "x * (r - z) - y".into(),
        dz: "x * y - b * z".into(),
        parameters: vec![
            Parameter::new("a", 9.0, 10.0),
            Parameter::new("b", 2.5, 3.0),
            Parameter::new("r", 24.0, 30.0),
        ],
        simulation: SimulationHyperparameters::default(),
        initial_state: unit_state_range(),
    }
}

/// The Thomas attractor, which exercises the `sin` function.
pub fn thomas_descriptor() -> AttractorDescriptor {
    AttractorDescriptor {
        id: "thomas".into(),
        name: "Thomas".into(),
        dx: "sin(y) - b * x".into(),
        dy: "sin(z) - b * y".into(),
        dz: "sin(x) - b * z".into(),
        parameters: vec![Parameter::new("b", 0.18, 0.24)],
        simulation: SimulationHyperparameters::default(),
        initial_state: unit_state_range(),
    }
}

fn unit_state_range() -> StateRange {
    StateRange::new(
        MinMax::new(-1.0, 1.0),
        MinMax::new(-1.0, 1.0),
        MinMax::new(-1.0, 1.0),
    )
}
