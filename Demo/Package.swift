// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AttractorDemo",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        // Debug/visualization app: renders an attractor with a SceneKit view.
        .executable(name: "AttractorDemo", targets: ["AttractorDemo"])
    ],
    dependencies: [
        // Shared pure core, consumed as a local path dependency. No changes
        // to the core package are required for the demo to build and run.
        // The package identity is the directory name, "Core".
        .package(path: "../Core"),
        // Shared SceneKit render layer (SimulationScene, TraceRenderer,
        // CameraController), also consumed by the .appex screensaver host.
        .package(path: "../Render")
    ],
    targets: [
        .executableTarget(
            name: "AttractorDemo",
            dependencies: [
                .product(name: "AttractorCore", package: "Core"),
                .product(name: "AttractorRender", package: "Render")
            ]
        )
    ]
)
