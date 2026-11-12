//! Syntax contract for the formula parser used by catalog validation.
//!
//! Evaluation and golden fixtures belong to I2; this suite pins the grammar,
//! the AST shape and the names the parser reports.

use attractor_core::formula::{Expression, FormulaError, MAX_PARSE_DEPTH, parse};

#[test]
fn parses_a_bare_variable() {
    assert_eq!(parse("x").unwrap(), Expression::Variable("x".into()));
}

#[test]
fn parses_numbers_including_scientific_notation() {
    assert_eq!(parse("1e-3").unwrap(), Expression::Constant(0.001));
    assert_eq!(parse(".5").unwrap(), Expression::Constant(0.5));
    assert_eq!(parse("2.5E2").unwrap(), Expression::Constant(250.0));
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    assert_eq!(
        parse("1 + 2 * 3").unwrap(),
        Expression::Add(
            Box::new(Expression::Constant(1.0)),
            Box::new(Expression::Multiply(
                Box::new(Expression::Constant(2.0)),
                Box::new(Expression::Constant(3.0)),
            )),
        )
    );
}

#[test]
fn exponentiation_is_right_associative() {
    assert_eq!(
        parse("2 ^ 3 ^ 2").unwrap(),
        Expression::Power(
            Box::new(Expression::Constant(2.0)),
            Box::new(Expression::Power(
                Box::new(Expression::Constant(3.0)),
                Box::new(Expression::Constant(2.0)),
            )),
        )
    );
}

#[test]
fn unary_minus_binds_looser_than_exponentiation() {
    assert_eq!(
        parse("-2 ^ 2").unwrap(),
        Expression::Negate(Box::new(Expression::Power(
            Box::new(Expression::Constant(2.0)),
            Box::new(Expression::Constant(2.0)),
        )))
    );
}

#[test]
fn parses_function_calls() {
    assert_eq!(
        parse("sin(y) - b * x").unwrap(),
        Expression::Subtract(
            Box::new(Expression::Function(
                "sin".into(),
                Box::new(Expression::Variable("y".into())),
            )),
            Box::new(Expression::Multiply(
                Box::new(Expression::Variable("b".into())),
                Box::new(Expression::Variable("x".into())),
            )),
        )
    );
}

#[test]
fn reports_every_referenced_variable() {
    let expression = parse("a * (y - x) + z").unwrap();
    assert_eq!(expression.referenced_variables(), ["a", "x", "y", "z"]);
}

#[test]
fn reports_every_referenced_function() {
    let expression = parse("sin(x) + cos(y)").unwrap();
    assert_eq!(expression.referenced_functions(), ["cos", "sin"]);
}

#[test]
fn supported_functions_match_the_swift_table() {
    let expected = ["abs", "cos", "exp", "ln", "log", "sin", "sqrt", "tan"];
    assert_eq!(Expression::SUPPORTED_FUNCTIONS, expected);
}

#[test]
fn rejects_an_empty_formula() {
    assert_eq!(parse("   ").unwrap_err(), FormulaError::EmptyExpression);
}

#[test]
fn rejects_an_unbalanced_parenthesis() {
    assert_eq!(
        parse("a * (y - x").unwrap_err(),
        FormulaError::UnbalancedParenthesis
    );
    assert_eq!(
        parse(")").unwrap_err(),
        FormulaError::UnexpectedToken(")".into())
    );
}

#[test]
fn rejects_an_unknown_character() {
    assert_eq!(
        parse("2 $ 3").unwrap_err(),
        FormulaError::UnexpectedToken("$".into())
    );
}

#[test]
fn rejects_trailing_tokens() {
    assert_eq!(
        parse("x y").unwrap_err(),
        FormulaError::UnexpectedToken("y".into())
    );
}

#[test]
fn rejects_incomplete_input() {
    assert_eq!(
        parse("x +").unwrap_err(),
        FormulaError::UnexpectedEndOfInput
    );
    assert_eq!(
        parse("sin(").unwrap_err(),
        FormulaError::UnexpectedEndOfInput
    );
}

#[test]
fn rejects_parentheses_nested_deeper_than_the_limit() {
    let source = format!(
        "{}x{}",
        "(".repeat(MAX_PARSE_DEPTH + 1),
        ")".repeat(MAX_PARSE_DEPTH + 1)
    );
    assert_eq!(parse(&source), Err(FormulaError::TooDeep));
}

#[test]
fn rejects_unary_chains_deeper_than_the_limit() {
    let source = format!("{}x", "-".repeat(MAX_PARSE_DEPTH + 1));
    assert_eq!(parse(&source), Err(FormulaError::TooDeep));
}

#[test]
fn accepts_nesting_within_the_limit() {
    let depth = MAX_PARSE_DEPTH / 4;
    let source = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
    assert_eq!(parse(&source), Ok(Expression::Variable("x".into())));
}

#[test]
fn keeps_a_bare_e_as_a_variable() {
    // A parameter named `e` (Aizawa uses one) must tokenize on its own; only
    // `e` followed by digits is an exponent of the preceding number.
    assert_eq!(parse("e").unwrap(), Expression::Variable("e".into()));
    assert_eq!(
        parse("2 * e").unwrap(),
        Expression::Multiply(
            Box::new(Expression::Constant(2.0)),
            Box::new(Expression::Variable("e".into())),
        )
    );
}
