import Foundation
import Testing
@testable import AttractorCore

@Suite("TraceColorMode")
struct TraceColorModeTests {
    @Test("Declares four distinct modes")
    func declaresFourDistinctModes() {
        #expect(TraceColorMode.allCases.count == 4)
        #expect(Set(TraceColorMode.allCases.map(\.rawValue)).count == 4)
        #expect(Set(TraceColorMode.allCases).count == 4)
    }

    @Test("Raw values round-trip through the initializer")
    func rawValuesRoundTrip() {
        for mode in TraceColorMode.allCases {
            #expect(TraceColorMode(rawValue: mode.rawValue) == mode)
        }
    }

    @Test("Midpoint settings default to the trail mode")
    func midpointSettingsDefaultToTrail() {
        let settings = SimulationSettings(midpointsOf: SimulationHyperparameters())
        #expect(settings.colorMode == .trail)
    }
}
