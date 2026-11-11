import Foundation

/// Error thrown when a catalog or a descriptor cannot produce a run.
public enum RandomizerError: Error, Equatable {
    /// The catalog handed to the randomizer is empty.
    case emptyCatalog
    /// A sampling range is malformed: non-finite bounds or `min > max`.
    case invalidRange(field: String, min: Double, max: Double)
}

/// Chooses a random attractor together with concrete parameter values and
/// hyperparameters for a simulation run.
///
/// The randomizer is the entry point the screensaver uses at startup: it
/// consumes the catalog of descriptors and produces a ready-to-run
/// ``AttractorSelection``.
public enum Randomizer {
    /// Pick a random attractor, parameter values, and hyperparameters from
    /// `catalog`.
    ///
    /// Parameter values are drawn uniformly from each declared `[min, max]`
    /// range; hyperparameters are drawn from the descriptor's simulation
    /// ranges.
    ///
    /// - Parameter catalog: The descriptors to choose from. Must not be empty.
    /// - Parameter rng: The random source. Pass a seeded ``SplitMix64`` for a
    ///   reproducible result.
    /// - Throws: ``RandomizerError/emptyCatalog`` or
    ///   ``RandomizerError/invalidRange(field:min:max:)``.
    public static func select(
        from catalog: [AttractorDescriptor],
        using rng: inout some RandomNumberGenerator
    ) throws -> AttractorSelection {
        guard !catalog.isEmpty else { throw RandomizerError.emptyCatalog }
        let descriptor = catalog[Int.random(in: 0..<catalog.count, using: &rng)]
        return try select(attractor: descriptor, using: &rng)
    }

    /// Pick parameter values, hyperparameters, and an initial state for a
    /// *fixed* attractor, sampling everything else from the descriptor's
    /// declared ranges.
    ///
    /// The ranges are validated for chaotic behaviour across the whole declared
    /// parameter span, so a sampled run stays healthy; the midpoint defaults of
    /// ``DebugConfig`` are not — several catalog attractors (Thomas, Chua,
    /// Moore–Spiegel) settle into fixed points or limit cycles exactly at the
    /// midpoints.
    /// - Throws: ``RandomizerError/invalidRange(field:min:max:)`` when a
    ///   declared range is unusable.
    public static func select(
        attractor descriptor: AttractorDescriptor,
        using rng: inout some RandomNumberGenerator
    ) throws -> AttractorSelection {
        for parameter in descriptor.parameters {
            try validate(
                min: parameter.min,
                max: parameter.max,
                field: "parameters.\(parameter.name)"
            )
        }
        try validate(simulation: descriptor.simulation)
        try validate(min: descriptor.initialState.x.min, max: descriptor.initialState.x.max, field: "initialState.x")
        try validate(min: descriptor.initialState.y.min, max: descriptor.initialState.y.max, field: "initialState.y")
        try validate(min: descriptor.initialState.z.min, max: descriptor.initialState.z.max, field: "initialState.z")

        var parameters: [String: Double] = [:]
        for parameter in descriptor.parameters {
            parameters[parameter.name] = Double.random(
                in: parameter.min...parameter.max,
                using: &rng
            )
        }

        let settings = SimulationSettings(
            stepSize: Double.random(in: descriptor.simulation.stepSize.closedRange, using: &rng),
            simulationSpeed: Double.random(in: descriptor.simulation.simulationSpeed.closedRange, using: &rng),
            cameraRotationSpeed: Double.random(in: descriptor.simulation.cameraRotationSpeed.closedRange, using: &rng),
            trailColorSpeed: Double.random(in: descriptor.simulation.trailColorSpeed.closedRange, using: &rng),
            trailLength: Int.random(in: descriptor.simulation.trailLength.closedRange, using: &rng),
            colorMode: TraceColorMode.allCases.randomElement(using: &rng) ?? .trail
        )

        let range = descriptor.initialState
        let initialState = State(
            x: Double.random(in: range.x.closedRange, using: &rng),
            y: Double.random(in: range.y.closedRange, using: &rng),
            z: Double.random(in: range.z.closedRange, using: &rng)
        )

        return AttractorSelection(
            descriptor: descriptor,
            parameters: parameters,
            settings: settings,
            initialState: initialState
        )
    }

    /// Pick a random attractor, parameter values, and hyperparameters using
    /// the system entropy source.
    public static func select(from catalog: [AttractorDescriptor]) throws -> AttractorSelection {
        var rng = SystemRandomNumberGenerator()
        return try select(from: catalog, using: &rng)
    }

    /// Pick parameters and hyperparameters for a fixed attractor using the
    /// system entropy source.
    public static func select(attractor descriptor: AttractorDescriptor) throws -> AttractorSelection {
        var rng = SystemRandomNumberGenerator()
        return try select(attractor: descriptor, using: &rng)
    }

    // MARK: - Range validation

    private static func validate(min: Double, max: Double, field: String) throws {
        guard min.isFinite, max.isFinite, min <= max else {
            throw RandomizerError.invalidRange(field: field, min: min, max: max)
        }
    }

    private static func validate(simulation: SimulationHyperparameters) throws {
        try validate(min: simulation.stepSize.min, max: simulation.stepSize.max, field: "stepSize")
        try validate(min: simulation.simulationSpeed.min, max: simulation.simulationSpeed.max, field: "simulationSpeed")
        try validate(min: simulation.cameraRotationSpeed.min, max: simulation.cameraRotationSpeed.max, field: "cameraRotationSpeed")
        try validate(min: simulation.trailColorSpeed.min, max: simulation.trailColorSpeed.max, field: "trailColorSpeed")
        guard simulation.trailLength.min <= simulation.trailLength.max else {
            throw RandomizerError.invalidRange(
                field: "trailLength",
                min: Double(simulation.trailLength.min),
                max: Double(simulation.trailLength.max)
            )
        }
    }
}
