//! The random-selection contract: ranges, determinism and validation errors.

use std::collections::HashSet;

use attractor_core::randomizer::{self, RandomizerError};
use attractor_core::{
    AttractorDescriptor, MinMax, Parameter, SimulationHyperparameters, SplitMix64, StateRange,
    TraceColorMode,
};

fn lorenz_descriptor() -> AttractorDescriptor {
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

fn thomas_descriptor() -> AttractorDescriptor {
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

fn catalog() -> Vec<AttractorDescriptor> {
    vec![lorenz_descriptor(), thomas_descriptor()]
}

#[test]
fn builds_a_valid_selection_from_every_draw() {
    let mut rng = SplitMix64::new(1);
    for _ in 0..25 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng)
            .expect("the test catalog is valid");
        assert!(selection.settings.validation_issues().is_empty());
        assert!(selection.parameters.values().all(|value| value.is_finite()));
        assert!(selection.initial_state.x.is_finite());
        assert!(selection.initial_state.y.is_finite());
        assert!(selection.initial_state.z.is_finite());
    }
}

#[test]
fn covers_every_declared_parameter() {
    let mut rng = SplitMix64::new(3);
    for _ in 0..25 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        for parameter in &selection.descriptor.parameters {
            assert!(
                selection.parameters.contains_key(&parameter.name),
                "{} is missing {}",
                selection.descriptor.id,
                parameter.name
            );
        }
    }
}

#[test]
fn parameter_values_stay_within_their_declared_ranges() {
    let mut rng = SplitMix64::new(7);
    for _ in 0..25 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        for parameter in &selection.descriptor.parameters {
            let value = selection.parameters[&parameter.name];
            assert!(value >= parameter.min, "{} below range", parameter.name);
            assert!(value <= parameter.max, "{} above range", parameter.name);
            assert!(value.is_finite());
        }
    }
}

#[test]
fn hyperparameter_values_stay_within_their_declared_ranges() {
    let mut rng = SplitMix64::new(11);
    for _ in 0..25 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        let simulation = &selection.descriptor.simulation;
        assert!(selection.settings.step_size >= simulation.step_size.min);
        assert!(selection.settings.step_size <= simulation.step_size.max);
        assert!(selection.settings.simulation_speed >= simulation.simulation_speed.min);
        assert!(selection.settings.simulation_speed <= simulation.simulation_speed.max);
        assert!(selection.settings.camera_rotation_speed >= simulation.camera_rotation_speed.min);
        assert!(selection.settings.camera_rotation_speed <= simulation.camera_rotation_speed.max);
        assert!(selection.settings.trail_color_speed >= simulation.trail_color_speed.min);
        assert!(selection.settings.trail_color_speed <= simulation.trail_color_speed.max);
        assert!(selection.settings.trail_length >= simulation.trail_length.min);
        assert!(selection.settings.trail_length <= simulation.trail_length.max);
    }
}

#[test]
fn same_seed_reproduces_the_same_selection() {
    let mut first_rng = SplitMix64::new(99);
    let first = randomizer::select_from_catalog(&catalog(), &mut first_rng).unwrap();
    let mut second_rng = SplitMix64::new(99);
    let second = randomizer::select_from_catalog(&catalog(), &mut second_rng).unwrap();
    assert_eq!(first, second);
}

#[test]
fn varied_seeds_pick_more_than_one_attractor() {
    let mut chosen = HashSet::new();
    for seed in 0..40 {
        let mut rng = SplitMix64::new(seed);
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        chosen.insert(selection.descriptor.id.clone());
    }
    assert!(chosen.len() > 1);
}

#[test]
fn initial_state_coordinates_stay_within_their_declared_ranges() {
    let mut rng = SplitMix64::new(17);
    for _ in 0..25 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        let range = &selection.descriptor.initial_state;
        assert!(selection.initial_state.x >= range.x.min);
        assert!(selection.initial_state.x <= range.x.max);
        assert!(selection.initial_state.y >= range.y.min);
        assert!(selection.initial_state.y <= range.y.max);
        assert!(selection.initial_state.z >= range.z.min);
        assert!(selection.initial_state.z <= range.z.max);
    }
}

