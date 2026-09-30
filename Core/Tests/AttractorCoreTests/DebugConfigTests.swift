import Foundation
import Testing
@testable import AttractorCore

@Suite("DebugConfig")
struct DebugConfigTests {
    private let catalog = [
        TestSupport.lorenzDescriptor(),
        TestSupport.thomasDescriptor(),
    ]

    @Test("Builds a selection for a known attractor")
    func buildsKnownAttractor() throws {
        let selection = try DebugConfig.selection(for: "lorenz", in: catalog)
        #expect(selection.descriptor.id == "lorenz")
    }

    @Test("Unknown attractor id throws")
    func unknownAttractorThrows() {
        #expect(throws: DebugConfigError.unknownAttractor("nope")) {
            _ = try DebugConfig.selection(for: "nope", in: catalog)
        }
    }

    @Test("Omitted parameters default to range midpoints")
    func omittedParametersDefaultToMidpoints() throws {
        let selection = try DebugConfig.selection(for: "lorenz", in: catalog)
        let declared = try #require(
            TestSupport.lorenzDescriptor().parameters.first { $0.name == "r" }
        )
        #expect(selection.parameters["r"] == (declared.min + declared.max) / 2)
    }

    @Test("Explicit parameters are preserved")
    func explicitParametersPreserved() throws {
        let selection = try DebugConfig.selection(for: "lorenz", in: catalog, parameters: ["r": 28])
        #expect(selection.parameters["r"] == 28)
    }

    @Test("Nil settings default to range midpoints")
    func nilSettingsDefaultToMidpoints() throws {
        let selection = try DebugConfig.selection(for: "thomas", in: catalog)
        let sim = TestSupport.thomasDescriptor().simulation
        #expect(selection.settings.trailLength == (sim.trailLength.min + sim.trailLength.max) / 2)
        #expect(selection.settings.stepSize == (sim.stepSize.min + sim.stepSize.max) / 2)
    }

    @Test("Explicit settings are preserved")
    func explicitSettingsPreserved() throws {
        let settings = SimulationSettings(
            stepSize: 0.05,
            simulationSpeed: 2,
            cameraRotationSpeed: 0.3,
            trailColorSpeed: 0.1,
            trailLength: 1234,
            colorMode: .trail
        )
        let selection = try DebugConfig.selection(for: "lorenz", in: catalog, settings: settings)
        #expect(selection.settings == settings)
    }

    @Test("Resulting model builds successfully")
    func resultingModelBuilds() throws {
        let selection = try DebugConfig.selection(for: "lorenz", in: catalog, parameters: ["r": 28])
        _ = try selection.buildModel()
    }

    @Test("Initial state is nudged off the degraded center midpoint")
    func initialStateIsOffDegenerateMidpoint() throws {
        let selection = try DebugConfig.selection(for: "thomas", in: catalog)
        let range = TestSupport.thomasDescriptor().initialState
        let midX = (range.x.min + range.x.max) / 2
        // The midpoint of a center-symmetric range is an equilibrium for Thomas;
        // DebugConfig must start off it so the simulation actually moves.
        #expect(selection.initialState.x != midX)
        #expect(selection.initialState.x > range.x.min && selection.initialState.x < range.x.max)
    }
}
