//! Parsed arithmetic expressions, their evaluation and their errors.
//!
//! The syntax tree landed in I1 so catalog validation could reject malformed
//! formulas, unknown functions and undeclared variables before the screen
//! saver starts. Evaluation and the golden fixtures arrived in I2.

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;

pub use crate::formula_parser::{MAX_PARSE_DEPTH, parse};

/// Error produced by formula parsing or evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaError {
    /// The formula string is empty or contains only whitespace.
    EmptyExpression,
    /// Encountered a token that cannot be part of a valid expression.
    UnexpectedToken(String),
    /// The input ended before the expression was complete.
    UnexpectedEndOfInput,
    /// A closing parenthesis without a matching opening one.
    UnbalancedParenthesis,
    /// A variable referenced by the formula has no binding during evaluation.
    UnknownVariable(String),
    /// A function name that the parser does not recognize.
    UnknownFunction(String),
    /// Evaluation hit a domain error (e.g. `sqrt` of a negative number).
    EvaluationFailed(String),
    /// The expression nests deeper than [`MAX_PARSE_DEPTH`].
    ///
    /// Rust-only safety limit: the Swift parser has no cap and overflows its
    /// stack on such input, so no formula the Swift oracle accepts is rejected
    /// here.
    TooDeep,
}

impl fmt::Display for FormulaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExpression => write!(formatter, "empty expression"),
            Self::UnexpectedToken(token) => write!(formatter, "unexpected token '{token}'"),
            Self::UnexpectedEndOfInput => write!(formatter, "unexpected end of input"),
            Self::UnbalancedParenthesis => write!(formatter, "unbalanced parenthesis"),
            Self::UnknownVariable(name) => write!(formatter, "unknown variable '{name}'"),
            Self::UnknownFunction(name) => write!(formatter, "unknown function '{name}'"),
            Self::EvaluationFailed(message) => write!(formatter, "evaluation failed: {message}"),
            Self::TooDeep => write!(
                formatter,
                "expression nests deeper than {MAX_PARSE_DEPTH} levels"
            ),
        }
    }
}

impl Error for FormulaError {}

/// A parsed arithmetic expression tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A numeric literal.
    Constant(f64),
    /// A named variable: a state coordinate or an attractor parameter.
    Variable(String),
    /// Unary negation.
    Negate(Box<Expression>),
    /// Binary addition.
    Add(Box<Expression>, Box<Expression>),
    /// Binary subtraction.
    Subtract(Box<Expression>, Box<Expression>),
    /// Binary multiplication.
    Multiply(Box<Expression>, Box<Expression>),
    /// Binary division.
    Divide(Box<Expression>, Box<Expression>),
    /// Binary exponentiation.
    Power(Box<Expression>, Box<Expression>),
    /// A unary function applied to a single argument; the name is one of
    /// [`Expression::SUPPORTED_FUNCTIONS`].
    Function(String, Box<Expression>),
}

