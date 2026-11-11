import Foundation

/// Metadata and formula definitions for a single attractor, loaded from
/// `attractors.json`.
///
/// Formulas are plain strings parsed at runtime by ``FormulaParser``; their
/// variables are the state coordinates (`x`, `y`, `z`) and the declared
/// ``Parameter`` names.
public struct AttractorDescriptor: Codable, Equatable, Sendable {
    /// Stable unique identifier (e.g. "lorenz").
    public let id: String
    /// Human-readable display name.
    public let name: String
    /// Formula for dX/dt as a string.
    public let dx: String
    /// Formula for dY/dt as a string.
    public let dy: String
    /// Formula for dZ/dt as a string.
    public let dz: String
    /// Tunable ODE coefficients together with their sampled ranges.
    public let parameters: [Parameter]
    /// Ranges governing the simulation/visualization.
    public let simulation: SimulationHyperparameters
    /// Ranges for the initial state coordinates.
    public let initialState: StateRange

    public init(
        id: String,
        name: String,
        dx: String,
        dy: String,
        dz: String,
        parameters: [Parameter],
        simulation: SimulationHyperparameters,
        initialState: StateRange
    ) {
        self.id = id
        self.name = name
        self.dx = dx
        self.dy = dy
        self.dz = dz
        self.parameters = parameters
        self.simulation = simulation
        self.initialState = initialState
    }
}
