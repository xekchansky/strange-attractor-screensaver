//! Golden fixtures produced by the Swift oracle.
//!
//! `tests/fixtures/formula_evaluation.json` holds every catalog formula
//! evaluated at eight sample states. The generator lives in
//! `Core/Tests/AttractorCoreTests/FormulaFixtureDumpTests.swift`; see
//! `tests/fixtures/README.md` for the command that reproduces it.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use attractor_core::catalog;
use attractor_core::formula::parse;
use serde::Deserialize;

const STATES_PER_FORMULA: usize = 8;
const TOLERANCE: f64 = 1e-12;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    entries: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    attractor: String,
    field: String,
    expression: String,
    bindings: HashMap<String, String>,
    expected: String,
}

fn load_fixtures() -> FixtureFile {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/formula_evaluation.json");
    let json = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}; regenerate it with the command in tests/fixtures/README.md",
            path.display()
        )
    });
    serde_json::from_str(&json).expect("fixture file is valid JSON")
}

fn parse_value(text: &str, context: &str) -> f64 {
    text.parse::<f64>()
        .unwrap_or_else(|error| panic!("{context}: cannot parse '{text}' as f64: {error}"))
}

#[test]
fn fixture_covers_every_catalog_formula_eight_times() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    let fixtures = load_fixtures();

    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for entry in &fixtures.entries {
        *seen
            .entry((entry.attractor.clone(), entry.field.clone()))
            .or_default() += 1;
    }

    let expected_formulas = catalog.len() * 3;
    assert_eq!(
        seen.len(),
        expected_formulas,
        "each catalog attractor contributes dx, dy and dz"
    );
    assert_eq!(
        fixtures.entries.len(),
        expected_formulas * STATES_PER_FORMULA
    );

    for descriptor in &catalog {
        for field in ["dx", "dy", "dz"] {
            let key = (descriptor.id.clone(), field.to_owned());
            assert_eq!(
                seen.get(&key),
                Some(&STATES_PER_FORMULA),
                "fixture is missing states for {}.{field}",
                descriptor.id
            );
        }
    }
}

#[test]
fn fixture_expressions_match_the_catalog() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    let fixtures = load_fixtures();

    let sources: HashMap<(&str, &str), &str> = catalog
        .iter()
        .flat_map(|descriptor| {
            [
                (descriptor.id.as_str(), "dx", descriptor.dx.as_str()),
                (descriptor.id.as_str(), "dy", descriptor.dy.as_str()),
                (descriptor.id.as_str(), "dz", descriptor.dz.as_str()),
            ]
        })
        .map(|(id, field, source)| ((id, field), source))
        .collect();

    for entry in &fixtures.entries {
        let key = (entry.attractor.as_str(), entry.field.as_str());
        let source = sources.get(&key).unwrap_or_else(|| {
            panic!(
                "fixture references {}.{} which is not in the catalog",
                entry.attractor, entry.field
            )
        });
        assert_eq!(
            entry.expression, *source,
            "fixture expression for {}.{} drifted from the catalog",
            entry.attractor, entry.field
        );
    }
}

#[test]
fn fixture_bindings_bind_the_state_and_every_parameter() {
    let catalog = catalog::bundled().expect("the embedded catalog decodes");
    let fixtures = load_fixtures();
    let parameters: HashMap<&str, HashSet<&str>> = catalog
        .iter()
        .map(|descriptor| {
            (
                descriptor.id.as_str(),
                descriptor
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.as_str())
                    .collect(),
            )
        })
        .collect();

    for entry in &fixtures.entries {
        for coordinate in ["x", "y", "z"] {
            assert!(
                entry.bindings.contains_key(coordinate),
                "{}.{} fixture does not bind {coordinate}",
                entry.attractor,
                entry.field
            );
        }
        let declared = &parameters[entry.attractor.as_str()];
        let bound: HashSet<&str> = entry.bindings.keys().map(String::as_str).collect();
        assert_eq!(
            &bound,
            &declared
                .iter()
                .copied()
                .chain(["x", "y", "z"])
                .collect::<HashSet<_>>(),
            "{}.{} fixture binds the wrong variables",
            entry.attractor,
            entry.field
        );
    }
}

#[test]
fn fixture_values_match_the_rust_evaluator() {
    let fixtures = load_fixtures();

    for entry in &fixtures.entries {
        let expression = parse(&entry.expression).unwrap_or_else(|error| {
            panic!(
                "fixture expression for {}.{} does not parse: {error}",
                entry.attractor, entry.field
            )
        });
        let variables: HashMap<String, f64> = entry
            .bindings
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    parse_value(
                        value,
                        &format!("{}.{} binding {name}", entry.attractor, entry.field),
                    ),
                )
            })
            .collect();
        let expected = parse_value(
            &entry.expected,
            &format!("{}.{} expected value", entry.attractor, entry.field),
        );

        let actual = expression.evaluate(&variables).unwrap_or_else(|error| {
            panic!(
                "evaluating {}.{} failed: {error}",
                entry.attractor, entry.field
            )
        });
        let scale = expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= TOLERANCE * scale,
            "{}.{} = {actual}, oracle says {expected}",
            entry.attractor,
            entry.field
        );
    }
}
