import Foundation

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
    public static func select(
        from catalog: [AttractorDescriptor],
        using rng: inout some RandomNumberGenerator
    ) -> AttractorSelection {
        precondition(!catalog.isEmpty, "Randomizer requires a non-empty catalog")
        let descriptor = catalog[Int.random(in: 0..<catalog.count, using: &rng)]
        return select(attractor: descriptor, using: &rng)
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
    public static func select(
        attractor descriptor: AttractorDescriptor,
        using rng: inout some RandomNumberGenerator
    ) -> AttractorSelection {
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
            colorMode: TraceColorMode.allCases.randomElement(using: &rng)!
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
    public static func select(from catalog: [AttractorDescriptor]) -> AttractorSelection {
        var rng = SystemRandomNumberGenerator()
        return select(from: catalog, using: &rng)
    }

    /// Pick parameters and hyperparameters for a fixed attractor using the
    /// system entropy source.
    public static func select(attractor descriptor: AttractorDescriptor) -> AttractorSelection {
        var rng = SystemRandomNumberGenerator()
        return select(attractor: descriptor, using: &rng)
    }
}
