import Foundation

/// A lexical token produced by the formula tokenizer.
private enum Token: Equatable {
    case number(Double)
    case identifier(String)
    case plus
    case minus
    case star
    case slash
    case caret
    case leftParen
    case rightParen
}

/// Turns a formula string into an ``Expression`` tree using recursive descent.
///
/// Supported grammar:
/// - numbers (including scientific notation such as `1e-3`) and the variables
///   `x`, `y`, `z` plus attractor parameter names;
/// - binary operators `+`, `-`, `*`, `/` and right-associative `^`;
/// - unary `+`/`-`;
/// - parentheses;
/// - unary functions listed in ``Expression/supportedFunctions``.
public enum FormulaParser {
    /// Parse `source` into an expression tree.
    ///
    /// - Throws: ``FormulaError`` on any syntax problem.
    public static func parse(_ source: String) throws -> Expression {
        let tokens = try tokenize(source)
        guard !tokens.isEmpty else { throw FormulaError.emptyExpression }
        var parser = Parser(tokens: tokens)
        let expression = try parser.parse()
        try parser.expectEnd()
        return expression
    }
}

// MARK: - Tokenizer

private func tokenize(_ source: String) throws -> [Token] {
    let bytes = Array(source.utf8)
    var tokens: [Token] = []
    var i = 0

    while i < bytes.count {
        let byte = bytes[i]

        // Whitespace.
        if byte == 32 || byte == 9 || byte == 13 || byte == 10 {
            i += 1
            continue
        }

        // Number literal (digits with an optional fractional part and an
        // optional exponent).
        if isDigit(byte) || byte == 46 { // '.'
            var j = i
            var sawDigit = false
            while j < bytes.count && isDigit(bytes[j]) {
                sawDigit = true
                j += 1
            }
            if j < bytes.count && bytes[j] == 46 {
                j += 1
                while j < bytes.count && isDigit(bytes[j]) {
                    sawDigit = true
                    j += 1
                }
            }
            guard sawDigit else {
                throw FormulaError.unexpectedToken(friendly(byte))
            }
            // Exponent: consume `e`/`E` only when at least one digit follows an
            // optional sign, so a bare identifier `e` (a parameter name) still
            // tokenizes on its own.
            if j < bytes.count && (bytes[j] == 101 || bytes[j] == 69) {
                var k = j + 1
                if k < bytes.count && (bytes[k] == 43 || bytes[k] == 45) {
                    k += 1
                }
                var sawExponentDigit = false
                while k < bytes.count && isDigit(bytes[k]) {
                    sawExponentDigit = true
                    k += 1
                }
                if sawExponentDigit {
                    j = k
                }
            }
            let slice = String(decoding: bytes[i..<j], as: UTF8.self)
            guard let value = Double(slice) else {
                throw FormulaError.unexpectedToken(slice)
            }
            tokens.append(.number(value))
            i = j
            continue
        }

        // Identifier (letters, digits, underscore; must start with a letter).
        if isLetter(byte) || byte == 95 { // '_'
            var j = i
            while j < bytes.count && (isLetter(bytes[j]) || isDigit(bytes[j]) || bytes[j] == 95) {
                j += 1
            }
            let slice = String(decoding: bytes[i..<j], as: UTF8.self)
            tokens.append(.identifier(slice))
            i = j
            continue
        }

        // Operators and punctuation.
        switch byte {
        case 43: tokens.append(.plus)       // '+'
        case 45: tokens.append(.minus)      // '-'
        case 42: tokens.append(.star)       // '*'
        case 47: tokens.append(.slash)      // '/'
        case 94: tokens.append(.caret)      // '^'
        case 40: tokens.append(.leftParen)  // '('
        case 41: tokens.append(.rightParen) // ')'
        default:
            throw FormulaError.unexpectedToken(friendly(byte))
        }
        i += 1
    }

    return tokens
}

private func isDigit(_ byte: UInt8) -> Bool { byte >= 48 && byte <= 57 }

private func isLetter(_ byte: UInt8) -> Bool {
    (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122)
}

private func friendly(_ byte: UInt8) -> String {
    String(UnicodeScalar(byte))
}

// MARK: - Parser

private struct Parser {
    let tokens: [Token]
    private var index = 0

    init(tokens: [Token]) {
        self.tokens = tokens
    }

    private var current: Token? {
        index < tokens.count ? tokens[index] : nil
    }

    mutating func parse() throws -> Expression {
        try parseExpression()
    }

    /// Ensure no tokens remain unconsumed.
    mutating func expectEnd() throws {
        if let token = current {
            throw FormulaError.unexpectedToken(describe(token))
        }
    }

    /// `e := t (('+' | '-') t)*`
    private mutating func parseExpression() throws -> Expression {
        var lhs = try parseTerm()
        while let token = current {
            switch token {
            case .plus:
                index += 1
                lhs = .add(lhs, try parseTerm())
            case .minus:
                index += 1
                lhs = .subtract(lhs, try parseTerm())
            default:
                return lhs
            }
        }
        return lhs
    }

    /// `t := u (('*' | '/') u)*`
    private mutating func parseTerm() throws -> Expression {
        var lhs = try parseUnary()
        while let token = current {
            switch token {
            case .star:
                index += 1
                lhs = .multiply(lhs, try parseUnary())
            case .slash:
                index += 1
                lhs = .divide(lhs, try parseUnary())
            default:
                return lhs
            }
        }
        return lhs
    }

    /// `u := ('+' | '-') u | p` — unary sign binds looser than `^`.
    private mutating func parseUnary() throws -> Expression {
        guard let token = current else {
            throw FormulaError.unexpectedEndOfInput
        }
        switch token {
        case .plus:
            index += 1
            return try parseUnary()
        case .minus:
            index += 1
            return .negate(try parseUnary())
        default:
            return try parsePower()
        }
    }

    /// `p := primary ('^' u)?` — right-associative exponentiation.
    private mutating func parsePower() throws -> Expression {
        let base = try parsePrimary()
        if current == .caret {
            index += 1
            let exponent = try parseUnary()
            return .power(base, exponent)
        }
        return base
    }

    private mutating func parsePrimary() throws -> Expression {
        guard let token = current else {
            throw FormulaError.unexpectedEndOfInput
        }
        switch token {
        case .number(let value):
            index += 1
            return .constant(value)
        case .identifier(let name):
            index += 1
            if current == .leftParen {
                index += 1
                let argument = try parseExpression()
                guard current == .rightParen else {
                    throw FormulaError.unbalancedParenthesis
                }
                index += 1
                return .function(name, argument)
            }
            return .variable(name)
        case .leftParen:
            index += 1
            let inner = try parseExpression()
            guard current == .rightParen else {
                throw FormulaError.unbalancedParenthesis
            }
            index += 1
            return inner
        default:
            throw FormulaError.unexpectedToken(describe(token))
        }
    }
}

private func describe(_ token: Token) -> String {
    switch token {
    case .number(let value): return String(value)
    case .identifier(let name): return name
    case .plus: return "+"
    case .minus: return "-"
    case .star: return "*"
    case .slash: return "/"
    case .caret: return "^"
    case .leftParen: return "("
    case .rightParen: return ")"
    }
}
