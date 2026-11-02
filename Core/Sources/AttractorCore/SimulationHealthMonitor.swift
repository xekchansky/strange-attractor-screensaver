import Foundation

/// A detected problem with a running simulation.
///
/// The screensaver treats every issue the same way: it discards the current
/// selection and starts a completely new random simulation.
public enum SimulationHealthIssue: Equatable, Sendable {
    /// The trajectory produced a NaN or an infinite coordinate.
    case nonFinite
    /// A coordinate grew past ``SimulationHealthThresholds/divergenceLimit``.
    case diverged
    /// The trajectory is confined to a tiny region: an equilibrium point or a
    /// very small limit cycle (Chua's near-zero ring).
    case collapsed
    /// The trajectory returns to an earlier state exactly: a stable limit
    /// cycle, large or small.
    case periodic
}

/// Tuning knobs for ``SimulationHealthMonitor``.
///
/// Defaults are chosen for the bundled catalog: warm up for 1000 steps, then
/// check often for the first 10k steps (where bad launches show up), less
/// often up to 100k, and rarely afterwards. Cheap checks (non-finite and
/// divergence) still run on every state.
public struct SimulationHealthThresholds: Equatable, Sendable {
    /// Number of observed states before structural checks (collapse/periodic)
    /// start. Prevents flagging a normal transient.
    public var warmupSteps: Int
    /// Number of recent states used to measure the trajectory's spatial span.
    public var collapseWindowSteps: Int
    /// A span below this value means the trajectory is visually a point.
    public var collapseSpan: Double
    /// A coordinate magnitude above this value means the run has diverged.
    public var divergenceLimit: Double
    /// Maximum number of states remembered for recurrence checks.
    public var historyCapacity: Int
    /// Minimum distance (in steps) between two matching trajectory segments.
    public var minimumPeriodSteps: Int
    /// Number of consecutive states compared when looking for a repeat.
    public var segmentLength: Int
    /// Per-coordinate tolerance under which two states count as identical.
    public var recurrenceTolerance: Double
    /// Length of the hot phase, counted in observed states.
    public var hotPhaseSteps: Int
    /// Length of the warm phase, counted in observed states.
    public var warmPhaseSteps: Int
    /// Structural check interval during the hot phase.
    public var hotCheckInterval: Int
    /// Structural check interval during the warm phase.
    public var warmCheckInterval: Int
    /// Structural check interval after the warm phase.
    public var coldCheckInterval: Int

    public init(
        warmupSteps: Int = 1_000,
        collapseWindowSteps: Int = 1_000,
        collapseSpan: Double = 0.01,
        divergenceLimit: Double = 1e6,
        historyCapacity: Int = 4_096,
        minimumPeriodSteps: Int = 64,
        segmentLength: Int = 8,
        recurrenceTolerance: Double = 1e-6,
        hotPhaseSteps: Int = 10_000,
        warmPhaseSteps: Int = 100_000,
        hotCheckInterval: Int = 16,
        warmCheckInterval: Int = 128,
        coldCheckInterval: Int = 1_024
    ) {
        self.warmupSteps = warmupSteps
        self.collapseWindowSteps = collapseWindowSteps
        self.collapseSpan = collapseSpan
        self.divergenceLimit = divergenceLimit
        self.historyCapacity = historyCapacity
        self.minimumPeriodSteps = minimumPeriodSteps
        self.segmentLength = segmentLength
        self.recurrenceTolerance = recurrenceTolerance
        self.hotPhaseSteps = hotPhaseSteps
        self.warmPhaseSteps = warmPhaseSteps
        self.hotCheckInterval = hotCheckInterval
        self.warmCheckInterval = warmCheckInterval
        self.coldCheckInterval = coldCheckInterval
    }
}

/// Watches a stream of simulation states and reports when the run has gone
/// wrong: it froze at an equilibrium, settled into a limit cycle, or blew up.
///
/// The monitor is pure and deterministic: it depends only on the observed
/// states and its configuration, so it is unit-testable without a renderer.
/// Feed it one state per integration step.
public struct SimulationHealthMonitor: Sendable {
    /// The configuration in effect.
    public let thresholds: SimulationHealthThresholds
    /// How many states have been observed so far.
    public private(set) var stepsObserved = 0
    /// How many full structural checks have run. Exposed for tests so the
    /// back-off schedule can be verified.
    private(set) var checkCount = 0

    private let capacity: Int
    private var history: [State] = []
    /// Next slot to overwrite once ``history`` is full; also the oldest entry.
    private var writeIndex = 0
    private var saturated = false
    private var nextCheckStep: Int

    public init(thresholds: SimulationHealthThresholds = SimulationHealthThresholds()) {
        self.thresholds = thresholds
        self.capacity = max(1, thresholds.historyCapacity)
        self.nextCheckStep = thresholds.warmupSteps
        self.history.reserveCapacity(self.capacity)
    }

