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

    /// Names of every variable referenced anywhere in the expression tree.
    ///
    /// Used by catalog validation to reject formulas that mention variables the
    /// descriptor does not declare.
    public var referencedVariables: Set<String> {
        switch self {
        case .constant:
            return []
        case .variable(let name):
            return [name]
        case .negate(let expression):
            return expression.referencedVariables
        case .add(let lhs, let rhs), .subtract(let lhs, let rhs),
             .multiply(let lhs, let rhs), .divide(let lhs, let rhs),
             .power(let lhs, let rhs):
            return lhs.referencedVariables.union(rhs.referencedVariables)
        case .function(_, let argument):
            return argument.referencedVariables
        }
    }

    /// Names of every function called anywhere in the expression tree.
    ///
    /// Used by catalog validation to reject formulas that call a function the
    /// parser does not implement; without this check such a formula only fails
    /// at evaluation time, after the screensaver has already started.
    public var referencedFunctions: Set<String> {
        switch self {
        case .constant, .variable:
            return []
        case .negate(let expression):
            return expression.referencedFunctions
        case .add(let lhs, let rhs), .subtract(let lhs, let rhs),
             .multiply(let lhs, let rhs), .divide(let lhs, let rhs),
             .power(let lhs, let rhs):
            return lhs.referencedFunctions.union(rhs.referencedFunctions)
        case .function(let name, let argument):
            return Set([name]).union(argument.referencedFunctions)
        }
    }

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
            let divisor = try rhs.evaluate(variables: variables)
            guard divisor != 0 else {
                throw FormulaError.evaluationFailed("division by zero")
            }
            return try lhs.evaluate(variables: variables) / divisor
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
        case "ln", "log":
            guard value > 0 else {
                throw FormulaError.evaluationFailed(
                    "logarithm of a non-positive number (got \(value))"
                )
            }
            return log(value)
        default:
            throw FormulaError.unknownFunction(name)
        }
    }
}
