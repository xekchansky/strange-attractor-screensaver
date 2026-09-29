import Foundation
@testable import AttractorCore

/// Shared test fixtures for building descriptors without repeating JSON.
enum TestSupport {
    /// The reference Lorenz descriptor with classic numeric constants.
    static func lorenzDescriptor() -> AttractorDescriptor {
        AttractorDescriptor(
            id: "lorenz",
            name: "Lorenz",
            dx: "a * (y - x)",
            dy: "x * (r - z) - y",
            dz: "x * y - b * z",
            parameters: [
                Parameter(name: "a", min: 9, max: 10),
                Parameter(name: "b", min: 2.5, max: 3),
                Parameter(name: "r", min: 24, max: 30),
            ],
            simulation: SimulationHyperparameters()
        )
    }

    /// The Thomas attractor, which exercises the `sin` function.
    static func thomasDescriptor() -> AttractorDescriptor {
        AttractorDescriptor(
            id: "thomas",
            name: "Thomas",
            dx: "sin(y) - b * x",
            dy: "sin(z) - b * y",
            dz: "sin(x) - b * z",
            parameters: [
                Parameter(name: "b", min: 0.18, max: 0.24),
            ],
            simulation: SimulationHyperparameters()
        )
    }
}
