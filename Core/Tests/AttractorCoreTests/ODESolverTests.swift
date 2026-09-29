import Testing
import Foundation
@testable import AttractorCore

@Suite("ODESolver")
struct ODESolverTests {
    /// A vector field that is constant on the whole state space.
    private func constantField() -> ODESolver.Rate {
        { _ in Derivatives(dx: 2, dy: -1, dz: 0.5) }
    }

    /// A diagonal flow `x' = -x`, `y' = 0`, `z' = 0` with the reference
    /// solution `x(t) = exp(-t)`.
    private func decayField() -> ODESolver.Rate {
        { state in Derivatives(dx: -state.x, dy: 0, dz: 0) }
    }

    @Test("Euler integrates a constant field exactly")
    func eulerConstant() throws {
        let after = try ODESolver.euler(from: State(x: 0, y: 0, z: 0), dt: 0.1, rate: constantField())
        #expect(after == State(x: 0.2, y: -0.1, z: 0.05))
    }

    @Test("RK4 integrates a constant field exactly")
    func rk4Constant() throws {
        let after = try ODESolver.rungeKutta4(from: State(x: 0, y: 0, z: 0), dt: 0.1, rate: constantField())
        #expect(after == State(x: 0.2, y: -0.1, z: 0.05))
    }

    @Test("RK4 is more accurate than Euler on the exponential decay")
    func accuracy() throws {
        let dt = 0.01
        let steps = 200
        let finalTime = Double(steps) * dt

        func integrate(_ method: IntegrationMethod) throws -> Double {
            var state = State(x: 1, y: 0, z: 0)
            for _ in 0..<steps {
                state = try ODESolver.step(from: state, dt: dt, method: method, rate: decayField())
            }
            return state.x
        }

        let exact = exp(-finalTime)
        let euler = try integrate(.euler)
        let rk4 = try integrate(.rungeKutta4)

        #expect(abs(euler - exact) > 1e-8)
        #expect(abs(rk4 - exact) < abs(euler - exact))
        #expect(abs(rk4 - exact) < 1e-5)
    }

    @Test("RK4 error scales as fourth order in the step size")
    func rk4Order() throws {
        func error(with step: Double) throws -> Double {
            var state = State(x: 1, y: 0, z: 0)
            let steps = Int(1.0 / step)
            for _ in 0..<steps {
                state = try ODESolver.rungeKutta4(from: state, dt: step, rate: decayField())
            }
            return abs(state.x - exp(-1.0))
        }

        let coarse = try error(with: 0.1)
        let fine = try error(with: 0.05)
        let ratio = coarse / fine
        // A fourth-order method roughly quarters the error per halving of step
        // (~16x), so the observed ratio should sit comfortably above 4.
        #expect((4...80).contains(ratio))
    }
}
