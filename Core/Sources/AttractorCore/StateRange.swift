import Foundation

/// A range for each coordinate of a simulation's initial state point.
///
/// Stored declaratively (just like ``SimulationHyperparameters``) so the
/// randomizer can draw a concrete initial ``State`` from it at startup, and
/// ``DebugConfig`` can pin it explicitly.
public struct StateRange: Codable, Equatable, Sendable {
    /// Sampling range for the `x` coordinate.
    public var x: MinMax<Double>
    /// Sampling range for the `y` coordinate.
    public var y: MinMax<Double>
    /// Sampling range for the `z` coordinate.
    public var z: MinMax<Double>

    public init(x: MinMax<Double>, y: MinMax<Double>, z: MinMax<Double>) {
        self.x = x
        self.y = y
        self.z = z
    }
}
