//! Evaluation contract for formula expressions.
//!
//! Ports the Swift `ExpressionTests` and pins the error precedence and domain
//! rules the oracle follows, so the two evaluators stay interchangeable.

use std::collections::HashMap;

use attractor_core::formula::{FormulaError, parse};

fn bindings(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), *value))
        .collect()
}

fn evaluate(source: &str, variables: &[(&str, f64)]) -> Result<f64, FormulaError> {
    parse(source).unwrap().evaluate(&bindings(variables))
}

#[test]
fn evaluates_a_constant() {
    assert_eq!(evaluate("3.5", &[]).unwrap(), 3.5);
}

#[test]
fn substitutes_variables_from_bindings() {
    let value = evaluate("a * (y - x)", &[("a", 10.0), ("x", 1.0), ("y", 2.0)]).unwrap();
    assert_eq!(value, 10.0 * (2.0 - 1.0));
}

#[test]
fn respects_operator_precedence() {
    assert_eq!(evaluate("2 + 3 * 4", &[]).unwrap(), 14.0);
}

#[test]
fn handles_parentheses_and_unary_minus() {
    assert_eq!(evaluate("-(2 + 3) * -2", &[]).unwrap(), 10.0);
}

#[test]
fn power_is_right_associative_and_binds_tighter_than_unary_minus() {
    assert_eq!(evaluate("2 ^ 3 ^ 2", &[]).unwrap(), 512.0);
    assert_eq!(evaluate("-2 ^ 2", &[]).unwrap(), -4.0);
}

#[test]
fn evaluates_trigonometric_and_math_functions() {
    let value = evaluate(
        "sin(y) - b * x",
        &[("b", 0.2), ("x", 1.0), ("y", std::f64::consts::FRAC_PI_2)],
    )
    .unwrap();
    assert!((value - (1.0 - 0.2)).abs() < 1e-9);
    assert_eq!(evaluate("sqrt(9)", &[]).unwrap(), 3.0);
}

#[test]
fn evaluates_every_supported_function() {
    assert_eq!(evaluate("abs(x)", &[("x", -2.5)]).unwrap(), 2.5);
    assert_eq!(evaluate("exp(x)", &[("x", 0.0)]).unwrap(), 1.0);
    assert_eq!(evaluate("cos(x)", &[("x", 0.0)]).unwrap(), 1.0);
    assert_eq!(evaluate("tan(x)", &[("x", 0.0)]).unwrap(), 0.0);
    assert_eq!(evaluate("sin(x)", &[("x", 0.0)]).unwrap(), 0.0);
    assert_eq!(evaluate("sqrt(x)", &[("x", 16.0)]).unwrap(), 4.0);
    // `ln` and `log` are both the natural logarithm.
    assert!((evaluate("ln(x)", &[("x", std::f64::consts::E)]).unwrap() - 1.0).abs() < 1e-12);
    assert!(
        (evaluate("log(x)", &[("x", std::f64::consts::E.powi(3))]).unwrap() - 3.0).abs() < 1e-12
    );
}

#[test]
fn parses_scientific_notation() {
    assert_eq!(evaluate("1e-3 + 2.5E+2", &[]).unwrap(), 0.001 + 250.0);
    assert_eq!(evaluate("x * 1e-6", &[("x", 1_000_000.0)]).unwrap(), 1.0);
}

#[test]
fn reports_an_unknown_variable() {
    assert_eq!(
        evaluate("q + 1", &[]),
        Err(FormulaError::UnknownVariable("q".into()))
    );
}

#[test]
fn reports_an_unknown_function_at_evaluation() {
    // The parser accepts any identifier in call position; catalog validation
    // and evaluation are what reject names outside the function table.
    assert_eq!(
        evaluate("tanh(x)", &[("x", 1.0)]),
        Err(FormulaError::UnknownFunction("tanh".into()))
    );
}

#[test]
fn a_logarithm_of_a_non_positive_number_is_an_evaluation_error() {
    assert!(matches!(
        evaluate("ln(x)", &[("x", 0.0)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
    assert!(matches!(
        evaluate("ln(x)", &[("x", -2.0)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
    assert!(matches!(
        evaluate("log(x)", &[("x", 0.0)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
}

#[test]
fn a_square_root_of_a_negative_number_is_an_evaluation_error() {
    assert!(matches!(
        evaluate("sqrt(x)", &[("x", -1.0)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
}

#[test]
fn a_square_root_of_nan_is_an_evaluation_error() {
    // The Swift guard is `value >= 0`, which NaN fails.
    assert!(matches!(
        evaluate("sqrt(x)", &[("x", f64::NAN)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
}

#[test]
fn division_by_zero_is_an_evaluation_error_instead_of_infinity() {
    assert!(matches!(
        evaluate("1 / x", &[("x", 0.0)]),
        Err(FormulaError::EvaluationFailed(_))
    ));
}

#[test]
fn division_checks_the_divisor_before_the_dividend() {
    // Swift evaluates the right-hand side first, so the zero wins over the
    // unknown variable on the left.
    assert!(matches!(
        evaluate("q / 0", &[]),
        Err(FormulaError::EvaluationFailed(_))
    ));
}

#[test]
fn a_negative_base_with_a_fractional_exponent_yields_nan() {
    // `pow` has no domain check in the oracle; it returns NaN.
    assert!(evaluate("(-1) ^ 0.5", &[]).unwrap().is_nan());
}
