//! Loading and validating the attractor catalog.
//!
//! The catalog is the single source describing every available attractor: the
//! randomizer and the debug tooling select from it at startup, and adding an
//! attractor never requires touching code — only extending the JSON resource,
//! which is embedded with [`include_str!`].

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::AttractorDescriptor;
use crate::formula::Expression;

/// The bundled catalog, embedded at compile time.
///
/// `data/attractors.json` at the repository root is the single source of truth
/// for every platform, including the web demo and the Android wallpaper.
pub const BUNDLED_JSON: &str = include_str!("../../../../data/attractors.json");

/// Error produced while loading the catalog.
#[derive(Debug)]
pub enum CatalogError {
    /// The JSON text does not match the descriptor schema.
    Decode(serde_json::Error),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "cannot decode attractor catalog: {error}"),
        }
    }
}

impl Error for CatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for CatalogError {
    fn from(error: serde_json::Error) -> Self {
        Self::Decode(error)
    }
}

/// Decode a catalog document.
///
/// Descriptors keep the order they appear in the file. Unknown JSON keys are
/// ignored, matching the Swift `Codable` behaviour.
pub fn decode(json: &str) -> Result<Vec<AttractorDescriptor>, CatalogError> {
    Ok(serde_json::from_str(json)?)
}

/// Decode the bundled attractor catalog.
pub fn bundled() -> Result<Vec<AttractorDescriptor>, CatalogError> {
    decode(BUNDLED_JSON)
}

/// One problem found while validating an attractor descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttractorValidationIssue {
    /// `id` of the descriptor the issue belongs to (empty for catalog-wide
    /// issues).
    pub descriptor_id: String,
    /// Field the issue is about, e.g. `"dx"`, `"stepSize"`, `"parameters"`.
    pub field: String,
    /// Human-readable description of the problem.
    pub message: String,
}

