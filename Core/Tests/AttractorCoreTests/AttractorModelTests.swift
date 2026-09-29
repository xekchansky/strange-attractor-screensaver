import Testing
import Foundation
@testable import AttractorCore

@Suite("AttractorModel")
struct AttractorModelTests {
    @Test("Lorenz rate matches the reference formula at a known state")
    func lorenzRate() throws {
        let model = try AttractorModel(
            descriptor: TestSupport.lorenzDescriptor(),
            parameters: ["a": 10, "b": 8.0 / 3.0, "r": 28]
        )
        let state = State(x: 1, y: 1, z: 1)
        let derivative = try model.rate(of: state)

        // x' = 10 * (1 - 1) = 0
        #expect(abs(derivative.dx - 0.0) < 1e-9)
        // y' = 1 * (28 - 1) - 1 = 26
        #expect(abs(derivative.dy - 26.0) < 1e-9)
        // z' = 1*1 - (8/3)*1 = 1 - 8/3
        #expect(abs(derivative.dz - (1 - 8.0 / 3.0)) < 1e-9)
    }

    @Test("Thomas attractor parses and evaluates sin")
    func thomasRate() throws {
        let model = try AttractorModel(
            descriptor: TestSupport.thomasDescriptor(),
            parameters: ["b": 0.2081]
        )
        let state = State(x: 0.5, y: 0.5, z: 0.5)
        let derivative = try model.rate(of: state)

        let expected = sin(0.5) - 0.2081 * 0.5
        #expect(abs(derivative.dx - expected) < 1e-9)
        #expect(abs(derivative.dy - expected) < 1e-9)
        #expect(abs(derivative.dz - expected) < 1e-9)
    }

    @Test("Throws when a declared parameter is not supplied")
    func missingParameter() {
        let descriptor = TestSupport.lorenzDescriptor()
        #expect(throws: AttractorModelError.self) {
            try AttractorModel(descriptor: descriptor, parameters: ["a": 10])
        }
    }

    @Test("Throws when a formula cannot be parsed")
    func invalidFormula() {
        let descriptor = AttractorDescriptor(
            id: "broken",
            name: "Broken",
            dx: "sin(",  // truncated function call
            dy: "x",
            dz: "y",
            parameters: [],
            simulation: SimulationHyperparameters()
        )
        #expect(throws: FormulaError.self) {
            try AttractorModel(descriptor: descriptor, parameters: [:])
        }
    }
}
