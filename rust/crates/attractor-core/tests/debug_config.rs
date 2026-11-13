//! The debug configuration fills omitted values with range midpoints and
//! nudges the start off the symmetry-breaking midpoint.

mod support;

use std::collections::HashMap;

use attractor_core::{
    AttractorDescriptor, DebugConfig, DebugConfigError, MinMax, SimulationHyperparameters,
    SimulationSettings, StateRange, TraceColorMode,
};
use support::{lorenz_descriptor, thomas_descriptor};

fn catalog() -> Vec<AttractorDescriptor> {
    vec![lorenz_descriptor(), thomas_descriptor()]
}

#[test]
fn builds_a_selection_for_a_known_attractor() {
    let selection = DebugConfig::selection("lorenz", &catalog()).unwrap();
    assert_eq!(selection.descriptor.id, "lorenz");
}

#[test]
fn unknown_attractor_id_is_rejected() {
    let error = DebugConfig::selection("nope", &catalog()).unwrap_err();
    assert_eq!(error, DebugConfigError::UnknownAttractor("nope".into()));
}

#[test]
fn omitted_parameters_default_to_range_midpoints() {
    let selection = DebugConfig::selection("lorenz", &catalog()).unwrap();
    let declared = lorenz_descriptor()
        .parameters
        .into_iter()
        .find(|parameter| parameter.name == "r")
        .unwrap();
    assert_eq!(
        selection.parameters["r"],
        (declared.min + declared.max) / 2.0
    );
}

#[test]
fn explicit_parameters_are_preserved() {
    let parameters = HashMap::from([("r".to_string(), 28.0)]);
    let selection = DebugConfig::selection_with("lorenz", &catalog(), parameters, None).unwrap();
    assert_eq!(selection.parameters["r"], 28.0);
}

#[test]
fn absent_settings_default_to_range_midpoints() {
    let selection = DebugConfig::selection("thomas", &catalog()).unwrap();
    let simulation = thomas_descriptor().simulation;
    assert_eq!(
        selection.settings.trail_length,
        (simulation.trail_length.min + simulation.trail_length.max) / 2
    );
    assert_eq!(
        selection.settings.step_size,
        (simulation.step_size.min + simulation.step_size.max) / 2.0
    );
}

#[test]
fn explicit_settings_are_preserved() {
    let settings = SimulationSettings::new(0.05, 2.0, 0.3, 0.1, 1234, TraceColorMode::Trail);
    let selection =
        DebugConfig::selection_with("lorenz", &catalog(), HashMap::new(), Some(settings)).unwrap();
    assert_eq!(selection.settings, settings);
}

#[test]
fn resulting_model_builds_successfully() {
    let parameters = HashMap::from([("r".to_string(), 28.0)]);
    let selection = DebugConfig::selection_with("lorenz", &catalog(), parameters, None).unwrap();
    selection.build_model().unwrap();
}

#[test]
fn initial_state_is_nudged_off_the_degenerate_midpoint() {
    let selection = DebugConfig::selection("thomas", &catalog()).unwrap();
    let range = thomas_descriptor().initial_state;
    let mid_x = (range.x.min + range.x.max) / 2.0;
    // The midpoint of a center-symmetric range is an equilibrium for Thomas;
    // the debug configuration must start off it so the simulation moves.
    assert_ne!(selection.initial_state.x, mid_x);
    assert!(selection.initial_state.x > range.x.min && selection.initial_state.x < range.x.max);
}

#[test]
fn the_offset_never_pushes_a_start_outside_a_narrow_range() {
    let narrow = AttractorDescriptor {
        id: "narrow".into(),
        name: "Narrow".into(),
        dx: "-x".into(),
        dy: "-y".into(),
        dz: "-z".into(),
        parameters: vec![],
        simulation: SimulationHyperparameters::default(),
        initial_state: StateRange::new(
            MinMax::new(0.0, 0.001),
            MinMax::new(0.0, 0.001),
            MinMax::new(0.0, 0.001),
        ),
    };
    let selection = DebugConfig::selection("narrow", std::slice::from_ref(&narrow)).unwrap();
    let range = narrow.initial_state;
    for (value, bounds) in [
        (selection.initial_state.x, range.x),
        (selection.initial_state.y, range.y),
        (selection.initial_state.z, range.z),
    ] {
        assert!(value >= bounds.min && value <= bounds.max);
    }
}
