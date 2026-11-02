import Foundation

/// Concrete, sampled values of the simulation/visualization hyperparameters.
///
/// This is the "frozen" counterpart of ``SimulationHyperparameters`` (which
/// holds `[min, max]` ranges): each field is a single value drawn from its
/// range by ``Randomizer`` or pinned explicitly by ``DebugConfig``.
public struct SimulationSettings: Equatable, Sendable {
    /// Integration time step (accuracy) of the ODE solver, per simulation tick.
    public var stepSize: Double
    /// Multiplier of simulated time per rendered frame.
    public var simulationSpeed: Double
    /// Angular velocity of the orbiting camera, radians per second.
    public var cameraRotationSpeed: Double
    /// How fast the trail's color gradient advances.
    public var trailColorSpeed: Double
    /// Number of points kept in the rendered trail buffer.
    public var trailLength: Int
    /// How the trail is colored (rainbow, age fade, depth, spatial).
    public var colorMode: TraceColorMode

    public init(
        stepSize: Double,
        simulationSpeed: Double,
        cameraRotationSpeed: Double,
        trailColorSpeed: Double,
        trailLength: Int,
        colorMode: TraceColorMode
    ) {
        self.stepSize = stepSize
        self.simulationSpeed = simulationSpeed
        self.cameraRotationSpeed = cameraRotationSpeed
        self.trailColorSpeed = trailColorSpeed
        self.trailLength = trailLength
        self.colorMode = colorMode
    }

    /// Sample the midpoint of every range in `hyperparameters`.
    ///
    /// Used by ``DebugConfig`` as the default when no explicit settings are
    /// supplied.
    public init(midpointsOf hyperparameters: SimulationHyperparameters) {
        self.init(
            stepSize: Self.midpoint(of: hyperparameters.stepSize),
            simulationSpeed: Self.midpoint(of: hyperparameters.simulationSpeed),
            cameraRotationSpeed: Self.midpoint(of: hyperparameters.cameraRotationSpeed),
            trailColorSpeed: Self.midpoint(of: hyperparameters.trailColorSpeed),
            trailLength: Self.midpoint(of: hyperparameters.trailLength),
            colorMode: .trail
        )
    }

    /// Problems that would make the renderer misbehave.
    ///
    /// Used by the demo to reject nonsensical command-line overrides before
    /// they produce a frozen trace (non-positive step), an empty polyline
    /// (trail shorter than two points) or non-finite camera/color motion.
    /// A zero speed is allowed: it intentionally freezes one aspect for
    /// debugging.
    public var validationIssues: [String] {
        var issues: [String] = []

        if !stepSize.isFinite || stepSize <= 0 {
            issues.append("stepSize must be positive and finite (got \(stepSize))")
        }
        if !simulationSpeed.isFinite || simulationSpeed < 0 {
            issues.append("simulationSpeed must be non-negative and finite (got \(simulationSpeed))")
        }
        if !cameraRotationSpeed.isFinite || cameraRotationSpeed < 0 {
            issues.append("cameraRotationSpeed must be non-negative and finite (got \(cameraRotationSpeed))")
        }
        if !trailColorSpeed.isFinite || trailColorSpeed < 0 {
            issues.append("trailColorSpeed must be non-negative and finite (got \(trailColorSpeed))")
        }
        if trailLength < 2 {
            issues.append("trailLength must keep at least 2 points (got \(trailLength))")
        }

        return issues
    }

    private static func midpoint(of range: MinMax<Double>) -> Double {
        (range.min + range.max) / 2
    }

    private static func midpoint(of range: MinMax<Int>) -> Int {
        (range.min + range.max) / 2
    }
}