#[test]
fn color_mode_is_one_of_the_declared_modes() {
    let mut rng = SplitMix64::new(13);
    for _ in 0..40 {
        let selection = randomizer::select_from_catalog(&catalog(), &mut rng).unwrap();
        assert!(TraceColorMode::ALL.contains(&selection.settings.color_mode));
    }
}

#[test]
fn an_empty_catalog_is_rejected_instead_of_trapping() {
    let mut rng = SplitMix64::new(1);
    let error = randomizer::select_from_catalog(&[], &mut rng).unwrap_err();
    assert!(matches!(error, RandomizerError::EmptyCatalog));
}

#[test]
fn a_malformed_parameter_range_is_rejected() {
    let mut descriptor = lorenz_descriptor();
    descriptor.parameters = vec![Parameter::new("a", 1.0, 0.0)];
    let mut rng = SplitMix64::new(2);

    let error = randomizer::select_attractor(&descriptor, &mut rng).unwrap_err();
    assert_eq!(
        error,
        RandomizerError::InvalidRange {
            field: "parameters.a".into(),
            min: 1.0,
            max: 0.0,
        }
    );
}

/// A mutation that makes one simulation range unusable.
type RangeBreaker = fn(&mut SimulationHyperparameters);

#[test]
fn malformed_simulation_ranges_report_their_field() {
    let cases: [(&str, RangeBreaker); 4] = [
        ("stepSize", |simulation| {
            simulation.step_size = MinMax::new(0.0, -1.0)
        }),
        ("simulationSpeed", |simulation| {
            simulation.simulation_speed = MinMax::new(f64::NAN, 1.0)
        }),
        ("cameraRotationSpeed", |simulation| {
            simulation.camera_rotation_speed = MinMax::new(2.0, 1.0)
        }),
        ("trailColorSpeed", |simulation| {
            simulation.trail_color_speed = MinMax::new(f64::INFINITY, 1.0)
        }),
    ];

    for (field, break_range) in cases {
        let mut descriptor = lorenz_descriptor();
        break_range(&mut descriptor.simulation);
        let mut rng = SplitMix64::new(5);
        let error = randomizer::select_attractor(&descriptor, &mut rng).unwrap_err();
        assert!(
            matches!(&error, RandomizerError::InvalidRange { field: reported, .. } if reported == field),
            "{field} should be rejected, got {error:?}"
        );
    }
}

#[test]
fn an_inverted_trail_length_is_rejected() {
    let mut descriptor = lorenz_descriptor();
    descriptor.simulation.trail_length = MinMax::new(10, 5);
    let mut rng = SplitMix64::new(5);
    let error = randomizer::select_attractor(&descriptor, &mut rng).unwrap_err();
    assert_eq!(
        error,
        RandomizerError::InvalidRange {
            field: "trailLength".into(),
            min: 10.0,
            max: 5.0,
        }
    );
}

#[test]
fn malformed_initial_state_ranges_report_their_field() {
    let cases: [(&str, MinMax<f64>); 3] = [
        ("initialState.x", MinMax::new(2.0, 1.0)),
        ("initialState.y", MinMax::new(f64::NAN, 1.0)),
        ("initialState.z", MinMax::new(-1.0, f64::INFINITY)),
    ];
    for (field, range) in cases {
        let mut descriptor = lorenz_descriptor();
        match field {
            "initialState.x" => descriptor.initial_state.x = range,
            "initialState.y" => descriptor.initial_state.y = range,
            _ => descriptor.initial_state.z = range,
        }
        let mut rng = SplitMix64::new(6);
        let error = randomizer::select_attractor(&descriptor, &mut rng).unwrap_err();
        assert!(
            matches!(&error, RandomizerError::InvalidRange { field: reported, .. } if reported == field),
            "{field} should be rejected, got {error:?}"
        );
    }
}
