import Testing
import Foundation
@testable import AttractorCore

@Suite("Simulation health monitor")
struct SimulationHealthMonitorTests {
    /// A healthy chaotic trajectory must not be mistaken for a failure.
    @Test("A chaotic Lorenz trajectory stays healthy")
    func chaoticTrajectoryIsHealthy() throws {
        let model = try AttractorModel(
            descriptor: TestSupport.lorenzDescriptor(),
            parameters: ["a": 10, "b": 8.0 / 3.0, "r": 28]
        )
        var engine = AttractorEngine(
            model: model,
            initialState: State(x: 0.5, y: -0.3, z: 0.2),
            stepSize: 0.004,
            trailCapacity: 100
        )
        var monitor = SimulationHealthMonitor()
        #expect(monitor.observe(engine.state) == nil)

        for _ in 0..<20_000 {
            try engine.advance()
            #expect(monitor.observe(engine.state) == nil)
        }
    }

    /// Aizawa has long laminar phases that shadow an unstable periodic orbit
    /// for about two thousand steps before the trajectory wanders off. The
    /// confirmation interval must reject them; treating one matching window
    /// as proof would heal a healthy run.
    @Test("Aizawa's laminar phases are not mistaken for a limit cycle")
    func aizawaLaminarPhasesAreHealthy() throws {
        let catalog = try AttractorCatalog.bundled()
        let selection = try DebugConfig.selection(for: "aizawa", in: catalog)
        var engine = AttractorEngine(
            model: try selection.buildModel(),
            initialState: selection.initialState,
            stepSize: selection.settings.stepSize,
            trailCapacity: 100
        )
        var monitor = SimulationHealthMonitor()

        for _ in 0..<16_384 {
            try engine.advance()
            let issue = monitor.observe(engine.state)
            #expect(issue == nil, "aizawa: \(String(describing: issue))")
            if issue != nil { break }
        }
    }

    /// A resting point is the simplest degenerate launch.
    @Test("A fixed point is reported as collapsed")
    func fixedPointIsCollapsed() {
        var monitor = SimulationHealthMonitor()
        let point = State(x: 0.1, y: 0.2, z: 0.3)

        var issue: SimulationHealthIssue?
        for _ in 0..<10_000 where issue == nil {
            issue = monitor.observe(point)
        }
        #expect(issue == .collapsed)
    }

    /// Chua's near-zero ring is a tiny periodic orbit: the span criterion
    /// catches it before the recurrence criterion would.
    @Test("A tiny limit cycle is reported as collapsed")
    func tinyLimitCycleIsCollapsed() {
        var monitor = SimulationHealthMonitor(thresholds: .init(warmupSteps: 100))

        var issue: SimulationHealthIssue?
        for step in 0..<5_000 where issue == nil {
            let angle = 2.0 * Double.pi * Double(step % 97) / 97.0
            issue = monitor.observe(State(
                x: 0.001 * cos(angle),
                y: 0.001 * sin(angle),
                z: 0.0
            ))
        }
        #expect(issue == .collapsed)
    }

    /// A large stable limit cycle moves visibly but is not chaotic.
    @Test("A large limit cycle is reported as periodic")
    func largeLimitCycleIsPeriodic() {
        var monitor = SimulationHealthMonitor(thresholds: .init(
            warmupSteps: 100,
            minimumPeriodSteps: 64
        ))

        var issue: SimulationHealthIssue?
        for step in 0..<5_000 where issue == nil {
            let angle = 2.0 * Double.pi * Double(step % 200) / 200.0
            issue = monitor.observe(State(
                x: 5.0 * cos(angle),
                y: 5.0 * sin(angle),
                z: 0.0
            ))
        }
        #expect(issue == .periodic)
    }

