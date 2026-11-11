import Foundation

/// Owns the state of a simulation, steps the system through numerical
/// integration, and keeps a bounded buffer of the most recent trace points.
public struct AttractorEngine: Sendable {
    /// The integrable attractor model being simulated.
    public let model: AttractorModel
    /// The integration scheme.
    public let method: IntegrationMethod
    /// Integration time step (simulation accuracy).
    public var stepSize: Double
    /// Maximum number of points kept in `trace`.
    public let trailCapacity: Int

    /// Current simulation state.
    public private(set) var state: State

    /// The bounded ring buffer of the most recent states, oldest first.
    public private(set) var trace: TraceBuffer

    /// Create an engine with an initial state and an empty trace containing
    /// just that initial state.
    public init(
        model: AttractorModel,
        initialState: State = State(x: 0.1, y: 0.1, z: 0.1),
        method: IntegrationMethod = .rungeKutta4,
        stepSize: Double,
        trailCapacity: Int = 2048
    ) {
        self.model = model
        self.state = initialState
        self.method = method
        self.stepSize = stepSize
        self.trailCapacity = max(1, trailCapacity)
        self.trace = TraceBuffer(capacity: max(1, trailCapacity), initial: initialState)
    }

    /// Advance the simulation by one integration step and record the new point.
    ///
    /// - Returns: The resulting state.
    /// - Throws: ``FormulaError`` if the vector field cannot be evaluated.
    @discardableResult
    public mutating func advance() throws -> State {
        state = try ODESolver.step(
            from: state,
            dt: stepSize,
            method: method,
            rate: { try self.model.rate(of: $0) }
        )
        trace.append(state)
        return state
    }
}
