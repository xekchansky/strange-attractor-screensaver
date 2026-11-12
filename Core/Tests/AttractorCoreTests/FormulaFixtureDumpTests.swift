import Testing
import Foundation
@testable import AttractorCore

/// Golden-fixture generator for the Rust formula evaluator.
///
/// Disabled unless `ATTRACTOR_DUMP_FIXTURES=1`, so a normal `swift test` run
/// never writes files. The output is committed under
/// `rust/crates/attractor-core/tests/fixtures/` and consumed by
/// `formula_fixtures.rs`; see the README next to the fixtures for the exact
/// command.
@Suite(
    "Formula fixture dump",
    .enabled(if: ProcessInfo.processInfo.environment["ATTRACTOR_DUMP_FIXTURES"] == "1")
)
struct FormulaFixtureDumpTests {
    /// Number of sample states recorded per formula.
    private static let statesPerFormula = 8

    @Test("Writes golden evaluation fixtures for every catalog formula")
    func dump() throws {
        let descriptors = try AttractorCatalog.bundled()
        var entries: [[String: Any]] = []

        for descriptor in descriptors {
            let parameters = descriptor.parameters.reduce(into: [String: Double]()) {
                $0[$1.name] = ($1.min + $1.max) / 2
            }
            for index in 0..<Self.statesPerFormula {
                let state = Self.sampleState(index)
                var variables = parameters
                variables["x"] = state.x
                variables["y"] = state.y
                variables["z"] = state.z

                for (field, source) in [
                    ("dx", descriptor.dx),
                    ("dy", descriptor.dy),
                    ("dz", descriptor.dz),
                ] {
                    let value = try FormulaParser.parse(source).evaluate(variables: variables)
                    guard value.isFinite else {
                        Issue.record(
                            "\(descriptor.id).\(field) is not finite at state \(index)"
                        )
                        continue
                    }
                    entries.append([
                        "attractor": descriptor.id,
                        "field": field,
                        "expression": source,
                        "bindings": variables.mapValues { Self.exact($0) },
                        "expected": Self.exact(value),
                    ])
                }
            }
        }

        let document: [String: Any] = [
            "generator": "Core/Tests/AttractorCoreTests/FormulaFixtureDumpTests.swift",
            "statesPerFormula": Self.statesPerFormula,
            "entries": entries,
        ]
        let data = try JSONSerialization.data(
            withJSONObject: document,
            options: [.prettyPrinted, .sortedKeys]
        )
        let url = Self.fixtureURL()
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: url, options: .atomic)
        print("Wrote \(entries.count) fixtures to \(url.path)")
    }

    /// Deterministic sample points: old callers rely on the values stored in
    /// the fixture, so changing this scheme means regenerating the file.
    private static func sampleState(_ index: Int) -> (x: Double, y: Double, z: Double) {
        let i = Double(index)
        return (x: 1.3 * i - 3.7, y: 0.7 * i * i - 2.0, z: 0.5 * i + 0.25)
    }

    /// Render a double with enough digits to round-trip exactly.
    private static func exact(_ value: Double) -> String {
        String(format: "%.17g", value)
    }

    /// `<repo>/rust/crates/attractor-core/tests/fixtures/formula_evaluation.json`.
    private static func fixtureURL() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()  // AttractorCoreTests
            .deletingLastPathComponent()  // Tests
            .deletingLastPathComponent()  // Core
            .deletingLastPathComponent()  // repository root
            .appendingPathComponent("rust/crates/attractor-core/tests/fixtures/formula_evaluation.json")
    }
}
