import Testing
@testable import AttractorCore

/// Build a Lorenz-like descriptor with the given overrides, so each test can
/// break exactly one rule and assert on the resulting catalog issue.
private func descriptor(
    id: String = "lorenz",
    dx: String = "a * (y - x)",
    dy: String = "x * (r - z) - y",
    dz: String = "x * y - b * z",
    parameters: [Parameter] = [
        Parameter(name: "a", min: 9, max: 10),
        Parameter(name: "b", min: 2.5, max: 3),
        Parameter(name: "r", min: 24, max: 30),
    ],
    simulation: SimulationHyperparameters = SimulationHyperparameters(),
    initialState: StateRange = StateRange(
        x: MinMax(min: -1, max: 1),
        y: MinMax(min: -1, max: 1),
        z: MinMax(min: -1, max: 1)
    )
) -> AttractorDescriptor {
    AttractorDescriptor(
        id: id,
        name: id,
        dx: dx,
        dy: dy,
        dz: dz,
        parameters: parameters,
        simulation: simulation,
        initialState: initialState
    )
}

private func hasIssue(
    _ validation: AttractorCatalogValidation,
    descriptorID: String,
    field: String
) -> Bool {
    validation.issues.contains { $0.descriptorID == descriptorID && $0.field == field }
}

@Suite("Catalog validation")
struct AttractorCatalogValidationTests {
    @Test("The bundled catalog is fully valid")
    func bundledCatalogIsValid() throws {
        let validation = try AttractorCatalog.bundledValidation()
        #expect(validation.issues.isEmpty)
        #expect(validation.valid.count == 12)
    }

    @Test("An empty catalog reports an issue and has no valid descriptors")
    func emptyCatalog() {
        let validation = AttractorCatalog.validate([])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "", field: "catalog"))
    }

    @Test("Drops a descriptor whose step size is not positive")
    func nonPositiveStepSize() {
        let broken = descriptor(
            simulation: SimulationHyperparameters(
                stepSize: MinMax(min: 0, max: 0.01)
            )
        )
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "stepSize"))
    }

    @Test("Drops a descriptor whose trail is shorter than two points")
    func tooShortTrail() {
        let broken = descriptor(
            simulation: SimulationHyperparameters(
                trailLength: MinMax(min: 1, max: 1)
            )
        )
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "trailLength"))
    }

    @Test("Drops a descriptor with an inverted range")
    func invertedRange() {
        let broken = descriptor(
            simulation: SimulationHyperparameters(
                trailLength: MinMax(min: 5000, max: 100)
            )
        )
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "trailLength"))
    }

    @Test("Drops a descriptor whose formula does not parse")
    func malformedFormula() {
        let broken = descriptor(dx: "a * (y - x")
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "dx"))
    }

    @Test("Drops a descriptor whose formula uses an unknown variable")
    func unknownVariable() {
        let broken = descriptor(dz: "x * y - q")
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "dz"))
    }

    @Test("Drops a descriptor with a duplicate parameter name")
    func duplicateParameter() {
        let broken = descriptor(parameters: [
            Parameter(name: "a", min: 9, max: 10),
            Parameter(name: "a", min: 1, max: 2),
        ])
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "parameters"))
    }

    @Test("Drops a descriptor that reuses a state coordinate as a parameter")
    func reservedParameterName() {
        let broken = descriptor(parameters: [
            Parameter(name: "x", min: 1, max: 2),
        ])
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "parameters"))
    }

    @Test("Drops a descriptor with an inverted initial-state range")
    func invertedInitialStateRange() {
        let broken = descriptor(initialState: StateRange(
            x: MinMax(min: 1, max: -1),
            y: MinMax(min: -1, max: 1),
            z: MinMax(min: -1, max: 1)
        ))
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "initialState.x"))
    }

    @Test("Drops a duplicate id and keeps the first descriptor")
    func duplicateID() {
        let first = descriptor(id: "lorenz", dx: "a * (y - x)")
        let second = descriptor(id: "lorenz", dx: "a * (x - y)")
        let validation = AttractorCatalog.validate([first, second])
        #expect(validation.valid == [first])
        #expect(hasIssue(validation, descriptorID: "lorenz", field: "id"))
    }

    @Test("Keeps valid descriptors when a sibling is broken")
    func keepsValidSiblings() {
        let broken = descriptor(id: "broken", dx: "oops(")
        let fine = descriptor(id: "fine")
        let validation = AttractorCatalog.validate([broken, fine])
        #expect(validation.valid == [fine])
        #expect(hasIssue(validation, descriptorID: "broken", field: "dx"))
    }

    @Test("Rejects formulas calling a function the parser does not support")
    func rejectsUnknownFunction() {
        // `sinh` parses as a function call but is not implemented, so without
        // this check the entry would only fail at evaluation time — after the
        // screensaver has already started.
        let broken = descriptor(id: "broken", dx: "sinh(y)")
        let validation = AttractorCatalog.validate([broken])
        #expect(validation.valid.isEmpty)
        #expect(hasIssue(validation, descriptorID: "broken", field: "dx"))
    }
}
