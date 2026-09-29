import Foundation

/// Error thrown while building a debug configuration.
public enum DebugConfigError: Error, Equatable {
    /// No descriptor with the requested id exists in the catalog.
    case unknownAttractor(String)
}

/// Builds a concrete ``AttractorSelection`` by hand for debugging.
///
/// Unlike ``Randomizer`` (which samples everything randomly), ``DebugConfig``
/// lets the demo app pin specific parameter and hyperparameter values and fills
/// any omitted value with the midpoint of its declared range.
public enum DebugConfig {
    /// Small deterministic offset applied to the initial state so it never sits
    /// exactly on the range midpoint. For center-symmetric ranges (Thomas,
    /// Lorenz) the midpoint is an equilibrium point where every derivative is
    /// zero, so a start at the exact midpoint produces no motion at all.
    private static let initialStateOffset = State(x: 0.01, y: -0.01, z: 0.01)

    /// Build a selection for the attractor with `id`.
    ///
    /// - Parameters:
    ///   - id: The `id` of a descriptor in `catalog`.
    ///   - catalog: The descriptors to look up.
    ///   - parameters: Explicit parameter values keyed by name. Omitted
    ///     parameters default to the midpoint of their declared range.
    ///   - settings: Explicit hyperparameters. When `nil`, every field defaults
    ///     to the midpoint of its range.
    /// - Throws: ``DebugConfigError/unknownAttractor`` if no descriptor with
    ///   `id` exists in `catalog`.
    public static func selection(
        for id: String,
        in catalog: [AttractorDescriptor],
        parameters: [String: Double] = [:],
        settings: SimulationSettings? = nil
    ) throws -> AttractorSelection {
        guard let descriptor = catalog.first(where: { $0.id == id }) else {
            throw DebugConfigError.unknownAttractor(id)
        }

        var concreteParameters = parameters
        for parameter in descriptor.parameters where concreteParameters[parameter.name] == nil {
            concreteParameters[parameter.name] = (parameter.min + parameter.max) / 2
        }

        let concreteSettings = settings ?? SimulationSettings(midpointsOf: descriptor.simulation)
        let initialState = State(
            x: Self.midpoint(of: descriptor.initialState.x) + initialStateOffset.x,
            y: Self.midpoint(of: descriptor.initialState.y) + initialStateOffset.y,
            z: Self.midpoint(of: descriptor.initialState.z) + initialStateOffset.z
        )
        return AttractorSelection(
            descriptor: descriptor,
            parameters: concreteParameters,
            settings: concreteSettings,
            initialState: initialState
        )
    }

    private static func midpoint(of range: MinMax<Double>) -> Double {
        (range.min + range.max) / 2
    }
}
