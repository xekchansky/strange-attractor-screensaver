//! Port of the Swift `ConfigurationDecodingTests`: the catalog JSON contract.

use attractor_core::catalog::{self, CatalogError};

/// A complete, valid JSON descriptor for a Lorenz-like attractor used as the
/// shared fixture across the decoding tests below.
fn lorenz_json() -> &'static str {
    r#"
    [
      {
        "id": "lorenz",
        "name": "Lorenz",
        "dx": "a * (y - x)",
        "dy": "x * (r - z) - y",
        "dz": "x * y - b * z",
        "parameters": [
          { "name": "a", "min": 9.0, "max": 10.0 },
          { "name": "b", "min": 2.5, "max": 3.0 },
          { "name": "r", "min": 24.0, "max": 30.0 }
        ],
        "initialState": {
          "x": { "min": -1.0, "max": 1.0 },
          "y": { "min": -1.0, "max": 1.0 },
          "z": { "min": -1.0, "max": 1.0 }
        },
        "simulation": {
          "stepSize": { "min": 0.004, "max": 0.01 },
          "simulationSpeed": { "min": 1.0, "max": 3.0 },
          "cameraRotationSpeed": { "min": 0.1, "max": 0.4 },
          "trailColorSpeed": { "min": 0.05, "max": 0.3 },
          "trailLength": { "min": 2000, "max": 8000 }
        }
      }
    ]
    "#
}

/// Same fixture but without the `simulation` block, to exercise missing-section
/// decoding failure.
fn lorenz_json_without_simulation() -> &'static str {
    r#"
    [
      {
        "id": "lorenz",
        "name": "Lorenz",
        "dx": "a * (y - x)",
        "dy": "x * (r - z) - y",
        "dz": "x * y - b * z",
        "parameters": [
          { "name": "a", "min": 9.0, "max": 10.0 }
        ],
        "initialState": {
          "x": { "min": -1.0, "max": 1.0 },
          "y": { "min": -1.0, "max": 1.0 },
          "z": { "min": -1.0, "max": 1.0 }
        }
      }
    ]
    "#
}

fn decode_first(json: &str) -> attractor_core::AttractorDescriptor {
    let descriptors = catalog::decode(json).expect("fixture should decode");
    descriptors
        .into_iter()
        .next()
        .expect("fixture has one entry")
}

fn decode_error(json: &str) -> CatalogError {
    catalog::decode(json).expect_err("fixture should be rejected")
}

#[test]
fn valid_fixture_decodes_all_fields() {
    let descriptor = decode_first(lorenz_json());

    assert_eq!(descriptor.id, "lorenz");
    assert_eq!(descriptor.name, "Lorenz");
    assert_eq!(descriptor.dx, "a * (y - x)");
    assert_eq!(descriptor.dy, "x * (r - z) - y");
    assert_eq!(descriptor.dz, "x * y - b * z");

    assert_eq!(descriptor.parameters.len(), 3);
    let a = descriptor
        .parameters
        .iter()
        .find(|parameter| parameter.name == "a")
        .expect("parameter a is declared");
    assert_eq!(a.min, 9.0);
    assert_eq!(a.max, 10.0);

    let simulation = descriptor.simulation;
    assert_eq!(simulation.step_size.min, 0.004);
    assert_eq!(simulation.step_size.max, 0.01);
    assert_eq!(simulation.simulation_speed.closed_range(), 1.0..=3.0);
    assert_eq!(simulation.camera_rotation_speed.min, 0.1);
    assert_eq!(simulation.trail_color_speed.max, 0.3);
    assert_eq!(simulation.trail_length.min, 2000);
    assert_eq!(simulation.trail_length.max, 8000);
    assert_eq!(simulation.trail_length.closed_range(), 2000..=8000);

    assert_eq!(descriptor.initial_state.x.closed_range(), -1.0..=1.0);
    assert_eq!(descriptor.initial_state.y.closed_range(), -1.0..=1.0);
    assert_eq!(descriptor.initial_state.z.closed_range(), -1.0..=1.0);
}

#[test]
fn missing_required_field_throws_a_decoding_error() {
    // Drop the "dz" formula key from the fixture.
    let malformed = lorenz_json().replace(r#""dz": "x * y - b * z","#, "");
    let error = decode_error(&malformed);

    assert!(
        error.to_string().contains("dz"),
        "error should name the missing field, got: {error}"
    );
}

#[test]
fn missing_simulation_section_throws_a_decoding_error() {
    let error = decode_error(lorenz_json_without_simulation());

    assert!(
        error.to_string().contains("simulation"),
        "error should name the missing section, got: {error}"
    );
}

#[test]
fn malformed_numeric_value_throws_a_decoding_error() {
    let malformed = lorenz_json().replace(r#""min": 9.0"#, r#""min": "not-a-number""#);
    let error = decode_error(&malformed);

    assert!(
        error.to_string().contains("not-a-number") || error.to_string().contains("invalid type"),
        "error should describe the malformed number, got: {error}"
    );
}

#[test]
fn unknown_keys_are_ignored_without_failing() {
    let extended = lorenz_json().replace(
        r#""id": "lorenz","#,
        r##""id": "lorenz", "color": "#fff", "extra": 42,"##,
    );

    let descriptor = decode_first(&extended);
    assert_eq!(descriptor.id, "lorenz");
}

#[test]
fn negative_trail_length_decodes_so_validation_can_reject_one_descriptor() {
    // Swift's `Int` accepts a negative trail length; decoding must not fail
    // the whole catalog because of one hand-edited entry — `validate` drops
    // the descriptor instead.
    let negative = lorenz_json().replace(
        r#""trailLength": { "min": 2000, "max": 8000 }"#,
        r#""trailLength": { "min": -5, "max": 100 }"#,
    );

    let descriptor = decode_first(&negative);
    assert_eq!(descriptor.simulation.trail_length.min, -5);
    assert_eq!(descriptor.simulation.trail_length.max, 100);
}

#[test]
fn bundled_catalog_contains_the_reference_attractors() {
    let catalog = catalog::bundled().expect("the bundled catalog should decode");

    let ids: Vec<&str> = catalog
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "lorenz",
            "rossler",
            "thomas",
            "chen",
            "aizawa",
            "lu",
            "chua",
            "sprott_a",
            "sprott_b",
            "sprott_c",
            "halvorsen",
            "moore_spiegel",
        ]
    );

    let names: Vec<&str> = catalog
        .iter()
        .map(|descriptor| descriptor.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Lorenz",
            "Rössler",
            "Thomas",
            "Chen",
            "Aizawa",
            "Lü",
            "Chua",
            "Sprott A",
            "Sprott B",
            "Sprott C",
            "Halvorsen",
            "Moore–Spiegel",
        ]
    );

    let parameter_names = |id: &str| -> Vec<&str> {
        catalog
            .iter()
            .find(|descriptor| descriptor.id == id)
            .unwrap_or_else(|| panic!("catalog has {id}"))
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect()
    };

    assert_eq!(parameter_names("lorenz"), ["a", "b", "r"]);
    assert_eq!(parameter_names("thomas"), ["b"]);
    assert_eq!(parameter_names("chen"), ["a", "b", "c"]);
    assert_eq!(parameter_names("aizawa"), ["a", "b", "c", "d", "e", "f"]);
    assert_eq!(parameter_names("chua"), ["alpha", "beta", "m0", "m1"]);
    assert!(parameter_names("sprott_a").is_empty());
    assert_eq!(parameter_names("halvorsen"), ["a"]);
    assert_eq!(parameter_names("moore_spiegel"), ["R", "T"]);
}
