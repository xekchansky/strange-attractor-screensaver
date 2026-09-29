import Testing
import Foundation
@testable import AttractorCore

@Suite("Formula parser")
struct ExpressionTests {
    @Test("Evaluates a constant")
    func constant() throws {
        let expression = try FormulaParser.parse("3.5")
        #expect(try expression.evaluate(variables: [:]) == 3.5)
    }

    @Test("Substitutes variables from bindings")
    func variables() throws {
        let expression = try FormulaParser.parse("a * (y - x)")
        let value = try expression.evaluate(variables: ["a": 10, "x": 1, "y": 2])
        #expect(value == 10 * (2 - 1))
    }

    @Test("Respects operator precedence")
    func precedence() throws {
        let expression = try FormulaParser.parse("2 + 3 * 4")
        #expect(try expression.evaluate(variables: [:]) == 14)
    }

    @Test("Handles parentheses and unary minus")
    func parenthesesAndUnary() throws {
        let expression = try FormulaParser.parse("-(2 + 3) * -2")
        #expect(try expression.evaluate(variables: [:]) == 10)
    }

    @Test("Power is right-associative and binds looser than unary minus")
    func powerAssociativity() throws {
        let rightAssoc = try FormulaParser.parse("2 ^ 3 ^ 2")
        #expect(try rightAssoc.evaluate(variables: [:]) == 512)
        let unary = try FormulaParser.parse("-2 ^ 2")
        #expect(try unary.evaluate(variables: [:]) == -4)
    }

    @Test("Evaluates trigonometric and math functions")
    func mathFunctions() throws {
        let sine = try FormulaParser.parse("sin(y) - b * x")
        let angle = Double.pi / 2
        let value = try sine.evaluate(variables: ["b": 0.2, "x": 1, "y": angle])
        #expect(abs(value - (1.0 - 0.2)) < 1e-9)

        let sqrtValue = try FormulaParser.parse("sqrt(9)").evaluate(variables: [:])
        #expect(sqrtValue == 3)
    }

    @Test("Throws on unknown variable during evaluation")
    func unknownVariable() throws {
        let expression = try FormulaParser.parse("q + 1")
        #expect(throws: FormulaError.self) {
            try expression.evaluate(variables: [:])
        }
    }

    @Test("Throws on empty expression")
    func emptyExpression() {
        #expect(throws: FormulaError.self) { try FormulaParser.parse("   ") }
    }

    @Test("Throws on malformed input")
    func malformedInput() {
        #expect(throws: FormulaError.self) { try FormulaParser.parse("1 + 2 )") }
        #expect(throws: FormulaError.self) { try FormulaParser.parse("2 *") }
        #expect(throws: FormulaError.self) { try FormulaParser.parse("(1 + 2") }
        #expect(throws: FormulaError.self) { try FormulaParser.parse("sin(x") }
        #expect(throws: FormulaError.self) { try FormulaParser.parse("1 2") }
    }
}
