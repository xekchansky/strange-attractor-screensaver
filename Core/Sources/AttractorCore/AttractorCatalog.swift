import Foundation

/// Error thrown while loading the attractor catalog from the bundle.
public enum AttractorCatalogError: Error, Equatable {
    /// The bundled `attractors.json` resource is missing.
    case resourceMissing
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
}
