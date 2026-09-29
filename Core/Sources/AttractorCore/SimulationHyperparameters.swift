import Foundation

/// Ranges that govern the *simulation/visualization* rather than the physical
/// constants of the ODE (which are modeled by ``Parameter``).
///
/// Every field is a `[min, max]` sampling range expressed as a ``MinMax``. At
/// startup the randomizer draws a concrete value from each range;
/// ``DebugConfig`` can pin them explicitly.
public struct SimulationHyperparameters: Codable, Equatable, Sendable {
    /// Integration time step (accuracy) of the ODE solver, per simulation tick.
    public var stepSize: MinMax<Double>
    /// Multiplier of simulated time per rendered frame (overall motion speed).
    public var simulationSpeed: MinMax<Double>
    /// Angular velocity of the orbiting camera, in radians per second.
    public var cameraRotationSpeed: MinMax<Double>
    /// How fast the trail's color gradient advances along the trace.
    public var trailColorSpeed: MinMax<Double>
    /// Number of points kept in the rendered trail buffer.
    public var trailLength: MinMax<Int>

    public init(
        stepSize: MinMax<Double> = MinMax(min: 0.004, max: 0.01),
        simulationSpeed: MinMax<Double> = MinMax(min: 1, max: 3),
        cameraRotationSpeed: MinMax<Double> = MinMax(min: 0.1, max: 0.4),
        trailColorSpeed: MinMax<Double> = MinMax(min: 0.05, max: 0.3),
        trailLength: MinMax<Int> = MinMax(min: 2000, max: 8000)
    ) {
        self.stepSize = stepSize
        self.simulationSpeed = simulationSpeed
        self.cameraRotationSpeed = cameraRotationSpeed
        self.trailColorSpeed = trailColorSpeed
        self.trailLength = trailLength
    }
}
