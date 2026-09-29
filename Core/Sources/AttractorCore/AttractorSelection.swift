import Foundation

/// A complete, concrete set of choices for one simulation run: the attractor
/// descriptor together with sampled parameter values and concrete
/// hyperparameters.
///
/// This is what ``Randomizer`` produces at startup (and ``DebugConfig`` builds
/// by hand): everything `AttractorEngine` needs to construct and run a
/// simulation.
public struct AttractorSelection: Equatable, Sendable {
    /// The chosen attractor definition.
    public let descriptor: AttractorDescriptor
    /// Concrete parameter values keyed by parameter name. Covers every
    /// parameter declared in ``descriptor``.
    public let parameters: [String: Double]
    /// Concrete simulation/visualization hyperparameters.
    public let settings: SimulationSettings

    public init(
        descriptor: AttractorDescriptor,
        parameters: [String: Double],
        settings: SimulationSettings
    ) {
        self.descriptor = descriptor
        self.parameters = parameters
        self.settings = settings
    }

    /// Turn the selection into an evaluable attractor model.
    ///
    /// - Throws: ``FormulaError`` if a formula cannot be parsed, or
    ///   ``AttractorModelError/missingParameter`` when a declared parameter is
    ///   not present in ``parameters``.
    public func buildModel() throws -> AttractorModel {
        try AttractorModel(descriptor: descriptor, parameters: parameters)
    }
}
