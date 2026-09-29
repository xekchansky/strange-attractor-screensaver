import AppKit
import AttractorCore

// ---- Parse command-line arguments ----
let arguments: CommandLineArguments
do {
    arguments = try CommandLineArguments.parse(CommandLine.arguments)
} catch {
    FileHandle.standardError.write(Data(("error: \(error)\n\n" + CommandLineArguments.usage + "\n").utf8))
    exit(2)
}

if arguments.showHelp {
    print(CommandLineArguments.usage)
    exit(0)
}

// ---- Build the selection: manual by request, random otherwise ----
let selection: AttractorSelection
do {
    let catalog = try AttractorCatalog.bundled()
    if let id = arguments.attractorID {
        selection = try DebugConfig.selection(for: id, in: catalog, parameters: arguments.parameters)
    } else {
        selection = Randomizer.select(from: catalog)
    }
} catch {
    FileHandle.standardError.write(Data(("error: failed to build simulation: \(error)\n").utf8))
    exit(1)
}

let model: AttractorModel
do {
    model = try selection.buildModel()
} catch {
    FileHandle.standardError.write(Data(("error: failed to build model: \(error)\n").utf8))
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
