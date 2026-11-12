//! Seeded selections produced by the Swift oracle.
//!
//! `tests/fixtures/randomizer_selections.json` records `Randomizer.select`
//! for sixteen seeds against the bundled catalog. Because the Rust port
//! reproduces the Swift standard library's integer and float range drawing, a
//! seed must select bit-identical values, not merely values in the same range.
//! The generator lives in
//! `Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift`; see
//! `tests/fixtures/README.md` for the command that reproduces it.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use attractor_core::SplitMix64;
use attractor_core::catalog;
use attractor_core::randomizer;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    entries: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    seed: u64,
    attractor: String,
    parameters: HashMap<String, String>,
    settings: FixtureSettings,
    #[serde(rename = "initialState")]
    initial_state: FixtureState,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureSettings {
    step_size: String,
    simulation_speed: String,
    camera_rotation_speed: String,
    trail_color_speed: String,
    trail_length: i64,
    color_mode: String,
}

#[derive(Debug, Deserialize)]
struct FixtureState {
    x: String,
    y: String,
    z: String,
}

fn load_fixtures() -> FixtureFile {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/randomizer_selections.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}; regenerate it with the command in tests/fixtures/README.md",
            path.display()
        )
    });
    serde_json::from_str(&json).expect("fixture file is valid JSON")
}

fn oracle_value(text: &str, context: &str) -> f64 {
    text.parse::<f64>()
        .unwrap_or_else(|error| panic!("{context}: cannot parse '{text}' as f64: {error}"))
}

fn assert_bits(actual: f64, expected: &str, context: &str) {
    let expected = oracle_value(expected, context);
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{context}: Rust picked {actual:e}, oracle says {expected:e}"
    );
}

#[test]
fn fixture_covers_sixteen_distinct_seeds() {
    let fixtures = load_fixtures();
    assert_eq!(fixtures.entries.len(), 16);
    let seeds: HashSet<u64> = fixtures.entries.iter().map(|entry| entry.seed).collect();
    assert_eq!(seeds.len(), fixtures.entries.len());
}

#[test]
fn seeded_selections_match_the_swift_oracle() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");

    for entry in load_fixtures().entries {
        let mut rng = SplitMix64::new(entry.seed);
        let selection = randomizer::select_from_catalog(&catalog, &mut rng)
            .unwrap_or_else(|error| panic!("seed {} failed: {error}", entry.seed));

        assert_eq!(
            selection.descriptor.id, entry.attractor,
            "seed {} picked the wrong attractor",
            entry.seed
        );

        assert_eq!(
            selection.parameters.len(),
            entry.parameters.len(),
            "seed {} sampled a different parameter set",
            entry.seed
        );
        for (name, expected) in &entry.parameters {
            let actual = selection.parameters[name];
            assert_bits(
                actual,
                expected,
                &format!("seed {} parameter {name}", entry.seed),
            );
        }

        assert_bits(
            selection.settings.step_size,
            &entry.settings.step_size,
            &format!("seed {} stepSize", entry.seed),
        );
        assert_bits(
            selection.settings.simulation_speed,
            &entry.settings.simulation_speed,
            &format!("seed {} simulationSpeed", entry.seed),
        );
        assert_bits(
            selection.settings.camera_rotation_speed,
            &entry.settings.camera_rotation_speed,
            &format!("seed {} cameraRotationSpeed", entry.seed),
        );
        assert_bits(
            selection.settings.trail_color_speed,
            &entry.settings.trail_color_speed,
            &format!("seed {} trailColorSpeed", entry.seed),
        );
        assert_eq!(
            selection.settings.trail_length, entry.settings.trail_length,
            "seed {} trailLength",
            entry.seed
        );
        assert_eq!(
            selection.settings.color_mode.as_str(),
            entry.settings.color_mode,
            "seed {} colorMode",
            entry.seed
        );

        assert_bits(
            selection.initial_state.x,
            &entry.initial_state.x,
            &format!("seed {} initial x", entry.seed),
        );
        assert_bits(
            selection.initial_state.y,
            &entry.initial_state.y,
            &format!("seed {} initial y", entry.seed),
        );
        assert_bits(
            selection.initial_state.z,
            &entry.initial_state.z,
            &format!("seed {} initial z", entry.seed),
        );
    }
}

#[test]
fn every_bundled_attractor_can_be_sampled() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    for descriptor in &catalog {
        for seed in 0..4 {
            let mut rng = SplitMix64::new(seed);
            randomizer::select_attractor(descriptor, &mut rng).unwrap_or_else(|error| {
                panic!(
                    "{} cannot be sampled with seed {seed}: {error}",
                    descriptor.id
                )
            });
        }
    }
}
