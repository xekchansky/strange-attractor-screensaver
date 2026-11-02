import Testing
@testable import AttractorCore

@Suite("Simulation step scheduler")
struct SimulationStepSchedulerTests {
    @Test("Converts a normal frame interval into whole steps")
    func normalFrameInterval() {
        var scheduler = SimulationStepScheduler(stepSize: 0.005, simulationSpeed: 2)
        // 1/60 s × 2 = 0.0333… simulated time → six whole 0.005 steps.
        #expect(scheduler.steps(forFrameInterval: 1.0 / 60.0) == 6)
    }

    @Test("Accumulates fractional steps across frames")
    func accumulatesRemainders() {
        var scheduler = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: 1)
        #expect(scheduler.steps(forFrameInterval: 0.004) == 0)
        #expect(scheduler.steps(forFrameInterval: 0.004) == 0)
        #expect(scheduler.steps(forFrameInterval: 0.004) == 1)
    }

    @Test("Clamps a long stall to the maximum frame interval")
    func clampsLongFrameInterval() {
        var scheduler = SimulationStepScheduler(
            stepSize: 0.005,
            simulationSpeed: 2,
            maximumFrameInterval: 0.1
        )
        // A 60-second stall (sleep/wake) is treated as 0.1 s → 40 steps.
        #expect(scheduler.steps(forFrameInterval: 60) == 40)
    }

    @Test("Never exceeds the per-frame step cap")
    func capsStepsPerFrame() {
        var scheduler = SimulationStepScheduler(
            stepSize: 0.001,
            simulationSpeed: 3,
            maximumFrameInterval: 0.1,
            maximumStepsPerFrame: 100
        )
        #expect(scheduler.steps(forFrameInterval: 0.1) == 100)
    }

    @Test("Drops the backlog after a capped frame")
    func dropsBacklogAfterCap() {
        var scheduler = SimulationStepScheduler(
            stepSize: 0.01,
            simulationSpeed: 1,
            maximumFrameInterval: 1,
            maximumStepsPerFrame: 10
        )
        #expect(scheduler.steps(forFrameInterval: 0.5) == 10)
        #expect(scheduler.accumulatedTime == 0)
        #expect(scheduler.steps(forFrameInterval: 0.05) == 5)
    }

    @Test("A non-positive step size produces no steps")
    func nonPositiveStepSize() {
        var zero = SimulationStepScheduler(stepSize: 0, simulationSpeed: 1)
        #expect(zero.steps(forFrameInterval: 1.0 / 60.0) == 0)

        var negative = SimulationStepScheduler(stepSize: -0.01, simulationSpeed: 1)
        #expect(negative.steps(forFrameInterval: 1.0 / 60.0) == 0)
    }

    @Test("Non-finite inputs produce no steps")
    func nonFiniteInputs() {
        var scheduler = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: 1)
        #expect(scheduler.steps(forFrameInterval: .nan) == 0)
        #expect(scheduler.steps(forFrameInterval: .infinity) == 0)

        var fast = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: .nan)
        #expect(fast.steps(forFrameInterval: 1.0 / 60.0) == 0)

        var broken = SimulationStepScheduler(stepSize: .infinity, simulationSpeed: 1)
        #expect(broken.steps(forFrameInterval: 1.0 / 60.0) == 0)
    }

    @Test("Non-positive frame intervals produce no steps")
    func nonPositiveFrameIntervals() {
        var scheduler = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: 1)
        #expect(scheduler.steps(forFrameInterval: 0) == 0)
        #expect(scheduler.steps(forFrameInterval: -1) == 0)
    }

    @Test("A non-positive simulation speed produces no steps")
    func nonPositiveSimulationSpeed() {
        var stopped = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: 0)
        #expect(stopped.steps(forFrameInterval: 1.0 / 60.0) == 0)

        var reversed = SimulationStepScheduler(stepSize: 0.01, simulationSpeed: -2)
        #expect(reversed.steps(forFrameInterval: 1.0 / 60.0) == 0)
    }
}
