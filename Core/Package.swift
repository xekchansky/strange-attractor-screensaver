// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AttractorCore",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        // Shared pure core: model, config, solver, engine, selection.
        .library(name: "AttractorCore", targets: ["AttractorCore"])
    ],
    dependencies: [
        // Swift Testing — test-only dependency. Deliberate, approved deviation
        // from "no external dependencies": XCTest is not bundled with
        // CommandLineTools, so `swift test` needs this package in CLI-only
        // environments. Pinned to swift-6.3.3-RELEASE (matches our toolchain).
        .package(
            url: "https://github.com/swiftlang/swift-testing.git",
            revision: "48d727cc1cf4eda667c858c501495f1018f69d21"
        )
    ],
    targets: [
        // Pure Swift core with no UI dependencies; fully testable via SPM.
        // Bundles attractors.json so descriptors load from the module bundle.
        .target(
            name: "AttractorCore",
            resources: [.process("Resources")]
        ),
        .testTarget(
            name: "AttractorCoreTests",
            dependencies: [
                "AttractorCore",
                .product(name: "Testing", package: "swift-testing")
            ]
        )
    ]
)
