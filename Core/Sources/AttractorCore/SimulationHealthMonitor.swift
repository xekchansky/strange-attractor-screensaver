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
/// divergence) still run on every state. The more expensive periodic check
/// runs every 512 states, and its verdict must persist for 4096 states before
/// the run is reported: chaotic flows can shadow an unstable orbit for a while.
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
    /// Minimum lag (in steps) between the current state and an earlier one
    /// for that pair to count as a possible period.
    public var minimumPeriodSteps: Int
    /// Number of recent states over which a candidate period is validated.
    public var periodicValidationWindow: Int
    /// Number of steps a periodic verdict must persist before it is reported.
    ///
    /// A chaotic flow can shadow an unstable periodic orbit for thousands of
    /// steps before wandering off. A genuine limit cycle keeps the verdict
    /// indefinitely, so requiring persistence separates the two.
    public var periodicConfirmationSteps: Int
    /// How often the periodic check runs, in observed steps.
    ///
    /// The periodic check is much more expensive than the collapse check, so
    /// it runs less often than the structural check schedule would imply.
    public var periodicCheckInterval: Int
    /// Maximum RMS distance for a validated period, as a fraction of the
    /// recent trajectory's spatial span. Above this the motion is not a
    /// repeatable closed orbit.
    public var periodicMeanDistanceFraction: Double
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
        periodicValidationWindow: Int = 2_048,
        periodicConfirmationSteps: Int = 4_096,
        periodicCheckInterval: Int = 512,
        periodicMeanDistanceFraction: Double = 0.1,
        hotPhaseSteps: Int = 10_000,
        warmPhaseSteps: Int = 100_000,
        hotCheckInterval: Int = 16,
        warmCheckInterval: Int = 128,
        coldCheckInterval: Int = 1_024
    ) {
        // Sanitize: a negative or non-finite threshold would otherwise flip the
        // meaning of every check (a negative divergence limit flags every
        // state, a zero window makes the periodic test meaningless).
        self.warmupSteps = max(0, warmupSteps)
        self.collapseWindowSteps = max(1, collapseWindowSteps)
        self.collapseSpan = collapseSpan.isFinite ? max(0, collapseSpan) : 0.01
        self.divergenceLimit = divergenceLimit.isFinite && divergenceLimit > 0
            ? divergenceLimit
            : 1e6
        let historyCapacity = max(2, historyCapacity)
        self.historyCapacity = historyCapacity
        self.minimumPeriodSteps = max(1, minimumPeriodSteps)
        self.periodicValidationWindow = max(1, min(periodicValidationWindow, historyCapacity - 1))
        self.periodicConfirmationSteps = max(1, periodicConfirmationSteps)
        self.periodicCheckInterval = max(1, periodicCheckInterval)
        self.periodicMeanDistanceFraction = periodicMeanDistanceFraction.isFinite
            ? min(max(periodicMeanDistanceFraction, 0), 1)
            : 0.1
        let hotPhaseSteps = max(0, hotPhaseSteps)
        self.hotPhaseSteps = hotPhaseSteps
        self.warmPhaseSteps = max(hotPhaseSteps, warmPhaseSteps)
        self.hotCheckInterval = max(1, hotCheckInterval)
        self.warmCheckInterval = max(1, warmCheckInterval)
        self.coldCheckInterval = max(1, coldCheckInterval)
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
    /// When the current run of periodic verdicts started, if one is active.
    private var pendingPeriodicSince: Int?
    private var nextPeriodicCheckStep: Int

    public init(thresholds: SimulationHealthThresholds = SimulationHealthThresholds()) {
        self.thresholds = thresholds
        self.capacity = max(1, thresholds.historyCapacity)
        self.nextCheckStep = thresholds.warmupSteps
        self.nextPeriodicCheckStep = thresholds.warmupSteps
        self.history.reserveCapacity(self.capacity)
    }

    /// Observe the next state and return an issue, if the run has gone wrong.
    ///
    /// Non-finite and divergent states are reported immediately, on any step.
    /// Structural checks (collapse and periodicity) start after
    /// ``SimulationHealthThresholds/warmupSteps`` and are then performed on a
    /// schedule that gets sparser as the simulation ages. Collapse is reported
    /// from the first matching check; periodicity only after the verdict has
    /// persisted for ``SimulationHealthThresholds/periodicConfirmationSteps``.
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
        if updatePeriodicConfirmation() { return .periodic }
        return nil
    }

    // MARK: - Structural checks

    /// Whether the recent trajectory is confined to a tiny region.
    private func isCollapsed() -> Bool {
        let window = min(max(1, thresholds.collapseWindowSteps), history.count)
        guard window > 1 else { return false }
        return recentSpan(window: window) < thresholds.collapseSpan
    }

    /// Track how long the trajectory has looked periodic and report it once
    /// the verdict has persisted long enough.
    ///
    /// One check is not conclusive: a chaotic flow can shadow an unstable
    /// periodic orbit for thousands of steps before wandering off. A genuine
    /// limit cycle reproduces the verdict on every check, so the pending
    /// timestamp survives until the run leaves the cycle.
    private mutating func updatePeriodicConfirmation() -> Bool {
        guard stepsObserved >= nextPeriodicCheckStep else { return false }
        nextPeriodicCheckStep = stepsObserved + max(1, thresholds.periodicCheckInterval)

        guard isPeriodic() else {
            pendingPeriodicSince = nil
            return false
        }

        let since = pendingPeriodicSince ?? stepsObserved
        pendingPeriodicSince = since
        return stepsObserved - since >= max(1, thresholds.periodicConfirmationSteps)
    }

    /// Whether the recent trajectory keeps retracing a closed orbit.
    ///
    /// A limit cycle rarely has a period that is an integer number of
    /// integration steps, so no sampled state ever repeats exactly: the
    /// closest return is off by up to half a step along the orbit. Instead of
    /// demanding an exact match, find the lag at which the newest state
    /// returns closest, then check that *every* state in a recent window has a
    /// similarly close partner one lag earlier. A chaotic flow has close
    /// encounters too, but they do not persist across a few hundred states.
    private func isPeriodic() -> Bool {
        let minimumPeriod = max(1, thresholds.minimumPeriodSteps)
        let validationWindow = min(
            max(16, thresholds.periodicValidationWindow),
            max(1, history.count / 2)
        )
        let maxGap = history.count - validationWindow
        guard maxGap >= minimumPeriod else { return false }

        let scaleWindow = min(
            max(thresholds.collapseWindowSteps, validationWindow),
            history.count
        )
        let scale = recentSpan(window: scaleWindow)
        guard scale > 0 else { return false }

        // Keep the few lags whose newest state is closest to the current one.
        let candidateLimit = 16
        var candidates: [(gap: Int, distance: Double)] = []
        candidates.reserveCapacity(candidateLimit)
        let newest = recentState(offsetFromNewest: 0)

        for gap in minimumPeriod...maxGap {
            let distance = squaredDistance(newest, recentState(offsetFromNewest: gap))
            if candidates.count < candidateLimit {
                candidates.append((gap, distance))
                candidates.sort { $0.distance < $1.distance }
            } else if distance < candidates[candidateLimit - 1].distance {
                candidates[candidateLimit - 1] = (gap, distance)
                candidates.sort { $0.distance < $1.distance }
            }
        }

        let threshold = thresholds.periodicMeanDistanceFraction * scale
        let thresholdSquared = threshold * threshold
        for candidate in candidates {
            let meanSquared = meanSquaredDistance(atGap: candidate.gap, window: validationWindow)
            guard meanSquared <= thresholdSquared else { continue }

            // Ordinary smooth motion also keeps nearby states close for short
            // lags, and its chord distance grows sub-linearly, which can look
            // like a weak dip. A real period is a sharp dip: half a period
            // away the trajectory is on the opposite side of the loop, an
            // order of magnitude farther. For chaos no such lag exists.
            let halfGap = max(1, candidate.gap / 2)
            let halfMeanSquared = meanSquaredDistance(atGap: halfGap, window: validationWindow)
            if meanSquared * 100 <= halfMeanSquared {
                return true
            }
        }
        return false
    }

    // MARK: - Helpers

    /// Mean squared distance between states one `gap` apart, over the last
    /// `window` states.
    private func meanSquaredDistance(atGap gap: Int, window: Int) -> Double {
        var sum = 0.0
        for offset in 0..<window {
            sum += squaredDistance(
                recentState(offsetFromNewest: offset),
                recentState(offsetFromNewest: offset + gap)
            )
        }
        return sum / Double(window)
    }

    /// The largest axis-aligned extent of the last `window` states.
    private func recentSpan(window: Int) -> Double {
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

        return max(maxX - minX, max(maxY - minY, maxZ - minZ))
    }

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