    /// A chaotic flow can look periodic for a while, and a single matching
    /// window is not enough to declare a limit cycle. The verdict must survive
    /// the whole confirmation interval; an interruption restarts the count.
    @Test("A periodic verdict must persist before it is reported")
    func periodicVerdictMustPersist() {
        func verdict(breakingAt breakStart: Int?) -> SimulationHealthIssue? {
            let thresholds = SimulationHealthThresholds(
                warmupSteps: 100,
                minimumPeriodSteps: 64,
                periodicValidationWindow: 128,
                periodicConfirmationSteps: 2_000,
                periodicCheckInterval: 50,
                hotCheckInterval: 1
            )
            var monitor = SimulationHealthMonitor(thresholds: thresholds)

            for step in 0..<4_000 {
                if let start = breakStart, step >= start, step < start + 500 {
                    let wandering = State(
                        x: 0.01 * Double(step),
                        y: sin(0.3 * Double(step)),
                        z: cos(0.17 * Double(step))
                    )
                    if let issue = monitor.observe(wandering) { return issue }
                    continue
                }
                let angle = 2.0 * Double.pi * Double(step % 200) / 200.0
                if let issue = monitor.observe(State(
                    x: 5.0 * cos(angle),
                    y: 5.0 * sin(angle),
                    z: 0
                )) {
                    return issue
                }
            }
            return nil
        }

        #expect(verdict(breakingAt: nil) == .periodic)
        #expect(verdict(breakingAt: 2_000) == nil)
    }

    @Test("A non-finite state is reported immediately")
    func nonFiniteIsReported() {
        var monitor = SimulationHealthMonitor()
        #expect(monitor.observe(State(x: .nan, y: 0, z: 0)) == .nonFinite)
        #expect(monitor.observe(State(x: 0, y: .infinity, z: 0)) == .nonFinite)
    }

    @Test("A divergent state is reported immediately")
    func divergenceIsReported() {
        var monitor = SimulationHealthMonitor()
        #expect(monitor.observe(State(x: 1e7, y: 0, z: 0)) == .diverged)
    }

    @Test("Structural checks do not run before the warm-up ends")
    func warmupSuppressesEarlyVerdict() {
        var monitor = SimulationHealthMonitor(thresholds: .init(warmupSteps: 500))
        let point = State(x: 0.1, y: 0.2, z: 0.3)

        for _ in 0..<499 {
            #expect(monitor.observe(point) == nil)
        }
        #expect(monitor.observe(point) == .collapsed)
    }

    /// Checks must get rarer as the run ages: a healthy 10k-step run under the
    /// three-phase schedule performs far fewer full checks than the hot
    /// interval alone would imply.
    @Test("Structural checks back off as the run ages")
    func checksBackOff() {
        let thresholds = SimulationHealthThresholds(
            warmupSteps: 50,
            hotPhaseSteps: 200,
            warmPhaseSteps: 1_000,
            hotCheckInterval: 5,
            warmCheckInterval: 20,
            coldCheckInterval: 100
        )
        var monitor = SimulationHealthMonitor(thresholds: thresholds)

        for step in 0..<10_000 {
            let state = State(
                x: 0.001 * Double(step),
                y: sin(Double(step) * 0.1),
                z: cos(Double(step) * 0.07)
            )
            #expect(monitor.observe(state) == nil)
        }
        // Every-5-steps throughout would be ~2000 checks; the schedule must
        // stay well below that.
        #expect(monitor.checkCount < 500)
        #expect(monitor.checkCount > 100)
    }

    /// A cycle that appears only after a long healthy run is still detected,
    /// within one cold-phase interval of the window refilling.
    @Test("A late collapse is still detected after checks back off")
    func lateCollapseIsDetected() {
        let thresholds = SimulationHealthThresholds(
            warmupSteps: 50,
            collapseWindowSteps: 100,
            hotPhaseSteps: 500,
            warmPhaseSteps: 1_000,
            hotCheckInterval: 5,
            warmCheckInterval: 20,
            coldCheckInterval: 50
        )
        var monitor = SimulationHealthMonitor(thresholds: thresholds)

        for step in 0..<5_000 {
            let state = State(
                x: 0.001 * Double(step),
                y: sin(Double(step) * 0.1),
                z: cos(Double(step) * 0.07)
            )
            #expect(monitor.observe(state) == nil)
        }

        let fixed = State(x: 0.1, y: 0.2, z: 0.3)
        var issue: SimulationHealthIssue?
        var detectedAfter = 0
        for offset in 1...500 where issue == nil {
            issue = monitor.observe(fixed)
            detectedAfter = offset
        }
        #expect(issue == .collapsed)
        #expect(detectedAfter <= thresholds.collapseWindowSteps + thresholds.coldCheckInterval)
    }