    /// Observe the next state and return an issue, if the run has gone wrong.
    ///
    /// Non-finite and divergent states are reported immediately, on any step.
    /// Structural checks (collapse and periodicity) start after
    /// ``SimulationHealthThresholds/warmupSteps`` and are then performed on a
    /// schedule that gets sparser as the simulation ages.
    public mutating func observe(_ state: State) -> SimulationHealthIssue? {
        guard state.isFinite else { return .nonFinite }
        guard !isDiverged(state) else { return .diverged }

        append(state)
        stepsObserved += 1

        guard stepsObserved >= thresholds.warmupSteps,
              stepsObserved >= nextCheckStep else {
            return nil
        }

        checkCount += 1
        nextCheckStep = stepsObserved + checkInterval(after: stepsObserved)

        if isCollapsed() { return .collapsed }
        if isPeriodic() { return .periodic }
        return nil
    }

    // MARK: - Structural checks

    /// Whether the recent trajectory is confined to a tiny region.
    private func isCollapsed() -> Bool {
        let window = min(max(1, thresholds.collapseWindowSteps), history.count)
        guard window > 1 else { return false }

        var minX = Double.infinity
        var maxX = -Double.infinity
        var minY = Double.infinity
        var maxY = -Double.infinity
        var minZ = Double.infinity
        var maxZ = -Double.infinity

        for offset in 0..<window {
            let state = recentState(offsetFromNewest: offset)
            minX = min(minX, state.x)
            maxX = max(maxX, state.x)
            minY = min(minY, state.y)
            maxY = max(maxY, state.y)
            minZ = min(minZ, state.z)
            maxZ = max(maxZ, state.z)
        }

        let span = max(maxX - minX, max(maxY - minY, maxZ - minZ))
        return span < thresholds.collapseSpan
    }

    /// Whether a recent segment of the trajectory repeats an earlier segment.
    ///
    /// A chaotic flow practically never revisits the same state to within
    /// ``SimulationHealthThresholds/recurrenceTolerance``; a periodic orbit
    /// repeats it every period (the integration is deterministic, so the
    /// coordinates agree up to floating-point round-off).
    private func isPeriodic() -> Bool {
        let segmentLength = max(1, thresholds.segmentLength)
        let minimumPeriod = max(1, thresholds.minimumPeriodSteps)
        guard history.count >= segmentLength + minimumPeriod else { return false }

        let toleranceSquared = thresholds.recurrenceTolerance * thresholds.recurrenceTolerance

        var newestSegment: [State] = []
        newestSegment.reserveCapacity(segmentLength)
        for offset in stride(from: segmentLength - 1, through: 0, by: -1) {
            newestSegment.append(recentState(offsetFromNewest: offset))
        }

        let firstCandidateEnd = minimumPeriod
        let lastCandidateEnd = history.count - segmentLength
        guard lastCandidateEnd >= firstCandidateEnd else { return false }

        for candidateEnd in firstCandidateEnd...lastCandidateEnd {
            var matches = true
            for index in 0..<segmentLength {
                let candidateOffset = candidateEnd + (segmentLength - 1 - index)
                let candidate = recentState(offsetFromNewest: candidateOffset)
                if squaredDistance(candidate, newestSegment[index]) > toleranceSquared {
                    matches = false
                    break
                }
            }
            if matches { return true }
        }
        return false
    }

    // MARK: - Helpers

    private func checkInterval(after steps: Int) -> Int {
        if steps < thresholds.hotPhaseSteps {
            return max(1, thresholds.hotCheckInterval)
        }
        if steps < thresholds.warmPhaseSteps {
            return max(1, thresholds.warmCheckInterval)
        }
        return max(1, thresholds.coldCheckInterval)
    }

    private func isDiverged(_ state: State) -> Bool {
        let limit = thresholds.divergenceLimit
        return abs(state.x) > limit || abs(state.y) > limit || abs(state.z) > limit
    }

    private mutating func append(_ state: State) {
        if history.count < capacity {
            history.append(state)
            if history.count == capacity {
                saturated = true
                writeIndex = 0
            }
        } else {
            history[writeIndex] = state
            writeIndex = (writeIndex + 1) % capacity
        }
    }

    /// The state `offset` steps back from the newest one (0 = newest).
    private func recentState(offsetFromNewest offset: Int) -> State {
        if saturated {
            let index = (writeIndex - 1 - offset + capacity * 2) % capacity
            return history[index]
        }
        return history[history.count - 1 - offset]
    }

    private func squaredDistance(_ lhs: State, _ rhs: State) -> Double {
        let dx = lhs.x - rhs.x
        let dy = lhs.y - rhs.y
        let dz = lhs.z - rhs.z
        return dx * dx + dy * dy + dz * dz
    }
}

private extension State {
    var isFinite: Bool {
        x.isFinite && y.isFinite && z.isFinite
    }
}
