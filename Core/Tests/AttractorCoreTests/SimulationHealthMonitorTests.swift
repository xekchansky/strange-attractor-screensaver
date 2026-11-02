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
            minimumPeriodSteps: 64,
            segmentLength: 8
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
    @Test("Chua's slow double-scroll start is not flagged")
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

        for _ in 0..<20_000 {
            try engine.advance()
            let issue = monitor.observe(engine.state)
            #expect(issue == nil, "chua: \(String(describing: issue))")
            if issue != nil { break }
        }
    }
}