    /// Calibration guard for the motivating case: Chua develops its double
    /// scroll spontaneously from a near-zero start, and that slow start must
    /// not be mistaken for the near-zero ring the monitor is meant to heal.
    /// The debug midpoint parameters settle into a large cycle much later,
    /// which is a genuine failure; only the slow start is checked here.
    @Test("Chua's slow start is not flagged")
    func chuaSlowStartIsHealthy() throws {
        let catalog = try AttractorCatalog.bundled()
        let selection = try DebugConfig.selection(for: "chua", in: catalog)
        var engine = AttractorEngine(
            model: try selection.buildModel(),
            initialState: selection.initialState,
            stepSize: selection.settings.stepSize,
            trailCapacity: 100
        )
        var monitor = SimulationHealthMonitor()
        #expect(monitor.observe(engine.state) == nil)

        for _ in 0..<5_000 {
            try engine.advance()
            let issue = monitor.observe(engine.state)
            #expect(issue == nil, "chua: \(String(describing: issue))")
            if issue != nil { break }
        }
    }

    /// A real sampled limit cycle usually has a period that is not an integer
    /// number of integration steps, so no pair of samples ever repeats
    /// exactly. The monitor must still recognise the closed orbit.
    @Test("A sampled limit cycle with a non-integer period is reported as periodic")
    func nonIntegerPeriodLimitCycleIsPeriodic() {
        var monitor = SimulationHealthMonitor(thresholds: .init(warmupSteps: 100))
        let period = 200.5

        var issue: SimulationHealthIssue?
        for step in 0..<20_000 where issue == nil {
            let angle = 2.0 * Double.pi * Double(step) / period
            issue = monitor.observe(State(
                x: 5.0 * cos(angle),
                y: 5.0 * sin(angle),
                z: 0.0
            ))
        }
        #expect(issue == .periodic)
    }

    /// Regression for the reported Chua run: these parameters settle into a
    /// large ring (a stable limit cycle, period ≈ 431.2 steps at dt=0.004)
    /// that the exact-recurrence detector could never see.
    @Test("The reported Chua limit cycle is reported as periodic")
    func reportedChuaRingIsPeriodic() throws {
        let catalog = try AttractorCatalog.bundled()
        let descriptor = try #require(catalog.first { $0.id == "chua" })
        let model = try AttractorModel(
            descriptor: descriptor,
            parameters: ["alpha": 10.164, "beta": 14.932, "m0": -0.499, "m1": 0.257]
        )
        var engine = AttractorEngine(
            model: model,
            initialState: State(x: -0.243, y: 0.468, z: 0.093),
            stepSize: 0.004,
            trailCapacity: 100
        )
        var monitor = SimulationHealthMonitor()

        var issue: SimulationHealthIssue?
        for _ in 0..<60_000 where issue == nil {
            try engine.advance()
            issue = monitor.observe(engine.state)
        }
        #expect(issue == .periodic)
    }

    @Test("Invalid thresholds are sanitized instead of flipping every check")
    func thresholdsAreSanitized() {
        let thresholds = SimulationHealthThresholds(
            warmupSteps: -10,
            collapseWindowSteps: 0,
            collapseSpan: .nan,
            divergenceLimit: -1,
            historyCapacity: 0,
            minimumPeriodSteps: 0,
            periodicValidationWindow: 10_000,
            periodicCheckInterval: 0,
            periodicMeanDistanceFraction: 5,
            hotPhaseSteps: -1,
            warmPhaseSteps: -1,
            hotCheckInterval: 0,
            warmCheckInterval: -5,
            coldCheckInterval: 0
        )
        #expect(thresholds.warmupSteps >= 0)
        #expect(thresholds.collapseWindowSteps >= 1)
        #expect(thresholds.collapseSpan.isFinite && thresholds.collapseSpan >= 0)
        #expect(thresholds.divergenceLimit > 0)
        #expect(thresholds.historyCapacity >= 2)
        #expect(thresholds.minimumPeriodSteps >= 1)
        #expect(thresholds.periodicValidationWindow >= 1)
        #expect(thresholds.periodicValidationWindow < thresholds.historyCapacity)
        #expect(thresholds.periodicCheckInterval >= 1)
        #expect(thresholds.periodicMeanDistanceFraction >= 0)
        #expect(thresholds.periodicMeanDistanceFraction <= 1)
        #expect(thresholds.warmPhaseSteps >= 0)
        #expect(thresholds.hotCheckInterval >= 1)
        #expect(thresholds.warmCheckInterval >= 1)
        #expect(thresholds.coldCheckInterval >= 1)
    }
}
