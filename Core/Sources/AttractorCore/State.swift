import Foundation

/// A single point in the 3D state space of an attractor's ODE system.
public struct State: Equatable, Sendable {
    public var x: Double
    public var y: Double
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
    public var dx: Double
    public var dy: Double
    public var dz: Double

    public init(dx: Double, dy: Double, dz: Double) {
        self.dx = dx
        self.dy = dy
        self.dz = dz
    }
}
