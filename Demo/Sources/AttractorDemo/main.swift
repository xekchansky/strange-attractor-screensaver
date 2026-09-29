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
    catalog = try AttractorCatalog.bundled()
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
        guard let value = Double(raw) else {
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
    if let descriptor {
        selection = try DebugConfig.selection(for: descriptor.id, in: catalog, parameters: parameters)
    } else {
        selection = Randomizer.select(from: catalog)
    }
} catch {
    FileHandle.standardError.write(Data("error: failed to build simulation: \(error)\n".utf8))
    exit(1)
}

let model: AttractorModel
do {
    model = try selection.buildModel()
} catch {
    FileHandle.standardError.write(Data("error: failed to build model: \(error)\n".utf8))
    exit(1)
}

let engine = AttractorEngine(
    model: model,
    initialState: selection.initialState,
    stepSize: selection.settings.stepSize,
    trailCapacity: selection.settings.trailLength
)

// ---- Launch the AppKit application ----
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let delegate = AppDelegate(selection: selection, engine: engine)
app.delegate = delegate
app.run()
