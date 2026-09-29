import Testing
import Foundation
@testable import AttractorCore

@Suite("AttractorEngine")
struct AttractorEngineTests {
    private func makeEngine(trailCapacity: Int) throws -> AttractorEngine {
        let model = try AttractorModel(
            descriptor: TestSupport.lorenzDescriptor(),
            parameters: ["a": 10, "b": 8.0 / 3.0, "r": 28]
        )
        return AttractorEngine(model: model, method: .rungeKutta4, stepSize: 0.01, trailCapacity: trailCapacity)
    }

    @Test("Starts with the initial state as the sole trace point")
    func initial() throws {
        let engine = try makeEngine(trailCapacity: 50)
        #expect(engine.trace.count == 1)
        #expect(engine.state == engine.trace.first)
    }

    @Test("Advancing moves the state and grows the trace")
    func advances() throws {
        var engine = try makeEngine(trailCapacity: 2000)
        let initial = engine.state
        try engine.advance()
        #expect(engine.state != initial)
        #expect(engine.trace.count == 2)
        #expect(engine.state == engine.trace.last)
    }

    @Test("Trace is bounded by the trail capacity")
    func boundedTrace() throws {
        var engine = try makeEngine(trailCapacity: 50)
        for _ in 0..<100 {
            try engine.advance()
        }
        #expect(engine.trace.count == 50)
        #expect(engine.state == engine.trace.last)
    }

    @Test("Produced trace points are finite")
    func finite() throws {
        var engine = try makeEngine(trailCapacity: 100)
        for _ in 0..<50 {
            try engine.advance()
        }
        for point in engine.trace {
            #expect(point.x.isFinite)
            #expect(point.y.isFinite)
            #expect(point.z.isFinite)
        }
    }

    @Test("Euler and RK4 engines stay finite on the Lorenz system")
    func stableLorenz() throws {
        let model = try AttractorModel(
            descriptor: TestSupport.lorenzDescriptor(),
            parameters: ["a": 10, "b": 8.0 / 3.0, "r": 28]
        )
        for method in [IntegrationMethod.euler, .rungeKutta4] {
            var engine = AttractorEngine(model: model, method: method, stepSize: 0.004, trailCapacity: 500)
            for _ in 0..<500 {
                try engine.advance()
            }
            #expect(engine.state.x.isFinite)
            #expect(engine.state.y.isFinite)
            #expect(engine.state.z.isFinite)
        }
    }

    @Test("Every bundled catalog attractor stays finite under its debug defaults")
    func allCatalogAttractorsFinite() throws {
        let catalog = try AttractorCatalog.bundled()
        for descriptor in catalog {
            let selection = try DebugConfig.selection(for: descriptor.id, in: catalog)
            let model = try selection.buildModel() // parses all formulas, incl. Aizawa's `^`
            var engine = AttractorEngine(
                model: model,
                initialState: selection.initialState,
                stepSize: selection.settings.stepSize,
                trailCapacity: 2000
            )
            for _ in 0..<2000 {
                try engine.advance()
            }
            #expect(engine.state.x.isFinite)
            #expect(engine.state.y.isFinite)
            #expect(engine.state.z.isFinite)
        }
    }
}
