import Foundation

/// Error produced while building an ``AttractorModel``.
public enum AttractorModelError: Error, Equatable {
    /// A parameter declared in the descriptor has no supplied value.
    case missingParameter(String)
}

/// A concrete, evaluable attractor: the `dX`/`dY`/`dZ` formulas parsed into
/// expression trees and bound to the state coordinates and a fixed set of
/// parameter values.
public struct AttractorModel: Sendable {
    /// The descriptor the model was built from.
    public let descriptor: AttractorDescriptor
    /// Concrete parameter values keyed by the parameter name.
    public let parameters: [String: Double]

    private let dx: Expression
    private let dy: Expression
    private let dz: Expression

    /// Build an evaluable model from a descriptor and concrete parameters.
    ///
    /// - Parameters:
    ///   - descriptor: The attractor definition from the config.
    ///   - parameters: Concrete parameter values keyed by name. Must cover every
    ///     parameter declared in the descriptor.
    /// - Throws: ``FormulaError`` if a formula cannot be parsed, or
    ///   ``AttractorModelError/missingParameter`` when a declared parameter is
    ///   not supplied.
    public init(descriptor: AttractorDescriptor, parameters: [String: Double]) throws {
        for parameter in descriptor.parameters where parameters[parameter.name] == nil {
            throw AttractorModelError.missingParameter(parameter.name)
        }
        self.descriptor = descriptor
        self.parameters = parameters
        self.dx = try FormulaParser.parse(descriptor.dx)
        self.dy = try FormulaParser.parse(descriptor.dy)
        self.dz = try FormulaParser.parse(descriptor.dz)
    }

    /// The vector field evaluated at `state`.
    ///
    /// The parameters stored in the model, together with the coordinates of
    /// `state`, are available as variables in the formulas.
    public func rate(of state: State) throws -> Derivatives {
        var variables = parameters
        variables["x"] = state.x
        variables["y"] = state.y
        variables["z"] = state.z
        return Derivatives(
            dx: try dx.evaluate(variables: variables),
            dy: try dy.evaluate(variables: variables),
            dz: try dz.evaluate(variables: variables)
        )
    }
}
