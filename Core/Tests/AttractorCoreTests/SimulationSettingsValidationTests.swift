import Testing
@testable import AttractorCore

private func settings(
    stepSize: Double = 0.006,
    simulationSpeed: Double = 2,
    cameraRotationSpeed: Double = 0.2,
    trailColorSpeed: Double = 0.1,
    trailLength: Int = 4_000
) -> SimulationSettings {
    SimulationSettings(
        stepSize: stepSize,
        simulationSpeed: simulationSpeed,
        cameraRotationSpeed: cameraRotationSpeed,
        trailColorSpeed: trailColorSpeed,
        trailLength: trailLength,
        colorMode: .trail
    )
}

private func mentions(_ issues: [String], _ field: String) -> Bool {
    issues.contains { $0.contains(field) }
}

@Suite("Simulation settings validation")
struct SimulationSettingsValidationTests {
    @Test("Debug midpoints are valid")
    func defaultsAreValid() {
        let issues = SimulationSettings(midpointsOf: SimulationHyperparameters()).validationIssues
        #expect(issues.isEmpty)
    }

    @Test("Rejects a non-positive step size")
    func nonPositiveStepSize() {
        #expect(mentions(settings(stepSize: 0).validationIssues, "stepSize"))
        #expect(mentions(settings(stepSize: -0.01).validationIssues, "stepSize"))
    }

    @Test("Rejects a non-finite step size")
    func nonFiniteStepSize() {
        #expect(mentions(settings(stepSize: .nan).validationIssues, "stepSize"))
        #expect(mentions(settings(stepSize: .infinity).validationIssues, "stepSize"))
    }

    @Test("Rejects a negative or non-finite simulation speed")
    func invalidSimulationSpeed() {
        #expect(mentions(settings(simulationSpeed: -1).validationIssues, "simulationSpeed"))
        #expect(mentions(settings(simulationSpeed: .nan).validationIssues, "simulationSpeed"))
        // Zero is allowed: it freezes the run, which is useful for debugging.
        #expect(!mentions(settings(simulationSpeed: 0).validationIssues, "simulationSpeed"))
    }

    @Test("Rejects negative or non-finite camera rotation speed")
    func invalidCameraRotationSpeed() {
        #expect(mentions(settings(cameraRotationSpeed: -0.1).validationIssues, "cameraRotationSpeed"))
        #expect(mentions(settings(cameraRotationSpeed: .infinity).validationIssues, "cameraRotationSpeed"))
        #expect(!mentions(settings(cameraRotationSpeed: 0).validationIssues, "cameraRotationSpeed"))
    }

    @Test("Rejects negative or non-finite trail color speed")
    func invalidTrailColorSpeed() {
        #expect(mentions(settings(trailColorSpeed: -0.1).validationIssues, "trailColorSpeed"))
        #expect(mentions(settings(trailColorSpeed: .nan).validationIssues, "trailColorSpeed"))
        #expect(!mentions(settings(trailColorSpeed: 0).validationIssues, "trailColorSpeed"))
    }

    @Test("Rejects a trail shorter than two points")
    func tooShortTrail() {
        #expect(mentions(settings(trailLength: 1).validationIssues, "trailLength"))
        #expect(mentions(settings(trailLength: 0).validationIssues, "trailLength"))
        #expect(!mentions(settings(trailLength: 2).validationIssues, "trailLength"))
    }
}
