import Foundation

/// Error thrown while loading the attractor catalog from the bundle.
public enum AttractorCatalogError: Error, Equatable {
    /// The bundled `attractors.json` resource is missing.
    case resourceMissing
}

/// One problem found while validating an attractor descriptor.
public struct AttractorValidationIssue: Equatable, Sendable {
    /// `id` of the descriptor the issue belongs to (empty for catalog-wide issues).
    public let descriptorID: String
    /// Field the issue is about, e.g. `"dx"`, `"stepSize"`, `"parameters"`.
    public let field: String
    /// Human-readable description of the problem.
    public let message: String

    public init(descriptorID: String, field: String, message: String) {
        self.descriptorID = descriptorID
        self.field = field
        self.message = message
    }
}

/// Result of validating a catalog: descriptors safe to run and the issues that
/// caused the rest to be dropped.
public struct AttractorCatalogValidation: Equatable, Sendable {
    /// Descriptors that passed every check, in catalog order.
    public let valid: [AttractorDescriptor]
    /// Problems found in the rejected descriptors (plus catalog-wide issues).
    public let issues: [AttractorValidationIssue]

    public init(valid: [AttractorDescriptor], issues: [AttractorValidationIssue]) {
        self.valid = valid
        self.issues = issues
    }
}

/// Loads ``[AttractorDescriptor]`` from the module's bundled `attractors.json`.
///
/// The catalog is the single source describing every available attractor: the
/// randomizer and the debug tooling select from it at startup, and adding an
/// attractor never requires touching code — only extending the JSON resource.
public enum AttractorCatalog {
    /// Decode the bundled attractor catalog.
    ///
    /// - Returns: The decoded descriptors, in the order they appear in the file.
    /// - Throws: ``AttractorCatalogError/resourceMissing`` if the resource is
    ///   absent, or a `DecodingError` when the file is malformed.
    public static func bundled() throws -> [AttractorDescriptor] {
        guard let url = Bundle.module.url(forResource: "attractors", withExtension: "json") else {
            throw AttractorCatalogError.resourceMissing
        }
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode([AttractorDescriptor].self, from: data)
    }

    /// Decode the bundled catalog and validate it in one step.
    ///
    /// - Throws: ``AttractorCatalogError/resourceMissing`` or a `DecodingError`
    ///   when the bundled resource cannot be read; validation problems are
    ///   reported in the result rather than thrown.
    public static func bundledValidation() throws -> AttractorCatalogValidation {
        validate(try bundled())
    }

    /// Check every descriptor and keep only the ones that can be run safely.
    ///
    /// A single malformed entry must not blank the whole screen saver, so
    /// invalid descriptors are dropped and reported instead of aborting the
    /// catalog. Checks cover range sanity, duplicate ids/parameters, formula
    /// syntax and formula variables that the descriptor does not declare.
    public static func validate(_ catalog: [AttractorDescriptor]) -> AttractorCatalogValidation {
        var valid: [AttractorDescriptor] = []
        var issues: [AttractorValidationIssue] = []
        var seenIDs: Set<String> = []

        if catalog.isEmpty {
            issues.append(AttractorValidationIssue(
                descriptorID: "",
                field: "catalog",
                message: "catalog is empty"
            ))
        }

        for descriptor in catalog {
            let descriptorIssues = validationIssues(for: descriptor, seenIDs: &seenIDs)
            if descriptorIssues.isEmpty {
                valid.append(descriptor)
            } else {
                issues.append(contentsOf: descriptorIssues)
            }
        }

        return AttractorCatalogValidation(valid: valid, issues: issues)
    }

    /// Variables every formula may reference in addition to declared parameters.
    private static let stateVariables: Set<String> = ["x", "y", "z"]

