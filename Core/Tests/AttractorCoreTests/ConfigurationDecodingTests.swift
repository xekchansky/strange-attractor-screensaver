import Foundation
import Testing
@testable import AttractorCore

/// A complete, valid JSON descriptor for a Lorenz-like attractor used as the
/// shared fixture across the decoding tests below.
private func lorenzJSON() -> String {
    """
    [
      {
        "id": "lorenz",
        "name": "Lorenz",
        "dx": "a * (y - x)",
        "dy": "x * (r - z) - y",
        "dz": "x * y - b * z",
        "parameters": [
          { "name": "a", "min": 9.0, "max": 10.0 },
          { "name": "b", "min": 2.5, "max": 3.0 },
          { "name": "r", "min": 24.0, "max": 30.0 }
        ],
        "initialState": {
          "x": { "min": -1.0, "max": 1.0 },
          "y": { "min": -1.0, "max": 1.0 },
          "z": { "min": -1.0, "max": 1.0 }
        },
        "simulation": {
          "stepSize": { "min": 0.004, "max": 0.01 },
          "simulationSpeed": { "min": 1.0, "max": 3.0 },
          "cameraRotationSpeed": { "min": 0.1, "max": 0.4 },
          "trailColorSpeed": { "min": 0.05, "max": 0.3 },
          "trailLength": { "min": 2000, "max": 8000 }
        }
      }
    ]
    """
}

/// Same fixture but without the `simulation` block, to exercise missing-section
/// decoding failure.
private func lorenzJSONWithoutSimulation() -> String {
    """
    [
      {
        "id": "lorenz",
        "name": "Lorenz",
        "dx": "a * (y - x)",
        "dy": "x * (r - z) - y",
        "dz": "x * y - b * z",
        "parameters": [
          { "name": "a", "min": 9.0, "max": 10.0 }
        ],
        "initialState": {
          "x": { "min": -1.0, "max": 1.0 },
          "y": { "min": -1.0, "max": 1.0 },
          "z": { "min": -1.0, "max": 1.0 }
        }
      }
    ]
    """
}

private func decodeFirst(_ json: String) throws -> AttractorDescriptor {
    let data = Data(json.utf8)
    let list = try JSONDecoder().decode([AttractorDescriptor].self, from: data)
    return try #require(list.first)
}

@Suite("Configuration decoding")
struct ConfigurationDecodingTests {
    @Test("Valid fixture decodes all fields")
    func decodesAllFields() throws {
        let descriptor = try decodeFirst(lorenzJSON())

        #expect(descriptor.id == "lorenz")
        #expect(descriptor.name == "Lorenz")
        #expect(descriptor.dx == "a * (y - x)")
        #expect(descriptor.dy == "x * (r - z) - y")
        #expect(descriptor.dz == "x * y - b * z")

        #expect(descriptor.parameters.count == 3)
        let a = try #require(descriptor.parameters.first { $0.name == "a" })
        #expect(a.min == 9.0)
        #expect(a.max == 10.0)

        let sim = descriptor.simulation
        #expect(sim.stepSize.min == 0.004)
        #expect(sim.stepSize.max == 0.01)
        #expect(sim.simulationSpeed.closedRange == 1.0...3.0)
        #expect(sim.cameraRotationSpeed.min == 0.1)
        #expect(sim.trailColorSpeed.max == 0.3)
        #expect(sim.trailLength.min == 2000)
        #expect(sim.trailLength.max == 8000)
        #expect(sim.trailLength.closedRange == 2000...8000)

        #expect(descriptor.initialState.x.closedRange == -1.0...1.0)
        #expect(descriptor.initialState.y.closedRange == -1.0...1.0)
        #expect(descriptor.initialState.z.closedRange == -1.0...1.0)
    }

    @Test("Missing required field throws a decoding error")
    func missingFieldThrows() {
        // Drop the "dz" formula key from the fixture.
        let malformed = lorenzJSON()
            .replacingOccurrences(of: #""dz": "x * y - b * z","#, with: "")

        #expect(throws: DecodingError.self) {
            _ = try JSONDecoder().decode([AttractorDescriptor].self, from: Data(malformed.utf8))
        }
    }

    @Test("Missing simulation section throws a decoding error")
    func missingSimulationThrows() {
        #expect(throws: DecodingError.self) {
            _ = try JSONDecoder().decode(
                [AttractorDescriptor].self,
                from: Data(lorenzJSONWithoutSimulation().utf8)
            )
        }
    }

    @Test("Malformed numeric value throws a decoding error")
    func malformedNumberThrows() {
        let malformed = lorenzJSON().replacingOccurrences(
            of: #""min": 9.0"#,
            with: #""min": "not-a-number""#
        )

        #expect(throws: DecodingError.self) {
            _ = try JSONDecoder().decode([AttractorDescriptor].self, from: Data(malformed.utf8))
        }
    }

    @Test("Unknown keys are ignored without failing")
    func unknownKeysIgnored() throws {
        let extended = lorenzJSON().replacingOccurrences(
            of: #""id": "lorenz","#,
            with: ##""id": "lorenz", "color": "#fff", "extra": 42,"##
        )

        let descriptor = try decodeFirst(extended)
        #expect(descriptor.id == "lorenz")
    }

    @Test("Bundled catalog contains the reference attractors")
    func bundledCatalog() throws {
        let catalog = try AttractorCatalog.bundled()

        #expect(catalog.count == 5)
        #expect(catalog.map(\.id) == ["lorenz", "rossler", "thomas", "chen", "aizawa"])
        #expect(catalog.map(\.name) == ["Lorenz", "Rössler", "Thomas", "Chen", "Aizawa"])

        let lorenz = try #require(catalog.first { $0.id == "lorenz" })
        #expect(lorenz.parameters.map(\.name) == ["a", "b", "r"])
        let thomas = try #require(catalog.first { $0.id == "thomas" })
        #expect(thomas.parameters.map(\.name) == ["b"])
        let chen = try #require(catalog.first { $0.id == "chen" })
        #expect(chen.parameters.map(\.name) == ["a", "b", "c"])
        let aizawa = try #require(catalog.first { $0.id == "aizawa" })
        #expect(aizawa.parameters.map(\.name) == ["a", "b", "c", "d", "e", "f"])
    }
}
