//! End-to-end determinism against the Swift engine.
//!
//! `tests/fixtures/engine_traces.json` records, for sixteen seeds, the
//! selection the Swift randomizer makes and the 200-step trajectory the Swift
//! engine produces from it. The Rust port must select the same values and stay
//! within the trajectory tolerance; together with the randomizer fixtures this
//! is the gate that proves I2+I3 are exact.
//!
//! The generator lives in the archived oracle tag `swift-oracle`
//! (`Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift`);
//! see `tests/fixtures/README.md` for the command that reproduces it.

use std::collections::HashMap;
use std::path::PathBuf;

use attractor_core::randomizer;
use attractor_core::{AttractorEngine, IntegrationMethod, SplitMix64, State, catalog};

/// Relative tolerance for a short chaotic trajectory.
const TRAJECTORY_TOLERANCE: f64 = 1e-9;

#[derive(Debug, serde::Deserialize)]
struct FixtureFile {
    entries: Vec<FixtureEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureEntry {
    seed: u64,
    attractor: String,
    parameters: HashMap<String, String>,
    settings: FixtureSettings,
    initial_state: FixtureState,
    trail_capacity: usize,
    steps: usize,
    states: Vec<FixtureState>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureSettings {
    step_size: String,
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

fn fixture() -> FixtureFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/engine_traces.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

fn assert_close(actual: State, expected: State, context: &str) {
    for (actual, expected, axis) in [
        (actual.x, expected.x, "x"),
        (actual.y, expected.y, "y"),
        (actual.z, expected.z, "z"),
    ] {
        let scale = expected.abs().max(1.0);
        let difference = (actual - expected).abs();
        assert!(
            difference <= TRAJECTORY_TOLERANCE * scale,
            "{context}: {axis} is {actual:.17e}, oracle says {expected:.17e} (diff {difference:e})"
        );
    }
}

fn parse_exact(value: &str, context: &str) -> f64 {
    value
        .parse()
        .unwrap_or_else(|error| panic!("{context}: cannot parse '{value}': {error}"))
}

#[test]
fn fixture_covers_sixteen_seeds_with_a_ring_wrap() {
    let fixture = fixture();
    assert_eq!(fixture.entries.len(), 16);
    for entry in &fixture.entries {
        assert_eq!(entry.states.len(), entry.steps + 1);
        assert!(
            entry.trail_capacity < entry.steps,
            "seed {}: the trace must wrap to exercise the ring",
            entry.seed
        );
    }
}

#[test]
fn seeded_engine_traces_match_the_swift_oracle() {
    let catalog = catalog::bundled().unwrap();
    let fixture = fixture();

    for entry in &fixture.entries {
        let context = format!("seed {}", entry.seed);
        let mut rng = SplitMix64::new(entry.seed);
        let selection = randomizer::select_from_catalog(&catalog, &mut rng).unwrap();

        assert_eq!(
            selection.descriptor.id, entry.attractor,
            "{context}: attractor"
        );
        assert_eq!(
            selection.parameters.len(),
            entry.parameters.len(),
            "{context}: parameter count"
        );
        for (name, expected) in &entry.parameters {
            assert_eq!(
                selection.parameters[name],
                parse_exact(expected, &format!("{context}: parameter {name}")),
                "{context}: parameter {name}"
            );
        }

        let step_size = parse_exact(&entry.settings.step_size, &context);
        assert_eq!(
            selection.settings.step_size, step_size,
            "{context}: step size"
        );
        assert_close(
            selection.initial_state,
            entry.initial_state.parse(&context),
            &format!("{context}: initial state"),
        );

        let mut engine = AttractorEngine::new(
            selection.build_model().unwrap(),
            selection.initial_state,
            IntegrationMethod::RungeKutta4,
            step_size,
            entry.trail_capacity,
        );

        for (step, expected) in entry.states.iter().enumerate().skip(1) {
            let actual = engine.advance().unwrap();
            assert_close(
                actual,
                expected.parse(&context),
                &format!("{context}: step {step}"),
            );
        }

        // The ring keeps the newest `trail_capacity` states, oldest first.
        let trace = engine.trace();
        assert_eq!(trace.len(), entry.trail_capacity, "{context}: trace length");
        let offset = entry.states.len() - trace.len();
        for position in 0..trace.len() {
            assert_close(
                trace[position],
                entry.states[offset + position].parse(&context),
                &format!("{context}: trace point {position}"),
            );
        }
        assert_eq!(Some(engine.state()), trace.last());
    }
}
