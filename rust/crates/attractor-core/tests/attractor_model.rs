//! An attractor model parses its formulas once and evaluates the vector field
//! against state coordinates and bound parameters.

mod support;

use std::collections::HashMap;

use attractor_core::formula::FormulaError;
use attractor_core::{
    AttractorDescriptor, AttractorModel, AttractorModelError, MinMax, SimulationHyperparameters,
    State, StateRange,
};
use support::{lorenz_descriptor, thomas_descriptor};

fn lorenz_parameters() -> HashMap<String, f64> {
    HashMap::from([
        ("a".to_string(), 10.0),
        ("b".to_string(), 8.0 / 3.0),
        ("r".to_string(), 28.0),
    ])
}

#[test]
fn lorenz_rate_matches_the_reference_formula_at_a_known_state() {
    let model = AttractorModel::new(lorenz_descriptor(), lorenz_parameters()).unwrap();
    let derivative = model.rate(State::new(1.0, 1.0, 1.0)).unwrap();

    // x' = 10 * (1 - 1) = 0
    assert!((derivative.dx - 0.0).abs() < 1e-9);
    // y' = 1 * (28 - 1) - 1 = 26
    assert!((derivative.dy - 26.0).abs() < 1e-9);
    // z' = 1*1 - (8/3)*1 = 1 - 8/3
    assert!((derivative.dz - (1.0 - 8.0 / 3.0)).abs() < 1e-9);
}

#[test]
fn thomas_attractor_parses_and_evaluates_sin() {
    let parameters = HashMap::from([("b".to_string(), 0.2081)]);
    let model = AttractorModel::new(thomas_descriptor(), parameters).unwrap();
    let derivative = model.rate(State::new(0.5, 0.5, 0.5)).unwrap();

    let expected = 0.5_f64.sin() - 0.2081 * 0.5;
    assert!((derivative.dx - expected).abs() < 1e-9);
    assert!((derivative.dy - expected).abs() < 1e-9);
    assert!((derivative.dz - expected).abs() < 1e-9);
}

#[test]
fn missing_declared_parameter_is_rejected() {
    let parameters = HashMap::from([("a".to_string(), 10.0)]);
    let error = AttractorModel::new(lorenz_descriptor(), parameters).unwrap_err();
    assert!(matches!(error, AttractorModelError::MissingParameter(name) if name == "b"));
}

#[test]
fn unparsable_formula_is_rejected() {
    let descriptor = AttractorDescriptor {
        id: "broken".into(),
        name: "Broken".into(),
        dx: "sin(".into(), // truncated function call
        dy: "x".into(),
        dz: "y".into(),
        parameters: vec![],
        simulation: SimulationHyperparameters::default(),
        initial_state: StateRange::new(
            MinMax::new(-1.0, 1.0),
            MinMax::new(-1.0, 1.0),
            MinMax::new(-1.0, 1.0),
        ),
    };
    let error = AttractorModel::new(descriptor, HashMap::new()).unwrap_err();
    assert!(matches!(error, AttractorModelError::Formula(_)));
}

#[test]
fn formula_error_is_exposed_as_its_source() {
    let error = AttractorModelError::Formula(FormulaError::UnexpectedEndOfInput);
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().contains("unexpected end of input"));
}
