import Foundation

/// The numerical integration scheme used to advance the ODE system.
public enum IntegrationMethod: Sendable {
    /// First-order explicit Euler method.
    case euler
    /// Fourth-order explicit Runge–Kutta method (RK4).
    case rungeKutta4
}

/// Numerical integration of the attractor's 3D ODE system.
public enum ODESolver {
    /// The vector field of the system: instantaneous derivatives at a state.
    public typealias Rate = (State) throws -> Derivatives

    /// Advance `state` by `dt` using `method`.
    ///
    /// - Parameters:
    ///   - state: The current state.
    ///   - dt: The integration time step.
    ///   - method: The integration scheme.
    ///   - rate: The vector field of the system.
    /// - Returns: The state after one step of length `dt`.
    public static func step(
        from state: State,
        dt: Double,
        method: IntegrationMethod,
        rate: Rate
    ) throws -> State {
        switch method {
        case .euler:
            return try euler(from: state, dt: dt, rate: rate)
        case .rungeKutta4:
            return try rungeKutta4(from: state, dt: dt, rate: rate)
        }
    }

    /// First-order explicit Euler step: `s' = s + h * f(s)`.
    public static func euler(from state: State, dt: Double, rate: Rate) throws -> State {
        let k = try rate(state)
        return State(
            x: state.x + dt * k.dx,
            y: state.y + dt * k.dy,
            z: state.z + dt * k.dz
        )
    }

    /// Classic fourth-order Runge–Kutta step.
    public static func rungeKutta4(from state: State, dt: Double, rate: Rate) throws -> State {
        let k1 = try rate(state)

        let s2 = State(
            x: state.x + 0.5 * dt * k1.dx,
            y: state.y + 0.5 * dt * k1.dy,
            z: state.z + 0.5 * dt * k1.dz
        )
        let k2 = try rate(s2)

        let s3 = State(
            x: state.x + 0.5 * dt * k2.dx,
            y: state.y + 0.5 * dt * k2.dy,
            z: state.z + 0.5 * dt * k2.dz
        )
        let k3 = try rate(s3)

        let s4 = State(
            x: state.x + dt * k3.dx,
            y: state.y + dt * k3.dy,
            z: state.z + dt * k3.dz
        )
        let k4 = try rate(s4)

        return State(
            x: state.x + dt / 6.0 * (k1.dx + 2.0 * k2.dx + 2.0 * k3.dx + k4.dx),
            y: state.y + dt / 6.0 * (k1.dy + 2.0 * k2.dy + 2.0 * k3.dy + k4.dy),
            z: state.z + dt / 6.0 * (k1.dz + 2.0 * k2.dz + 2.0 * k3.dz + k4.dz)
        )
    }
}
