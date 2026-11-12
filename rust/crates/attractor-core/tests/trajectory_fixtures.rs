//! Golden trajectories from the Swift oracle, plus long-run invariants.
//!
//! `tests/fixtures/trajectories.json` records 1000 RK4 steps for Lorenz, Chua
//! and Aizawa and 500 Euler steps for Lorenz. Short horizons keep the
//! comparison meaningful: a chaotic flow amplifies the last bit of a double, so
//! long runs are checked by invariants (finite, bounded, moving) instead of
//! point-by-point equality.
//!
//! The generator lives in
//! `Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift`; see
//! `tests/fixtures/README.md` for the command that reproduces it.

use std::collections::HashMap;
use std::path::PathBuf;

use attractor_core::formula::{self, Expression, FormulaError};
use attractor_core::ode_solver::{self, IntegrationMethod};
use attractor_core::{AttractorDescriptor, Derivatives, MinMax, State, catalog};

/// Relative tolerance for a short chaotic trajectory.
const TRAJECTORY_TOLERANCE: f64 = 1e-9;

#[derive(Debug, serde::Deserialize)]
struct FixtureFile {
    entries: Vec<FixtureEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureEntry {
    attractor: String,
    method: String,
    step_size: String,
    parameters: HashMap<String, String>,
    initial_state: FixtureState,
    states: Vec<FixtureState>,
}

#[derive(Debug, serde::Deserialize)]
struct FixtureState {
    x: String,
    y: String,
    z: String,
}

impl FixtureState {
    fn parse(&self, context: &str) -> State {
        State::new(
            self.x
                .parse()
                .unwrap_or_else(|error| panic!("{context}: cannot parse x '{}': {error}", self.x)),
            self.y
                .parse()
                .unwrap_or_else(|error| panic!("{context}: cannot parse y '{}': {error}", self.y)),
            self.z
                .parse()
                .unwrap_or_else(|error| panic!("{context}: cannot parse z '{}': {error}", self.z)),
        )
    }
}

fn load_fixtures() -> FixtureFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trajectories.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}; regenerate it with the command in tests/fixtures/README.md",
            path.display()
        )
    });
    serde_json::from_str(&json).expect("fixture file is valid JSON")
}

/// The vector field of a descriptor bound to concrete parameter values.
///
/// `AttractorModel` arrives with the engine iteration; until then the tests
/// compose the parser and evaluator themselves.
struct TestModel {
    dx: Expression,
    dy: Expression,
    dz: Expression,
    parameters: HashMap<String, f64>,
}

impl TestModel {
    fn new(descriptor: &AttractorDescriptor, parameters: &HashMap<String, f64>) -> Self {
        let parse = |source: &str| {
            formula::parse(source).unwrap_or_else(|error| {
                panic!(
                    "{} formula '{source}' does not parse: {error}",
                    descriptor.id
                )
            })
        };
        Self {
            dx: parse(&descriptor.dx),
            dy: parse(&descriptor.dy),
            dz: parse(&descriptor.dz),
            parameters: parameters.clone(),
        }
    }

    fn rate(&self, state: State) -> Result<Derivatives, FormulaError> {
        let mut variables = self.parameters.clone();
        variables.insert("x".into(), state.x);
        variables.insert("y".into(), state.y);
        variables.insert("z".into(), state.z);
        Ok(Derivatives::new(
            self.dx.evaluate(&variables)?,
            self.dy.evaluate(&variables)?,
            self.dz.evaluate(&variables)?,
        ))
    }
}

fn method(name: &str) -> IntegrationMethod {
    match name {
        "euler" => IntegrationMethod::Euler,
        "rungeKutta4" => IntegrationMethod::RungeKutta4,
        other => panic!("unknown integration method '{other}'"),
    }
}

fn descriptor(catalog: &[AttractorDescriptor], id: &str) -> AttractorDescriptor {
    catalog
        .iter()
        .find(|descriptor| descriptor.id == id)
        .unwrap_or_else(|| panic!("the catalog has no '{id}' attractor"))
        .clone()
}

#[test]
fn fixture_covers_the_reference_attractors_and_methods() {
    let fixtures = load_fixtures();
    let mut seen: Vec<(String, String, usize)> = fixtures
        .entries
        .iter()
        .map(|entry| {
            (
                entry.attractor.clone(),
                entry.method.clone(),
                entry.states.len(),
            )
        })
        .collect();
    seen.sort();

    assert_eq!(
        seen,
        [
            ("aizawa".to_owned(), "rungeKutta4".to_owned(), 1001),
            ("chua".to_owned(), "rungeKutta4".to_owned(), 1001),
            ("lorenz".to_owned(), "euler".to_owned(), 501),
            ("lorenz".to_owned(), "rungeKutta4".to_owned(), 1001),
        ]
    );
}

