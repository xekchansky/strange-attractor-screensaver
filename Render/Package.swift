// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AttractorRender",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        // Reusable SceneKit render layer on top of the pure core: the
        // SimulationScene (view + engine loop + formula overlay), the
        // TraceRenderer (gradient polyline) and the CameraController (orbit).
        // Shared by the Demo executable and the .appex screensaver host.
        .library(name: "AttractorRender", targets: ["AttractorRender"])
    ],
    dependencies: [
        // Shared pure core, consumed as a local path dependency.
        // The package identity is the directory name, "Core".
        .package(path: "../Core")
    ],
    targets: [
        // UI rendering (SceneKit/AppKit) over the core. Not part of the pure
        // testable core by design; kept here so both consumers share one copy.
        .target(
            name: "AttractorRender",
            dependencies: [
                .product(name: "AttractorCore", package: "Core")
            ]
        ),
        // Golden-fixture generator for the Rust view layer, disabled unless
        // ATTRACTOR_DUMP_FIXTURES=1. Not run by CI; it pins the Rust port to
        // the Swift camera and ribbon implementation.
        .testTarget(
            name: "AttractorRenderTests",
            dependencies: ["AttractorRender"]
        )
    ]
)
