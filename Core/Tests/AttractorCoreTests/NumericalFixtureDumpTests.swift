import Testing
import Foundation
@testable import AttractorCore

/// Golden-fixture generator for the Rust numerical core.
///
/// Disabled unless `ATTRACTOR_DUMP_FIXTURES=1`, so a normal `swift test` run
/// never writes files. The output is committed under
/// `rust/crates/attractor-core/tests/fixtures/` and consumed by the Rust
/// `trajectory_fixtures.rs` and `randomizer_fixtures.rs` tests; see the README
/// next to the fixtures for the exact command.
@Suite(
    "Numerical fixture dump",
    .enabled(if: ProcessInfo.processInfo.environment["ATTRACTOR_DUMP_FIXTURES"] == "1")
)
struct NumericalFixtureDumpTests {
    /// Integration steps recorded for every Runge–Kutta trajectory.
    private static let trajectorySteps = 1000
    /// Integration steps recorded for the extra Euler trajectory.
    private static let eulerSteps = 500
    /// Number of seeds recorded for the randomizer fixture.
    private static let selectionSeeds = 0..<16

    @Test("Writes golden trajectories for Lorenz, Chua and Aizawa")
    func dumpTrajectories() throws {
        let catalog = try AttractorCatalog.bundled()
        var entries: [[String: Any]] = []

        for id in ["lorenz", "chua", "aizawa"] {
            let selection = try DebugConfig.selection(for: id, in: catalog)
            let model = try selection.buildModel()
            let runs: [(name: String, method: IntegrationMethod, steps: Int)] = [
                ("rungeKutta4", .rungeKutta4, Self.trajectorySteps),
                // Euler is first order, so it gets a shorter horizon: it is a
                // stepping-scheme witness, not an accuracy reference.
                ("euler", .euler, id == "lorenz" ? Self.eulerSteps : 0),
            ]

            for run in runs where run.steps > 0 {
                var state = selection.initialState
                var states = [Self.stateDocument(state)]
                for _ in 0..<run.steps {
                    state = try ODESolver.step(
                        from: state,
                        dt: selection.settings.stepSize,
                        method: run.method,
                        rate: { try model.rate(of: $0) }
                    )
                    states.append(Self.stateDocument(state))
                }
                entries.append([
                    "attractor": id,
                    "method": run.name,
                    "stepSize": Self.exact(selection.settings.stepSize),
                    "parameters": selection.parameters.mapValues { Self.exact($0) },
                    "initialState": Self.stateDocument(selection.initialState),
                    "states": states,
                ])
            }
        }

        try Self.write(
            [
                "generator": "Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift",
                "entries": entries,
            ],
            to: "trajectories.json"
        )
    }

    @Test("Writes seeded randomizer selections")
    func dumpSelections() throws {
        let catalog = try AttractorCatalog.bundled()
        var entries: [[String: Any]] = []

        for seed in Self.selectionSeeds {
            var rng = SplitMix64(seed: UInt64(seed))
            let selection = try Randomizer.select(from: catalog, using: &rng)
            let settings = selection.settings
            entries.append([
                "seed": seed,
                "attractor": selection.descriptor.id,
                "parameters": selection.parameters.mapValues { Self.exact($0) },
                "settings": [
                    "stepSize": Self.exact(settings.stepSize),
                    "simulationSpeed": Self.exact(settings.simulationSpeed),
                    "cameraRotationSpeed": Self.exact(settings.cameraRotationSpeed),
                    "trailColorSpeed": Self.exact(settings.trailColorSpeed),
                    "trailLength": settings.trailLength,
                    "colorMode": settings.colorMode.rawValue,
                ],
                "initialState": Self.stateDocument(selection.initialState),
            ])
        }

        try Self.write(
            [
                "generator": "Core/Tests/AttractorCoreTests/NumericalFixtureDumpTests.swift",
                "entries": entries,
            ],
            to: "randomizer_selections.json"
        )
    }

    private static func stateDocument(_ state: State) -> [String: String] {
        ["x": Self.exact(state.x), "y": Self.exact(state.y), "z": Self.exact(state.z)]
    }

    /// Render a double with enough digits to round-trip exactly.
    private static func exact(_ value: Double) -> String {
        String(format: "%.17g", value)
    }

    private static func write(_ document: [String: Any], to name: String) throws {
        let data = try JSONSerialization.data(
            withJSONObject: document,
            options: [.prettyPrinted, .sortedKeys]
        )
        let url = Self.fixtureDirectory().appendingPathComponent(name)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try data.write(to: url, options: .atomic)
        print("Wrote \(url.path)")
    }

    /// `<repo>/rust/crates/attractor-core/tests/fixtures`.
    private static func fixtureDirectory() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()  // AttractorCoreTests
            .deletingLastPathComponent()  // Tests
            .deletingLastPathComponent()  // Core
            .deletingLastPathComponent()  // repository root
            .appendingPathComponent("rust/crates/attractor-core/tests/fixtures")
    }
}
