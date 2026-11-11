import AppKit
import AttractorCore

/// Build a hint listing every available attractor and its parameters.
private func attractorHints(_ catalog: [AttractorDescriptor]) -> String {
    let lines = catalog.map { descriptor in
        let parameters = descriptor.parameters.map(\.name).joined(separator: ", ")
        return "  \(descriptor.id)    parameters: \(parameters)"
    }
    return "Available attractors:\n" + lines.joined(separator: "\n")
}

/// Print the usage text (plus attractor hints) to `stderr` or `stdout`.
private func printUsage(to stderr: Bool) {
    let text = CommandLineArguments.usage + "\n\n" + attractorHints(catalog) + "\n"
    if stderr {
        FileHandle.standardError.write(Data(text.utf8))
    } else {
        print(text)
    }
}

// ---- Load the catalog first so we can validate parameters and show hints ----
let catalog: [AttractorDescriptor]
do {
    let validation = try AttractorCatalog.bundledValidation()
    for issue in validation.issues {
        FileHandle.standardError.write(
            Data("warning: dropping attractor '\(issue.descriptorID)' (\(issue.field)): \(issue.message)\n".utf8)
        )
    }
    guard !validation.valid.isEmpty else {
        FileHandle.standardError.write(Data("error: the attractor catalog has no runnable entries\n".utf8))
        exit(1)
    }
    catalog = validation.valid
} catch {
    FileHandle.standardError.write(Data("error: failed to load attractor catalog: \(error)\n".utf8))
    exit(1)
}

// ---- Parse command-line arguments ----
let arguments: CommandLineArguments
do {
    arguments = try CommandLineArguments.parse(CommandLine.arguments)
} catch {
    FileHandle.standardError.write(Data("error: \(error)\n\n".utf8))
    printUsage(to: true)
    exit(2)
}

if arguments.showHelp {
    printUsage(to: false)
    exit(0)
}

// ---- Resolve the attractor and validate parameters against the catalog ----
var descriptor: AttractorDescriptor?
if let id = arguments.attractorID {
    guard let match = catalog.first(where: { $0.id == id }) else {
        FileHandle.standardError.write(Data("error: unknown attractor '\(id)'.\n\n".utf8))
        printUsage(to: true)
        exit(2)
    }
    descriptor = match
}

var parameters: [String: Double] = [:]
if !arguments.parameters.isEmpty {
    guard let descriptor else {
        FileHandle.standardError.write(
            Data("error: specify an attractor with -a <id> to pass parameters.\n\n".utf8)
        )
        printUsage(to: true)
        exit(2)
    }
    let validNames = Set(descriptor.parameters.map(\.name))
    let available = descriptor.parameters.map(\.name).joined(separator: ", ")

    for (name, raw) in arguments.parameters {
        guard validNames.contains(name) else {
            FileHandle.standardError.write(
                Data("error: unknown parameter '\(name)' for '\(descriptor.id)'. Available: \(available)\n\n".utf8)
            )
            printUsage(to: true)
            exit(2)
        }
        guard let value = Double(raw), value.isFinite else {
            FileHandle.standardError.write(
                Data("error: invalid value '\(raw)' for parameter '\(name)'. Available: \(available)\n\n".utf8)
            )
            printUsage(to: true)
            exit(2)
        }
        parameters[name] = value
    }
}

// ---- Build the selection: manual by request, random otherwise ----
let selection: AttractorSelection
do {
    if let descriptor, !parameters.isEmpty {
        // Fully pinned run: parameters are explicit, everything else keeps the
        // deterministic midpoint defaults for reproducible debugging.
        selection = try DebugConfig.selection(for: descriptor.id, in: catalog, parameters: parameters)
    } else if let descriptor {
        // Attractor fixed by id, parameters sampled from its declared ranges:
        // several catalog midpoints (Thomas, Chua, Moore–Spiegel) are exactly
        // fixed points or limit cycles, so a midpoint run would be degenerate.
        selection = try Randomizer.select(attractor: descriptor)
    } else {
        selection = try Randomizer.select(from: catalog)
    }
} catch {
    FileHandle.standardError.write(Data("error: failed to build simulation: \(error)\n".utf8))
    exit(1)
}

// Overlay any simulation hyperparameter overrides on top of the selection.
let baseSettings = selection.settings
let overrides = arguments.overrides
let effectiveSettings = SimulationSettings(
    stepSize: overrides.stepSize ?? baseSettings.stepSize,
    simulationSpeed: overrides.simulationSpeed ?? baseSettings.simulationSpeed,
    cameraRotationSpeed: overrides.cameraRotationSpeed ?? baseSettings.cameraRotationSpeed,
    trailColorSpeed: overrides.trailColorSpeed ?? baseSettings.trailColorSpeed,
    trailLength: overrides.trailLength ?? baseSettings.trailLength,
    colorMode: overrides.colorMode ?? baseSettings.colorMode
)
let settingsIssues = effectiveSettings.validationIssues
if !settingsIssues.isEmpty {
    for issue in settingsIssues {
        FileHandle.standardError.write(Data("error: \(issue)\n".utf8))
    }
    exit(2)
}

let effectiveSelection = AttractorSelection(
    descriptor: selection.descriptor,
    parameters: selection.parameters,
    settings: effectiveSettings,
    // `--initial-state` overrides the sampled start: some catalog midpoints
    // fall into a fixed point's basin (Thomas), so a pinned capture has to
    // choose its own chaotic start.
    initialState: arguments.initialState ?? selection.initialState
)

let model: AttractorModel
do {
    model = try effectiveSelection.buildModel()
} catch {
    FileHandle.standardError.write(Data("error: failed to build model: \(error)\n".utf8))
    exit(1)
}

let engine = AttractorEngine(
    model: model,
    initialState: effectiveSelection.initialState,
    stepSize: effectiveSelection.settings.stepSize,
    trailCapacity: effectiveSelection.settings.trailLength
)

// ---- Capture request (if any) ----
let captureController = arguments.capture.map { request in
    let scale = arguments.captureScale ?? 2
    switch request {
    case .screenshot(let outputPath):
        return CaptureController(mode: .screenshot(outputPath: outputPath), captureScale: scale)
    case .record(let directory, let seconds, let stride):
        return CaptureController(
            mode: .record(outputDirectory: directory, seconds: seconds, frameStride: stride),
            captureScale: scale
        )
    }
}

// ---- Launch the AppKit application ----
let app = NSApplication.shared
app.setActivationPolicy(.regular)
// Only a launch with no arguments is the pure random showcase; that is the
// one that self-heals. Any explicit option pins a setup for debugging.
let healsAutomatically = CommandLine.arguments.count == 1
let delegate = AppDelegate(
    selection: effectiveSelection,
    engine: engine,
    catalog: catalog,
    healsAutomatically: healsAutomatically,
    capture: captureController,
    warmupSteps: arguments.warmupSteps,
    cameraAzimuth: arguments.cameraAzimuthDegrees.map { $0 * .pi / 180 },
    cameraElevation: arguments.cameraElevationDegrees.map { $0 * .pi / 180 }
)
app.delegate = delegate
// In capture mode the delegate's `applicationDidFinishLaunching` is not
// reliably dispatched in a background GUI session (XPC services are not
// reachable), so the window and scene are built up front; the capture
// controller then quits the process itself.
if captureController != nil {
    delegate.buildCaptureWindowAndScene()
}
app.run()
