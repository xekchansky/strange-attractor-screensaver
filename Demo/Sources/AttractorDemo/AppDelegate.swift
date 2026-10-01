import AppKit
import AttractorCore
import AttractorRender

/// Entry point of the demo app: parses arguments, builds a selection (manual
/// via `--attractor`/`--param`, or random by default), constructs the engine
/// and launches the AppKit application hosting the SceneKit view.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let selection: AttractorSelection
    private let engine: AttractorEngine
    private var scene: SimulationScene?
    private var window: NSWindow?

    init(selection: AttractorSelection, engine: AttractorEngine) {
        self.selection = selection
        self.engine = engine
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let scene = SimulationScene(engine: engine, selection: selection)
        self.scene = scene

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 900, height: 700),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Attractor Demo"
        window.contentView = scene.view
        scene.view.frame = window.contentView!.bounds
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        self.window = window
        scene.start()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
