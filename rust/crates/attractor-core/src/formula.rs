//! Parsed arithmetic expressions and their syntax errors.
//!
//! Evaluation is added in I2 together with the golden fixtures; I1 needs the
//! syntax tree only so catalog validation can reject malformed formulas,
//! unknown functions and undeclared variables before the screen saver starts.

use std::collections::BTreeSet;
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