impl Expression {
    /// Built-in unary functions available to formulas.
    pub const SUPPORTED_FUNCTIONS: [&'static str; 8] =
        ["abs", "cos", "exp", "ln", "log", "sin", "sqrt", "tan"];

    /// Names of every variable referenced anywhere in the expression tree.
    ///
    /// Sorted and deduplicated, so validation messages are stable. Used by
    /// catalog validation to reject formulas that mention variables the
    /// descriptor does not declare.
    pub fn referenced_variables(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        self.collect_variables(&mut names);
        names.into_iter().collect()
    }

    /// Names of every function called anywhere in the expression tree.
    ///
    /// Sorted and deduplicated. Used by catalog validation to reject formulas
    /// that call a function the engine does not implement; without this check
    /// such a formula only fails at evaluation time, after the screen saver
    /// has already started.
    pub fn referenced_functions(&self) -> Vec<String> {
        let mut functions = BTreeSet::new();
        self.collect_functions(&mut functions);
        functions.into_iter().collect()
    }

    /// Evaluate the expression tree against a set of variable bindings.
    ///
    /// `variables` maps a name to a value: the state coordinates `x`/`y`/`z`
    /// plus the attractor's parameters. Evaluation matches the Swift oracle:
    /// a missing binding is [`FormulaError::UnknownVariable`], division by an
    /// exactly zero divisor and the domain limits of `sqrt`/`ln`/`log` are
    /// [`FormulaError::EvaluationFailed`], and every other function is
    /// [`FormulaError::UnknownFunction`].
    pub fn evaluate(&self, variables: &HashMap<String, f64>) -> Result<f64, FormulaError> {
        match self {
            Self::Constant(value) => Ok(*value),
            Self::Variable(name) => variables
                .get(name)
                .copied()
                .ok_or_else(|| FormulaError::UnknownVariable(name.clone())),
            Self::Negate(expression) => Ok(-expression.evaluate(variables)?),
            Self::Add(lhs, rhs) => Ok(lhs.evaluate(variables)? + rhs.evaluate(variables)?),
            Self::Subtract(lhs, rhs) => Ok(lhs.evaluate(variables)? - rhs.evaluate(variables)?),
            Self::Multiply(lhs, rhs) => Ok(lhs.evaluate(variables)? * rhs.evaluate(variables)?),
            Self::Divide(lhs, rhs) => {
                // The Swift oracle evaluates the divisor first, so a zero
                // divisor wins over an error in the dividend.
                let divisor = rhs.evaluate(variables)?;
                if divisor == 0.0 {
                    return Err(FormulaError::EvaluationFailed("division by zero".into()));
                }
                Ok(lhs.evaluate(variables)? / divisor)
            }
            Self::Power(base, exponent) => Ok(base
                .evaluate(variables)?
                .powf(exponent.evaluate(variables)?)),
            Self::Function(name, argument) => {
                let value = argument.evaluate(variables)?;
                apply_function(name, value)
            }
        }
    }

    fn collect_variables(&self, names: &mut BTreeSet<String>) {
        match self {
            Self::Constant(_) => {}
            Self::Variable(name) => {
                names.insert(name.clone());
            }
            Self::Negate(expression) => expression.collect_variables(names),
            Self::Add(lhs, rhs)
            | Self::Subtract(lhs, rhs)
            | Self::Multiply(lhs, rhs)
            | Self::Divide(lhs, rhs)
            | Self::Power(lhs, rhs) => {
                lhs.collect_variables(names);
                rhs.collect_variables(names);
            }
            Self::Function(_, argument) => argument.collect_variables(names),
        }
    }

    fn collect_functions(&self, names: &mut BTreeSet<String>) {
        match self {
            Self::Constant(_) | Self::Variable(_) => {}
            Self::Negate(expression) => expression.collect_functions(names),
            Self::Add(lhs, rhs)
            | Self::Subtract(lhs, rhs)
            | Self::Multiply(lhs, rhs)
            | Self::Divide(lhs, rhs)
            | Self::Power(lhs, rhs) => {
                lhs.collect_functions(names);
                rhs.collect_functions(names);
            }
            Self::Function(name, argument) => {
                names.insert(name.clone());
                argument.collect_functions(names);
            }
        }
    }
}

/// Apply one of [`Expression::SUPPORTED_FUNCTIONS`] to an evaluated argument.
///
/// `ln` and `log` are both the natural logarithm, as in the Swift oracle.
/// The domain checks reject NaN explicitly, matching Swift's `guard`, where
/// NaN fails both `value >= 0` and `value > 0`.
fn apply_function(name: &str, value: f64) -> Result<f64, FormulaError> {
    match name {
        "sin" => Ok(value.sin()),
        "cos" => Ok(value.cos()),
        "tan" => Ok(value.tan()),
        "sqrt" => {
            if value.is_nan() || value < 0.0 {
                return Err(FormulaError::EvaluationFailed(format!(
                    "sqrt of a negative number (got {value})"
                )));
            }
            Ok(value.sqrt())
        }
        "abs" => Ok(value.abs()),
        "exp" => Ok(value.exp()),
        "ln" | "log" => {
            if value.is_nan() || value <= 0.0 {
                return Err(FormulaError::EvaluationFailed(format!(
                    "logarithm of a non-positive number (got {value})"
                )));
            }
            Ok(value.ln())
        }
        other => Err(FormulaError::UnknownFunction(other.to_owned())),
    }
}
