import Foundation

/// Error produced by formula parsing or evaluation.
public enum FormulaError: Error, Equatable {
    /// The formula string is empty or contains only whitespace.
    case emptyExpression
    /// Encountered a token that cannot be part of a valid expression.
    case unexpectedToken(String)
    /// The input ended before the expression was complete.
    case unexpectedEndOfInput
    /// A closing parenthesis without a matching opening one.
    case unbalancedParenthesis
    /// A variable referenced by the formula has no binding during evaluation.
    case unknownVariable(String)
    /// A function name that the parser does not recognize.
    case unknownFunction(String)
    /// Evaluation hit a domain error (e.g. `sqrt` of a negative number).
    case evaluationFailed(String)
}

/// A parsed arithmetic expression tree.
///
/// - Note: `indirect` is required because this value type can recurse.
public indirect enum Expression: Equatable, Sendable {
    case constant(Double)
    case variable(String)
    case negate(Expression)
    case add(Expression, Expression)
    case subtract(Expression, Expression)
    case multiply(Expression, Expression)
    case divide(Expression, Expression)
    case power(Expression, Expression)
    /// A unary function applied to a single argument; the name is one of the
    /// keys of ``Expression/supportedFunctions``.
    case function(String, Expression)
}

extension Expression {
    /// Built-in unary functions available to formulas.
    public static let supportedFunctions: Set<String> = [
        "sin", "cos", "tan", "sqrt", "abs", "exp", "ln", "log",
    ]

    /// Evaluate the expression tree against a set of variable bindings.
    ///
    /// - Parameters:
    ///   - variables: Mapping of variable name to value (the state coordinates
    ///     `x`/`y`/`z` plus the attractor's parameters).
    /// - Returns: The computed scalar value.
    /// - Throws: ``FormulaError/unknownVariable`` when a referenced variable is
    ///   not bound, or ``FormulaError/evaluationFailed`` on a domain error.
    public func evaluate(variables: [String: Double]) throws -> Double {
        switch self {
        case .constant(let value):
            return value
        case .variable(let name):
            guard let value = variables[name] else {
                throw FormulaError.unknownVariable(name)
            }
            return value
        case .negate(let expression):
            return try -expression.evaluate(variables: variables)
        case .add(let lhs, let rhs):
            return try lhs.evaluate(variables: variables) + rhs.evaluate(variables: variables)
        case .subtract(let lhs, let rhs):
            return try lhs.evaluate(variables: variables) - rhs.evaluate(variables: variables)
        case .multiply(let lhs, let rhs):
            return try lhs.evaluate(variables: variables) * rhs.evaluate(variables: variables)
        case .divide(let lhs, let rhs):
            return try lhs.evaluate(variables: variables) / rhs.evaluate(variables: variables)
        case .power(let base, let exponent):
            let b = try base.evaluate(variables: variables)
            let e = try exponent.evaluate(variables: variables)
            return pow(b, e)
        case .function(let name, let argument):
            return try applyFunction(name, to: argument, variables: variables)
        }
    }

    private func applyFunction(
        _ name: String,
        to argument: Expression,
        variables: [String: Double]
    ) throws -> Double {
        let value = try argument.evaluate(variables: variables)
        switch name {
        case "sin": return sin(value)
        case "cos": return cos(value)
        case "tan": return tan(value)
        case "sqrt":
            guard value >= 0 else {
                throw FormulaError.evaluationFailed("sqrt of a negative number (got \(value))")
            }
            return sqrt(value)
        case "abs": return Swift.abs(value)
        case "exp": return exp(value)
        case "ln", "log": return log(value)
        default:
            throw FormulaError.unknownFunction(name)
        }
    }
}