impl AttractorValidationIssue {
    fn new(
        descriptor_id: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            descriptor_id: descriptor_id.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Result of validating a catalog: descriptors safe to run and the issues that
/// caused the rest to be dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct AttractorCatalogValidation {
    /// Descriptors that passed every check, in catalog order.
    pub valid: Vec<AttractorDescriptor>,
    /// Problems found in the rejected descriptors (plus catalog-wide issues).
    pub issues: Vec<AttractorValidationIssue>,
}

/// Decode the bundled catalog and validate it in one step.
pub fn bundled_validation() -> Result<AttractorCatalogValidation, CatalogError> {
    Ok(validate(&bundled()?))
}

/// Check every descriptor and keep only the ones that can be run safely.
///
/// A single malformed entry must not blank the whole screen saver, so invalid
/// descriptors are dropped and reported instead of aborting the catalog.
/// Checks cover range sanity, duplicate ids and parameters, formula syntax,
/// functions the engine does not implement, and formula variables that the
/// descriptor does not declare.
pub fn validate(catalog: &[AttractorDescriptor]) -> AttractorCatalogValidation {
    let mut valid = Vec::new();
    let mut issues = Vec::new();
    let mut seen_ids = HashSet::new();

    if catalog.is_empty() {
        issues.push(AttractorValidationIssue::new(
            "",
            "catalog",
            "catalog is empty",
        ));
    }

    for descriptor in catalog {
        let descriptor_issues = validation_issues(descriptor, &mut seen_ids);
        if descriptor_issues.is_empty() {
            valid.push(descriptor.clone());
        } else {
            issues.extend(descriptor_issues);
        }
    }

    AttractorCatalogValidation { valid, issues }
}

/// Variables every formula may reference in addition to declared parameters.
const STATE_VARIABLES: [&str; 3] = ["x", "y", "z"];

fn is_state_variable(name: &str) -> bool {
    STATE_VARIABLES.contains(&name)
}

fn validation_issues(
    descriptor: &AttractorDescriptor,
    seen_ids: &mut HashSet<String>,
) -> Vec<AttractorValidationIssue> {
    let mut issues = Vec::new();
    let id = descriptor.id.trim();

    if id.is_empty() {
        issues.push(issue(descriptor, "id", "id must not be empty"));
    } else if !seen_ids.insert(id.to_owned()) {
        issues.push(issue(descriptor, "id", format!("duplicate id '{id}'")));
    }

    let mut parameter_names = HashSet::new();
    for parameter in &descriptor.parameters {
        let name = parameter.name.as_str();
        if name.is_empty() {
            issues.push(issue(
                descriptor,
                "parameters",
                "parameter name must not be empty",
            ));
            continue;
        }
        if is_state_variable(name) {
            issues.push(issue(
                descriptor,
                "parameters",
                format!("parameter '{name}' collides with a state coordinate"),
            ));
        }
        if !parameter_names.insert(name.to_owned()) {
            issues.push(issue(
                descriptor,
                "parameters",
                format!("duplicate parameter '{name}'"),
            ));
        }
        append_double_range_issues(
            &mut issues,
            descriptor,
            &format!("parameters.{name}"),
            parameter.min,
            parameter.max,
            None,
        );
    }

    let mut allowed_variables: HashSet<&str> = STATE_VARIABLES.into_iter().collect();
    allowed_variables.extend(parameter_names.iter().map(String::as_str));
    append_formula_issues(
        &mut issues,
        descriptor,
        "dx",
        &descriptor.dx,
        &allowed_variables,
    );
    append_formula_issues(
        &mut issues,
        descriptor,
        "dy",
        &descriptor.dy,
        &allowed_variables,
    );
    append_formula_issues(
        &mut issues,
        descriptor,
        "dz",
        &descriptor.dz,
        &allowed_variables,
    );

    let simulation = &descriptor.simulation;
    append_double_range_issues(
        &mut issues,
        descriptor,
        "stepSize",
        simulation.step_size.min,
        simulation.step_size.max,
        Some(LowerBound {
            value: 0.0,
            exclusive: true,
        }),
    );
    append_double_range_issues(
        &mut issues,
        descriptor,
        "simulationSpeed",
        simulation.simulation_speed.min,
        simulation.simulation_speed.max,
        Some(LowerBound {
            value: 0.0,
            exclusive: true,
        }),
    );
    append_double_range_issues(
        &mut issues,
        descriptor,
        "cameraRotationSpeed",
        simulation.camera_rotation_speed.min,
        simulation.camera_rotation_speed.max,
        Some(LowerBound {
            value: 0.0,
            exclusive: false,
        }),
    );
    append_double_range_issues(
        &mut issues,
        descriptor,
        "trailColorSpeed",
        simulation.trail_color_speed.min,
        simulation.trail_color_speed.max,
        Some(LowerBound {
            value: 0.0,
            exclusive: false,
        }),
    );
    if simulation.trail_length.min > simulation.trail_length.max {
        issues.push(issue(
            descriptor,
            "trailLength",
            format!(
                "min {} is greater than max {}",
                simulation.trail_length.min, simulation.trail_length.max
            ),
        ));
    }
    if simulation.trail_length.min < 2 {
        issues.push(issue(
            descriptor,
            "trailLength",
            "must keep at least 2 points",
        ));
    }

    append_double_range_issues(
        &mut issues,
        descriptor,
        "initialState.x",
        descriptor.initial_state.x.min,
        descriptor.initial_state.x.max,
        None,
    );
    append_double_range_issues(
        &mut issues,
        descriptor,
        "initialState.y",
        descriptor.initial_state.y.min,
        descriptor.initial_state.y.max,
        None,
    );
    append_double_range_issues(
        &mut issues,
        descriptor,
        "initialState.z",
        descriptor.initial_state.z.min,
        descriptor.initial_state.z.max,
        None,
    );

    issues
}

fn issue(
    descriptor: &AttractorDescriptor,
    field: impl Into<String>,
    message: impl Into<String>,
) -> AttractorValidationIssue {
    AttractorValidationIssue::new(descriptor.id.clone(), field, message)
}

/// An optional lower bound a range's minimum must respect.
struct LowerBound {
    value: f64,
    exclusive: bool,
}

fn append_double_range_issues(
    issues: &mut Vec<AttractorValidationIssue>,
    descriptor: &AttractorDescriptor,
    field: &str,
    min: f64,
    max: f64,
    lower_bound: Option<LowerBound>,
) {
    if !min.is_finite() || !max.is_finite() {
        issues.push(issue(descriptor, field, "bounds must be finite"));
        return;
    }
    if min > max {
        issues.push(issue(
            descriptor,
            field,
            format!("min {min} is greater than max {max}"),
        ));
        return;
    }
    if let Some(lower_bound) = lower_bound {
        let violates = if lower_bound.exclusive {
            min <= lower_bound.value
        } else {
            min < lower_bound.value
        };
        if violates {
            let value = lower_bound.value;
            issues.push(issue(
                descriptor,
                field,
                if lower_bound.exclusive {
                    format!("min {min} must be greater than {value}")
                } else {
                    format!("min {min} must be at least {value}")
                },
            ));
        }
    }
}

fn append_formula_issues(
    issues: &mut Vec<AttractorValidationIssue>,
    descriptor: &AttractorDescriptor,
    field: &str,
    formula: &str,
    allowed_variables: &HashSet<&str>,
) {
    let expression = match crate::formula::parse(formula) {
        Ok(expression) => expression,
        Err(error) => {
            issues.push(issue(
                descriptor,
                field,
                format!("cannot parse formula: {error}"),
            ));
            return;
        }
    };

    let unknown_variables: Vec<String> = expression
        .referenced_variables()
        .into_iter()
        .filter(|name| !allowed_variables.contains(name.as_str()))
        .collect();
    if !unknown_variables.is_empty() {
        issues.push(issue(
            descriptor,
            field,
            format!("unknown variable(s): {}", unknown_variables.join(", ")),
        ));
    }

    let unknown_functions: Vec<String> = expression
        .referenced_functions()
        .into_iter()
        .filter(|name| !Expression::SUPPORTED_FUNCTIONS.contains(&name.as_str()))
        .collect();
    if !unknown_functions.is_empty() {
        issues.push(issue(
            descriptor,
            field,
            format!("unknown function(s): {}", unknown_functions.join(", ")),
        ));
    }
}
