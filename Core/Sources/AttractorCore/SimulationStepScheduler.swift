import Foundation

/// Turns wall-clock frame intervals into whole ODE integration steps.
///
/// The renderer runs on a timer, and `CACurrentMediaTime()` jumps by the whole
/// sleep/suspend duration after the machine wakes up. Feeding that interval
/// straight into `simulatedTime / stepSize` would request tens of thousands of
/// integration steps in one frame, and a zero step size would trap on
/// `Int(infinity)`. This scheduler bounds both the accepted frame interval and
/// the number of steps per frame, and drops the accumulated backlog once the
/// cap is hit, so a stall costs one slow frame instead of a freeze.
public struct SimulationStepScheduler: Sendable {
    /// Integration step size in simulated-time units. Non-positive values make
    /// the scheduler produce no steps.
    public var stepSize: Double
    /// Multiplier applied to wall-clock time. Non-positive values stop stepping.
    public var simulationSpeed: Double
    /// Longest frame interval the scheduler accounts for. Anything longer
    /// (sleep/wake, a debugger breakpoint) is clamped to this value.
    public var maximumFrameInterval: Double
    /// Hard cap on the number of integration steps performed in one call.
    public var maximumStepsPerFrame: Int
    /// Simulated time accumulated from previous frames but not consumed yet.
    public private(set) var accumulatedTime: Double

    /// Create a scheduler for one run.
    ///
    /// - Parameters:
    ///   - stepSize: Integration time step; must be positive to produce steps.
    ///   - simulationSpeed: Simulated-time multiplier per wall-clock second.
    ///   - maximumFrameInterval: Upper bound for one frame interval, in seconds.
    ///   - maximumStepsPerFrame: Upper bound for one frame's integration work.
    public init(
        stepSize: Double,
        simulationSpeed: Double,
        maximumFrameInterval: Double = 0.1,
        maximumStepsPerFrame: Int = 4_096
    ) {
        self.stepSize = stepSize
        self.simulationSpeed = simulationSpeed
        self.maximumFrameInterval =
            maximumFrameInterval.isFinite && maximumFrameInterval > 0
            ? maximumFrameInterval
            : 0.1
        self.maximumStepsPerFrame = max(1, min(maximumStepsPerFrame, 1_000_000))
        self.accumulatedTime = 0
    }

    /// Consume one rendered frame and return how many integration steps to run.
    ///
    /// Non-finite or non-positive inputs yield zero steps rather than a crash.
    /// If the requested work exceeds ``maximumStepsPerFrame``, the backlog is
    /// discarded: replaying minutes of missed simulation after a sleep/wake
    /// would keep the next frames slow for a long time.
    public mutating func steps(forFrameInterval frameInterval: Double) -> Int {
        guard frameInterval.isFinite, frameInterval > 0,
              stepSize.isFinite, stepSize > 0,
              simulationSpeed.isFinite, simulationSpeed > 0
        else {
            return 0
        }

        accumulatedTime += simulationSpeed * min(frameInterval, maximumFrameInterval)

        let requested = accumulatedTime / stepSize
        guard requested.isFinite else {
            accumulatedTime = 0
            return 0
        }

        if requested >= Double(maximumStepsPerFrame) {
            accumulatedTime = 0
            return maximumStepsPerFrame
        }

        let steps = Int(requested)
        accumulatedTime -= Double(steps) * stepSize
        return steps
    }
}
