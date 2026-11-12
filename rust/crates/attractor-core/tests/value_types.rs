//! Contract tests for the shared value types: constructors, ranges, defaults,
//! midpoints and settings validation.

use attractor_core::{
    Derivatives, MinMax, Parameter, SimulationHyperparameters, SimulationSettings, State,
    StateRange, TraceColorMode,
};

#[test]
fn min_max_exposes_its_closed_range() {
    let range = MinMax::new(-1.0, 2.5);
    assert_eq!(range.min, -1.0);
    assert_eq!(range.max, 2.5);
    assert_eq!(range.closed_range(), -1.0..=2.5);
}

#[test]
fn state_and_derivatives_compare_by_value() {
    let state = State::new(1.0, 2.0, 3.0);
    assert_eq!(state, State::new(1.0, 2.0, 3.0));
    assert_ne!(state, State::new(3.0, 2.0, 1.0));

    let derivatives = Derivatives::new(0.1, -0.2, 0.3);
    assert_eq!(derivatives, Derivatives::new(0.1, -0.2, 0.3));
}

#[test]
fn state_range_and_parameter_keep_their_bounds() {
    let initial = StateRange::new(
        MinMax::new(-1.0, 1.0),
        MinMax::new(-2.0, 2.0),
        MinMax::new(-3.0, 3.0),
    );
    assert_eq!(initial.y, MinMax::new(-2.0, 2.0));

    let parameter = Parameter::new("a", 9.0, 10.0);
    assert_eq!(parameter.name, "a");
    assert_eq!(parameter.min, 9.0);
    assert_eq!(parameter.max, 10.0);
}

#[test]
fn simulation_hyperparameters_default_to_the_reference_ranges() {
    let hyperparameters = SimulationHyperparameters::default();
    assert_eq!(hyperparameters.step_size, MinMax::new(0.004, 0.01));
    assert_eq!(hyperparameters.simulation_speed, MinMax::new(1.0, 3.0));
    assert_eq!(hyperparameters.camera_rotation_speed, MinMax::new(0.1, 0.4));
    assert_eq!(hyperparameters.trail_color_speed, MinMax::new(0.05, 0.3));
    assert_eq!(hyperparameters.trail_length, MinMax::new(2000, 8000));
}

#[test]
fn settings_midpoints_sample_every_range() {
    let settings = SimulationSettings::midpoints_of(&SimulationHyperparameters::default());
    assert_eq!(settings.step_size, 0.007);
    assert_eq!(settings.simulation_speed, 2.0);
    assert_eq!(settings.camera_rotation_speed, 0.25);
    assert_eq!(settings.trail_color_speed, 0.175);
    assert_eq!(settings.trail_length, 5000);
    assert_eq!(settings.color_mode, TraceColorMode::Trail);
}

#[test]
fn midpoint_settings_are_valid() {
    let settings = SimulationSettings::midpoints_of(&SimulationHyperparameters::default());
    assert!(settings.validation_issues().is_empty());
}

#[test]
fn settings_reject_a_non_positive_or_non_finite_step_size() {
    for step_size in [0.0, -0.01, f64::NAN, f64::INFINITY] {
        let settings = settings_with(|settings| settings.step_size = step_size);
        assert!(
            mentions(&settings.validation_issues(), "stepSize"),
            "step size {step_size} should be rejected"
        );
    }
}

/// A field setter used to exercise every speed field with the same loop.
type SpeedSetter = fn(&mut SimulationSettings, f64);

#[test]
fn settings_reject_negative_or_non_finite_speeds() {
    let cases: [(&str, SpeedSetter); 3] = [
        ("simulationSpeed", |settings, value| {
            settings.simulation_speed = value
        }),
        ("cameraRotationSpeed", |settings, value| {
            settings.camera_rotation_speed = value
        }),
        ("trailColorSpeed", |settings, value| {
            settings.trail_color_speed = value
        }),
    ];

    for (field, set) in cases {
        for value in [-0.1, f64::NAN, f64::INFINITY] {
            let settings = settings_with(|settings| set(settings, value));
            assert!(
                mentions(&settings.validation_issues(), field),
                "{field} = {value} should be rejected"
            );
        }
        // Zero is allowed: it intentionally freezes one aspect for debugging.
        let settings = settings_with(|settings| set(settings, 0.0));
        assert!(
            !mentions(&settings.validation_issues(), field),
            "{field} = 0 should be allowed"
        );
    }
}

#[test]
fn settings_reject_a_trail_shorter_than_two_points() {
    for trail_length in [0, 1] {
        let settings = settings_with(|settings| settings.trail_length = trail_length);
        assert!(mentions(&settings.validation_issues(), "trailLength"));
    }
    let settings = settings_with(|settings| settings.trail_length = 2);
    assert!(!mentions(&settings.validation_issues(), "trailLength"));
}

#[test]
fn trace_color_mode_declares_four_distinct_modes() {
    assert_eq!(TraceColorMode::ALL.len(), 4);
    let raw_values: Vec<&str> = TraceColorMode::ALL
        .iter()
        .map(|mode| mode.as_str())
        .collect();
    assert_eq!(raw_values, ["trail", "age", "depth", "spatial"]);
}

fn settings_with(change: impl FnOnce(&mut SimulationSettings)) -> SimulationSettings {
    let mut settings = SimulationSettings::midpoints_of(&SimulationHyperparameters::default());
    change(&mut settings);
    settings
}

fn mentions(issues: &[String], field: &str) -> bool {
    issues.iter().any(|issue| issue.contains(field))
}