#[test]
fn golden_trajectories_match_the_swift_oracle() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");

    for entry in load_fixtures().entries {
        let descriptor = descriptor(&catalog, &entry.attractor);
        let parameters: HashMap<String, f64> = entry
            .parameters
            .iter()
            .map(|(name, value)| {
                let value = value.parse().unwrap_or_else(|error| {
                    panic!(
                        "{}.{name}: cannot parse '{value}': {error}",
                        entry.attractor
                    )
                });
                (name.clone(), value)
            })
            .collect();
        let model = TestModel::new(&descriptor, &parameters);
        let step_size: f64 = entry
            .step_size
            .parse()
            .unwrap_or_else(|error| panic!("{}: cannot parse step size: {error}", entry.attractor));
        let method = method(&entry.method);

        let mut state = entry
            .initial_state
            .parse(&format!("{}.initialState", entry.attractor));
        assert_eq!(
            state,
            entry.states[0].parse(&format!("{}.states[0]", entry.attractor)),
            "{}.{}: fixture states must start at the initial state",
            entry.attractor,
            entry.method
        );

        for (index, expected) in entry.states.iter().enumerate().skip(1) {
            state = ode_solver::step(state, step_size, method, |state| model.rate(state))
                .unwrap_or_else(|error| {
                    panic!(
                        "{}.{} diverged at step {index}: {error}",
                        entry.attractor, entry.method
                    )
                });
            let expected = expected.parse(&format!(
                "{}.{} step {index}",
                entry.attractor, entry.method
            ));
            assert_close(
                state.x,
                expected.x,
                &format!("{}.{} step {index} x", entry.attractor, entry.method),
            );
            assert_close(
                state.y,
                expected.y,
                &format!("{}.{} step {index} y", entry.attractor, entry.method),
            );
            assert_close(
                state.z,
                expected.z,
                &format!("{}.{} step {index} z", entry.attractor, entry.method),
            );
        }
    }
}

#[test]
fn reference_attractors_stay_finite_and_bounded_over_a_long_run() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    const STEPS: usize = 20_000;

    for id in ["lorenz", "chua", "aizawa"] {
        let descriptor = descriptor(&catalog, id);
        let parameters = midpoint_parameters(&descriptor);
        let model = TestModel::new(&descriptor, &parameters);
        let step_size = midpoint(descriptor.simulation.step_size);
        let mut state = debug_initial_state(&descriptor);

        for step in 0..STEPS {
            state = ode_solver::step(state, step_size, IntegrationMethod::RungeKutta4, |state| {
                model.rate(state)
            })
            .unwrap_or_else(|error| panic!("{id} failed at step {step}: {error}"));
            assert!(
                is_bounded(state),
                "{id} left the bounded region at step {step}: {state:?}"
            );
        }
        assert_ne!(
            state,
            debug_initial_state(&descriptor),
            "{id} must keep moving, not settle at its start"
        );
    }
}

#[test]
fn both_methods_stay_finite_on_the_lorenz_system() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    let descriptor = descriptor(&catalog, "lorenz");
    let model = TestModel::new(&descriptor, &midpoint_parameters(&descriptor));

    for method in [IntegrationMethod::Euler, IntegrationMethod::RungeKutta4] {
        let mut state = debug_initial_state(&descriptor);
        for step in 0..2000 {
            state = ode_solver::step(state, 0.004, method, |state| model.rate(state))
                .unwrap_or_else(|error| panic!("{method:?} failed at step {step}: {error}"));
            assert!(is_bounded(state), "{method:?} left the bounded region");
        }
    }
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    let scale = expected.abs().max(1.0);
    let difference = (actual - expected).abs();
    assert!(
        difference <= TRAJECTORY_TOLERANCE * scale,
        "{context}: Rust integrated {actual:.17e}, oracle says {expected:.17e} (diff {difference:e})"
    );
}

fn is_bounded(state: State) -> bool {
    const LIMIT: f64 = 1e6;
    state.x.is_finite()
        && state.y.is_finite()
        && state.z.is_finite()
        && state.x.abs() < LIMIT
        && state.y.abs() < LIMIT
        && state.z.abs() < LIMIT
}

fn midpoint(range: MinMax<f64>) -> f64 {
    (range.min + range.max) / 2.0
}

fn midpoint_parameters(descriptor: &AttractorDescriptor) -> HashMap<String, f64> {
    descriptor
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                midpoint(MinMax::new(parameter.min, parameter.max)),
            )
        })
        .collect()
}

/// The debug configuration's start: the range midpoint nudged off the exact
/// centre (which is an equilibrium for several attractors), clamped back into
/// the declared range.
fn debug_initial_state(descriptor: &AttractorDescriptor) -> State {
    let range = &descriptor.initial_state;
    State::new(
        offset_into(range.x, 0.01),
        offset_into(range.y, -0.01),
        offset_into(range.z, 0.01),
    )
}

fn offset_into(range: MinMax<f64>, delta: f64) -> f64 {
    (midpoint(range) + delta).clamp(range.min, range.max)
}
