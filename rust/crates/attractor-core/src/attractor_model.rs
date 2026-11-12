//! An attractor's formulas parsed into expression trees and bound to concrete
//! parameter values.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::formula::{self, Expression, FormulaError};
use crate::{AttractorDescriptor, Derivatives, State};

/// Error produced while building an [`AttractorModel`].
#[derive(Debug, Clone, PartialEq)]
pub enum AttractorModelError {
    /// A parameter declared in the descriptor has no supplied value.
    MissingParameter(String),
    /// One of the formulas cannot be parsed.
    Formula(FormulaError),
}

impl fmt::Display for AttractorModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingParameter(name) => write!(formatter, "missing parameter '{name}'"),
            Self::Formula(error) => write!(formatter, "cannot parse formula: {error}"),
        }
    }
}

impl Error for AttractorModelError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MissingParameter(_) => None,
            Self::Formula(error) => Some(error),
        }
    }
}

impl From<FormulaError> for AttractorModelError {
    fn from(error: FormulaError) -> Self {
        Self::Formula(error)
    }
}

/// A concrete, evaluable attractor: the `dx`/`dy`/`dz` formulas parsed into
/// expression trees and bound to the state coordinates and a fixed set of
/// parameter values.
#[derive(Debug, Clone, PartialEq)]
pub struct AttractorModel {
    /// The descriptor the model was built from.
    pub descriptor: AttractorDescriptor,
    /// Concrete parameter values keyed by the parameter name.
    pub parameters: HashMap<String, f64>,
    dx: Expression,
    dy: Expression,
    dz: Expression,
}

impl AttractorModel {
    /// Build an evaluable model from a descriptor and concrete parameters.
    ///
    /// Every parameter declared in `descriptor` must be present in
    /// `parameters`; a formula that cannot be parsed is reported as
    /// [`AttractorModelError::Formula`].
    pub fn new(
        descriptor: AttractorDescriptor,
        parameters: HashMap<String, f64>,
    ) -> Result<Self, AttractorModelError> {
        for parameter in &descriptor.parameters {
            if !parameters.contains_key(&parameter.name) {
                return Err(AttractorModelError::MissingParameter(
                    parameter.name.clone(),
                ));
            }
        }

        let dx = formula::parse(&descriptor.dx)?;
        let dy = formula::parse(&descriptor.dy)?;
        let dz = formula::parse(&descriptor.dz)?;
        Ok(Self {
            descriptor,
            parameters,
            dx,
            dy,
            dz,
        })
    }

    /// The vector field evaluated at `state`.
    ///
    /// The parameters stored in the model, together with the coordinates of
    /// `state`, are available as variables in the formulas.
    pub fn rate(&self, state: State) -> Result<Derivatives, FormulaError> {
        let mut variables = self.parameters.clone();
        variables.insert("x".to_string(), state.x);
        variables.insert("y".to_string(), state.y);
        variables.insert("z".to_string(), state.z);
        Ok(Derivatives::new(
            self.dx.evaluate(&variables)?,
            self.dy.evaluate(&variables)?,
            self.dz.evaluate(&variables)?,
        ))
    }
}
