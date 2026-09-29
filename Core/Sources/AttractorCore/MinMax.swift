import Foundation

/// A closed numeric range expressed declaratively as `min`/`max`, so it can be
/// stored in and loaded from a JSON config just like any other ``Codable``
/// value. Used by configuration types that model sampling ranges.
public struct MinMax<Value: Codable & Equatable & Sendable>: Codable, Equatable, Sendable {
    /// Lower bound (inclusive).
    public var min: Value
    /// Upper bound (inclusive).
    public var max: Value

    public init(min: Value, max: Value) {
        self.min = min
        self.max = max
    }
}

extension MinMax where Value: Comparable {
    /// The range as a native `ClosedRange`.
    public var closedRange: ClosedRange<Value> {
        min...max
    }
}
