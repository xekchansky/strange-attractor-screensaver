import Foundation
import SceneKit
import Testing
@testable import AttractorRender
import AttractorCore

/// Golden-fixture generator for the Rust view layer.
///
/// Disabled unless `ATTRACTOR_DUMP_FIXTURES=1`, so a normal `swift test` run
/// never writes files. The output is committed under
/// `rust/crates/attractor-view/tests/fixtures/` and consumed by the Rust
/// `view_fixtures.rs` test; see the README next to the fixtures for the exact
/// command.
@Suite(
    "View fixture dump",
    .enabled(if: ProcessInfo.processInfo.environment["ATTRACTOR_DUMP_FIXTURES"] == "1")
)
struct ViewFixtureDumpTests {
    /// Pinned orbit angle for the camera fixture. The screensaver starts at a
    /// random angle, but a capture pins one so the result is reproducible.
    private static let cameraAzimuth = 0.7
    /// Frames recorded for the camera fixture. Fourteen covers two full
    /// framing-refresh strides (frames 1 and 7 are replenished, frame 13 is
    /// due next) and keeps the easing visibly in motion.
    private static let cameraFrames = 14
    /// Real frame interval fed to every camera update, in seconds.
    private static let frameInterval = 1.0 / 60.0
    /// Orbit angular velocity, in radians per second.
    private static let rotationSpeed = 0.35
    /// Gradient offset applied to the animated color modes.
    private static let trailColorOffset = 0.3

    @Test("Writes camera and trail geometry fixtures")
    func dumpViewFixtures() throws {
        let cameraTrace = Self.cameraTrace()
        let trailTrace = Self.trailTrace()

        let controller = CameraController(
            azimuth: Self.cameraAzimuth,
            elevation: Double.pi / 5
        )
        let node = SCNNode()
        var frames: [[String: Any]] = []
        for _ in 0..<Self.cameraFrames {
            controller.update(
                dt: Self.frameInterval,
                rotationSpeed: Self.rotationSpeed,
                around: cameraTrace,
                cameraNode: node
            )
            frames.append([
                "position": Self.vectorDocument(node.position),
            ])
        }

        let cameraPosition = SCNVector3(4.0, 3.0, 5.0)
        let renderer = TraceRenderer()
        let cases: [(TraceColorMode, Double)] = [
            (.trail, 0.0),
            (.trail, Self.trailColorOffset),
            (.age, Self.trailColorOffset),
            (.depth, Self.trailColorOffset),
            (.spatial, Self.trailColorOffset),
        ]
        var trailCases: [[String: Any]] = []
        for (mode, offset) in cases {
            let geometry = renderer.geometry(
                for: trailTrace,
                colorOffset: offset,
                mode: mode,
                cameraPosition: cameraPosition
            )
            let vertexSource = geometry.sources.first { $0.semantic == .vertex }!
            let colorSource = geometry.sources.first { $0.semantic == .color }!
            let element = geometry.elements[0]
            // Every source this renderer builds is a packed Float32 array.
            precondition(vertexSource.bytesPerComponent == 4)
            precondition(colorSource.bytesPerComponent == 4)
            trailCases.append([
                "mode": mode.rawValue,
                "colorOffset": Self.exact(offset),
                "vertexCount": vertexSource.vectorCount,
                "positions": Self.floats(vertexSource).map { Self.exact(Double($0)) },
                "colors": Self.floats(colorSource).map { Self.exact(Double($0)) },
                "indices": Self.indices(element).map(Int.init),
                "primitiveCount": element.primitiveCount,
            ])
        }

        try Self.write(
            [
                "generator": "Render/Tests/AttractorRenderTests/ViewFixtureDumpTests.swift",
                "camera": [
                    "azimuth": Self.exact(Self.cameraAzimuth),
                    "elevation": Self.exact(Double.pi / 5),
                    "dt": Self.exact(Self.frameInterval),
                    "rotationSpeed": Self.exact(Self.rotationSpeed),
                    "states": cameraTrace.map(Self.stateDocument),
                    "frames": frames,
                ],
                "trail": [
                    "cameraPosition": Self.vectorDocument(cameraPosition),
                    "states": trailTrace.map(Self.stateDocument),
                    "cases": trailCases,
                ],
            ],
            to: "view_fixtures.json"
        )
    }

    /// A short, off-center, asymmetric trajectory so the framing scan and the
    /// per-axis normalization see distinct extremes on every axis.
    private static func cameraTrace() -> TraceBuffer {
        var trace = TraceBuffer(
            capacity: 64,
            initial: State(x: 2.0, y: 0.5, z: -0.75)
        )
        for step in 1..<9 {
            let i = Double(step)
            trace.append(
                State(
                    x: 3.0 * cos(i * 0.7) - 1.0,
                    y: 1.5 * sin(i * 1.3) + 0.5,
                    z: 2.0 * i / 8.0 - 0.75
                )
            )
        }
        return trace
    }

    /// Seven points along a helix with drifting amplitude: enough to exercise
    /// the ribbon topology without a large fixture.
    private static func trailTrace() -> TraceBuffer {
        var trace = TraceBuffer(
            capacity: 64,
            initial: State(x: -1.5, y: 0.2, z: cos(0.0) * 1.2)
        )
        for step in 1..<7 {
            let i = Double(step)
            trace.append(
                State(
                    x: i * 0.5 - 1.5,
                    y: sin(i * 0.9) * 0.8 + 0.2,
                    z: cos(i * 0.4) * 1.2
                )
            )
        }
        return trace
    }

    private static func floats(_ source: SCNGeometrySource) -> [Float] {
        source.data.withUnsafeBytes { raw in
            var values: [Float] = []
            values.reserveCapacity(source.vectorCount * source.componentsPerVector)
            for vector in 0..<source.vectorCount {
                for component in 0..<source.componentsPerVector {
                    let offset = source.dataOffset
                        + vector * source.dataStride
                        + component * source.bytesPerComponent
                    values.append(raw.loadUnaligned(fromByteOffset: offset, as: Float.self))
                }
            }
            return values
        }
    }

    private static func indices(_ element: SCNGeometryElement) -> [UInt32] {
        element.data.withUnsafeBytes { raw in
            var values: [UInt32] = []
            values.reserveCapacity(element.data.count / 4)
            for offset in stride(from: 0, to: element.data.count, by: 4) {
                values.append(raw.loadUnaligned(fromByteOffset: offset, as: UInt32.self))
            }
            return values
        }
    }

    private static func stateDocument(_ state: State) -> [String: String] {
        ["x": Self.exact(state.x), "y": Self.exact(state.y), "z": Self.exact(state.z)]
    }

    private static func vectorDocument(_ vector: SCNVector3) -> [String: String] {
        [
            "x": Self.exact(Double(vector.x)),
            "y": Self.exact(Double(vector.y)),
            "z": Self.exact(Double(vector.z)),
        ]
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

    /// `<repo>/rust/crates/attractor-view/tests/fixtures`.
    private static func fixtureDirectory() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()  // AttractorRenderTests
            .deletingLastPathComponent()  // Tests
            .deletingLastPathComponent()  // Render
            .deletingLastPathComponent()  // repository root
            .appendingPathComponent("rust/crates/attractor-view/tests/fixtures")
    }
}
