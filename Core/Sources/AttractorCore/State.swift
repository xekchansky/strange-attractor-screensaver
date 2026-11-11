import Foundation

/// A single point in the 3D state space of an attractor's ODE system.
public struct State: Equatable, Sendable {
    /// Position along the first axis of the attractor's phase space.
    public var x: Double
    /// Position along the second axis.
    public var y: Double
    /// Position along the third axis.
    public var z: Double

    public init(x: Double, y: Double, z: Double) {
        self.x = x
        self.y = y
        self.z = z
    }
}

/// The instantaneous rates of change of the state variables — the right-hand
/// side of the ODE system evaluated at a given state and parameter set.
public struct Derivatives: Equatable, Sendable {
    /// Rate of change of ``State/x``.
    public var dx: Double
    /// Rate of change of ``State/y``.
    public var dy: Double
    /// Rate of change of ``State/z``.
    public var dz: Double

    public init(dx: Double, dy: Double, dz: Double) {
        self.dx = dx
        self.dy = dy
        self.dz = dz
    }
}
