import Foundation
import Testing
@testable import AttractorCore

@Suite("Randomizer")
struct RandomizerTests {
    private let catalog = [
        TestSupport.lorenzDescriptor(),
        TestSupport.thomasDescriptor(),
    ]

    @Test("Builds a valid model from every selection")
    func buildsValidModel() throws {
        var rng = SplitMix64(seed: 1)
        for _ in 0..<25 {
            let selection = Randomizer.select(from: catalog, using: &rng)
            _ = try selection.buildModel()
        }
    }

    @Test("Every declared parameter is covered")
    func coversAllDeclaredParameters() throws {
        var rng = SplitMix64(seed: 3)
        for _ in 0..<25 {
            let selection = Randomizer.select(from: catalog, using: &rng)
            for parameter in selection.descriptor.parameters {
                #expect(selection.parameters[parameter.name] != nil)
            }
        }
    }

    @Test("Parameter values stay within their declared ranges")
    func parameterValuesWithinRanges() throws {
        var rng = SplitMix64(seed: 7)
        for _ in 0..<25 {
            let selection = Randomizer.select(from: catalog, using: &rng)
            for parameter in selection.descriptor.parameters {
                let value = try #require(selection.parameters[parameter.name])
                #expect(value >= parameter.min)
                #expect(value <= parameter.max)
                #expect(value.isFinite)
            }
        }
    }

    @Test("Hyperparameter values stay within their declared ranges")
    func hyperparametersWithinRanges() {
        var rng = SplitMix64(seed: 11)
        for _ in 0..<25 {
            let selection = Randomizer.select(from: catalog, using: &rng)
            let sim = selection.descriptor.simulation
            #expect(selection.settings.stepSize >= sim.stepSize.min)
            #expect(selection.settings.stepSize <= sim.stepSize.max)
            #expect(selection.settings.simulationSpeed >= sim.simulationSpeed.min)
            #expect(selection.settings.simulationSpeed <= sim.simulationSpeed.max)
            #expect(selection.settings.cameraRotationSpeed >= sim.cameraRotationSpeed.min)
            #expect(selection.settings.cameraRotationSpeed <= sim.cameraRotationSpeed.max)
            #expect(selection.settings.trailColorSpeed >= sim.trailColorSpeed.min)
            #expect(selection.settings.trailColorSpeed <= sim.trailColorSpeed.max)
            #expect(selection.settings.trailLength >= sim.trailLength.min)
            #expect(selection.settings.trailLength <= sim.trailLength.max)
        }
    }

    @Test("Same seed reproduces the same selection")
    func sameSeedReproducesSelection() {
        var a = SplitMix64(seed: 99)
        let first = Randomizer.select(from: catalog, using: &a)
        var b = SplitMix64(seed: 99)
        let second = Randomizer.select(from: catalog, using: &b)
        #expect(first == second)
    }

    @Test("Varied seeds pick more than one attractor")
    func variedSeedsPickDifferentAttractors() {
        var chosen = Set<String>()
        for seed in UInt64(0)..<40 {
            var rng = SplitMix64(seed: seed)
            let selection = Randomizer.select(from: catalog, using: &rng)
            chosen.insert(selection.descriptor.id)
        }
        #expect(chosen.count > 1)
    }

    @Test("Initial state coordinates stay within their declared ranges")
    func initialStateWithinRanges() {
        var rng = SplitMix64(seed: 17)
        for _ in 0..<25 {
            let selection = Randomizer.select(from: catalog, using: &rng)
            let range = selection.descriptor.initialState
            #expect(selection.initialState.x >= range.x.min)
            #expect(selection.initialState.x <= range.x.max)
            #expect(selection.initialState.y >= range.y.min)
            #expect(selection.initialState.y <= range.y.max)
            #expect(selection.initialState.z >= range.z.min)
            #expect(selection.initialState.z <= range.z.max)
        }
    }
}