    private static func validationIssues(
        for descriptor: AttractorDescriptor,
        seenIDs: inout Set<String>
    ) -> [AttractorValidationIssue] {
        var issues: [AttractorValidationIssue] = []
        func add(_ field: String, _ message: String) {
            issues.append(AttractorValidationIssue(
                descriptorID: descriptor.id,
                field: field,
                message: message
            ))
        }

        let id = descriptor.id.trimmingCharacters(in: .whitespacesAndNewlines)
        if id.isEmpty {
            add("id", "id must not be empty")
        } else if !seenIDs.insert(id).inserted {
            add("id", "duplicate id '\(id)'")
        }

        var parameterNames: Set<String> = []
        for parameter in descriptor.parameters {
            let name = parameter.name
            if name.isEmpty {
                add("parameters", "parameter name must not be empty")
                continue
            }
            if stateVariables.contains(name) {
                add("parameters", "parameter '\(name)' collides with a state coordinate")
            }
            if !parameterNames.insert(name).inserted {
                add("parameters", "duplicate parameter '\(name)'")
            }
            appendDoubleRangeIssues(
                &issues,
                descriptorID: descriptor.id,
                field: "parameters.\(name)",
                min: parameter.min,
                max: parameter.max
            )
        }

        let allowedVariables = stateVariables.union(parameterNames)
        appendFormulaIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "dx",
            formula: descriptor.dx,
            allowedVariables: allowedVariables
        )
        appendFormulaIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "dy",
            formula: descriptor.dy,
            allowedVariables: allowedVariables
        )
        appendFormulaIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "dz",
            formula: descriptor.dz,
            allowedVariables: allowedVariables
        )

        let simulation = descriptor.simulation
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "stepSize",
            min: simulation.stepSize.min,
            max: simulation.stepSize.max,
            lowerBound: 0,
            lowerBoundIsExclusive: true
        )
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "simulationSpeed",
            min: simulation.simulationSpeed.min,
            max: simulation.simulationSpeed.max,
            lowerBound: 0,
            lowerBoundIsExclusive: true
        )
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "cameraRotationSpeed",
            min: simulation.cameraRotationSpeed.min,
            max: simulation.cameraRotationSpeed.max,
            lowerBound: 0
        )
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "trailColorSpeed",
            min: simulation.trailColorSpeed.min,
            max: simulation.trailColorSpeed.max,
            lowerBound: 0
        )
        if simulation.trailLength.min > simulation.trailLength.max {
            add(
                "trailLength",
                "min \(simulation.trailLength.min) is greater than max \(simulation.trailLength.max)"
            )
        }
        if simulation.trailLength.min < 2 {
            add("trailLength", "must keep at least 2 points")
        }

        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "initialState.x",
            min: descriptor.initialState.x.min,
            max: descriptor.initialState.x.max
        )
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "initialState.y",
            min: descriptor.initialState.y.min,
            max: descriptor.initialState.y.max
        )
        appendDoubleRangeIssues(
            &issues,
            descriptorID: descriptor.id,
            field: "initialState.z",
            min: descriptor.initialState.z.min,
            max: descriptor.initialState.z.max
        )

        return issues
    }

    private static func appendDoubleRangeIssues(
        _ issues: inout [AttractorValidationIssue],
        descriptorID: String,
        field: String,
        min: Double,
        max: Double,
        lowerBound: Double? = nil,
        lowerBoundIsExclusive: Bool = false
    ) {
        func add(_ message: String) {
            issues.append(AttractorValidationIssue(
                descriptorID: descriptorID,
                field: field,
                message: message
            ))
        }

        guard min.isFinite, max.isFinite else {
            add("bounds must be finite")
            return
        }
        guard min <= max else {
            add("min \(min) is greater than max \(max)")
            return
        }
        if let lowerBound {
            let violates = lowerBoundIsExclusive ? min <= lowerBound : min < lowerBound
            if violates {
                add(lowerBoundIsExclusive
                    ? "min \(min) must be greater than \(lowerBound)"
                    : "min \(min) must be at least \(lowerBound)")
            }
        }
    }

    private static func appendFormulaIssues(
        _ issues: inout [AttractorValidationIssue],
        descriptorID: String,
        field: String,
        formula: String,
        allowedVariables: Set<String>
    ) {
        func add(_ message: String) {
            issues.append(AttractorValidationIssue(
                descriptorID: descriptorID,
                field: field,
                message: message
            ))
        }

        let expression: Expression
        do {
            expression = try FormulaParser.parse(formula)
        } catch {
            add("cannot parse formula: \(error)")
            return
        }

        let unknown = expression.referencedVariables.subtracting(allowedVariables)
        if !unknown.isEmpty {
            add("unknown variable(s): \(unknown.sorted().joined(separator: ", "))")
        }
    }
}
