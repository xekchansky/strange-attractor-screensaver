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
        .package(path: "../Core")
    ],
    targets: [
        .executableTarget(
            name: "AttractorDemo",
            dependencies: [
                .product(name: "AttractorCore", package: "Core")
            ]
        )
    ]
)
