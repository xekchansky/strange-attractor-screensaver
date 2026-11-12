//! Port of the Swift `AttractorCatalogValidationTests`.
//!
//! Every test breaks exactly one rule and asserts on the resulting catalog
//! issue, so a single malformed entry can never blank the whole screen saver.

use attractor_core::catalog::{self, AttractorCatalogValidation};
use attractor_core::{
    AttractorDescriptor, MinMax, Parameter, SimulationHyperparameters, StateRange,
};

/// Build a Lorenz-like descriptor, so each test can override one field.
fn descriptor(
    id: &str,
    dx: &str,
    dy: &str,
    dz: &str,
    parameters: Vec<Parameter>,
    simulation: SimulationHyperparameters,
    initial_state: StateRange,
) -> AttractorDescriptor {
    AttractorDescriptor {
        id: id.to_owned(),
        name: id.to_owned(),
        dx: dx.to_owned(),
        dy: dy.to_owned(),
        dz: dz.to_owned(),
        parameters,
        simulation,
        initial_state,
    }
}

fn lorenz() -> AttractorDescriptor {
    descriptor(
        "lorenz",
        "a * (y - x)",
        "x * (r - z) - y",
        "x * y - b * z",
        vec![
            Parameter::new("a", 9.0, 10.0),
            Parameter::new("b", 2.5, 3.0),
            Parameter::new("r", 24.0, 30.0),
        ],
        SimulationHyperparameters::default(),
        unit_state_range(),
    )
}

fn unit_state_range() -> StateRange {
    StateRange::new(
        MinMax::new(-1.0, 1.0),
        MinMax::new(-1.0, 1.0),
        MinMax::new(-1.0, 1.0),
    )
}

fn has_issue(validation: &AttractorCatalogValidation, descriptor_id: &str, field: &str) -> bool {
    validation
        .issues
        .iter()
        .any(|issue| issue.descriptor_id == descriptor_id && issue.field == field)
}

#[test]
fn the_bundled_catalog_is_fully_valid() {
    let validation = catalog::bundled_validation().expect("bundled catalog should decode");

    assert!(validation.issues.is_empty(), "{:?}", validation.issues);
    assert_eq!(validation.valid.len(), 12);
}

#[test]
fn an_empty_catalog_reports_an_issue_and_has_no_valid_descriptors() {
    let validation = catalog::validate(&[]);

    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "", "catalog"));
}

#[test]
fn drops_a_descriptor_whose_step_size_is_not_positive() {
    let mut broken = lorenz();
    broken.simulation.step_size = MinMax::new(0.0, 0.01);

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "stepSize"));
}

#[test]
fn drops_a_descriptor_whose_trail_is_shorter_than_two_points() {
    let mut broken = lorenz();
    broken.simulation.trail_length = MinMax::new(1, 1);

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "trailLength"));
}

#[test]
fn drops_a_descriptor_with_a_negative_trail_length_and_keeps_siblings() {
    let mut broken = lorenz();
    broken.id = "broken".to_owned();
    broken.name = "broken".to_owned();
    broken.simulation.trail_length = MinMax::new(-5, 100);
    let fine = lorenz();

    let validation = catalog::validate(&[broken, fine.clone()]);
    assert_eq!(validation.valid, [fine]);
    assert!(has_issue(&validation, "broken", "trailLength"));
}

#[test]
fn drops_a_descriptor_with_an_inverted_range() {
    let mut broken = lorenz();
    broken.simulation.trail_length = MinMax::new(5000, 100);

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "trailLength"));
}

#[test]
fn drops_a_descriptor_whose_formula_does_not_parse() {
    let mut broken = lorenz();
    broken.dx = "a * (y - x".to_owned();

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "dx"));
}

#[test]
fn drops_a_descriptor_whose_formula_uses_an_unknown_variable() {
    let mut broken = lorenz();
    broken.dz = "x * y - q".to_owned();

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "dz"));
}

#[test]
fn drops_a_descriptor_with_a_duplicate_parameter_name() {
    let mut broken = lorenz();
    broken.parameters = vec![
        Parameter::new("a", 9.0, 10.0),
        Parameter::new("a", 1.0, 2.0),
    ];

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "parameters"));
}

#[test]
fn drops_a_descriptor_that_reuses_a_state_coordinate_as_a_parameter() {
    let mut broken = lorenz();
    broken.parameters = vec![Parameter::new("x", 1.0, 2.0)];

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "parameters"));
}

#[test]
fn drops_a_descriptor_with_an_inverted_initial_state_range() {
    let mut broken = lorenz();
    broken.initial_state = StateRange::new(
        MinMax::new(1.0, -1.0),
        MinMax::new(-1.0, 1.0),
        MinMax::new(-1.0, 1.0),
    );

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "lorenz", "initialState.x"));
}

#[test]
fn drops_a_duplicate_id_and_keeps_the_first_descriptor() {
    let first = lorenz();
    let mut second = lorenz();
    second.dx = "a * (x - y)".to_owned();

    let validation = catalog::validate(&[first.clone(), second]);
    assert_eq!(validation.valid, [first]);
    assert!(has_issue(&validation, "lorenz", "id"));
}

#[test]
fn keeps_valid_descriptors_when_a_sibling_is_broken() {
    let mut broken = lorenz();
    broken.id = "broken".to_owned();
    broken.name = "broken".to_owned();
    broken.dx = "oops(".to_owned();
    let fine = lorenz();

    let validation = catalog::validate(&[broken, fine.clone()]);
    assert_eq!(validation.valid, [fine]);
    assert!(has_issue(&validation, "broken", "dx"));
}

#[test]
fn rejects_formulas_calling_a_function_the_engine_does_not_support() {
    // `sinh` parses as a function call but is not implemented, so without this
    // check the entry would only fail at evaluation time — after the screen
    // saver has already started.
    let mut broken = lorenz();
    broken.id = "broken".to_owned();
    broken.name = "broken".to_owned();
    broken.dx = "sinh(y)".to_owned();

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "broken", "dx"));
}

#[test]
fn rejects_a_descriptor_with_an_empty_id() {
    let mut broken = lorenz();
    broken.id = "   ".to_owned();

    let validation = catalog::validate(&[broken]);
    assert!(validation.valid.is_empty());
    assert!(has_issue(&validation, "   ", "id"));
}
