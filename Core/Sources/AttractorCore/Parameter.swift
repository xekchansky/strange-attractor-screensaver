import Foundation

/// A tunable coefficient of an attractor's ODE system.
///
/// Each parameter is a named scalar that the formula strings reference as a
/// variable (e.g. `a`, `b`, `r`). At runtime the engine draws a concrete value
/// uniformly from the closed range `min...max`.
public struct Parameter: Codable, Equatable, Sendable {
    /// Variable name used inside the formula strings (e.g. "a").
    public let name: String
    /// Lower bound of the sampled range (inclusive).
    public let min: Double
    /// Upper bound of the sampled range (inclusive).
    public let max: Double

    public init(name: String, min: Double, max: Double) {
        self.name = name
        self.min = min
        self.max = max
    }
}
