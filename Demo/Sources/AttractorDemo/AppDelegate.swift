import AppKit
import AttractorCore
import AttractorRender

/// Entry point of the demo app: parses arguments, builds a selection (manual
/// via `--attractor`/`--param`, or random by default), constructs the engine
/// and launches the AppKit application hosting the SceneKit view.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let initialSelection: AttractorSelection
    private let initialEngine: AttractorEngine
    private let catalog: [AttractorDescriptor]
    /// Whether a detected bad run should be replaced by a new random one.
    /// Only a fully random launch (no command-line arguments) enables this;
    /// an explicit setup must stay on screen for debugging.
    private let healsAutomatically: Bool
    private var scene: SimulationScene?
    private var window: NSWindow?

    init(
        selection: AttractorSelection,
        engine: AttractorEngine,
        catalog: [AttractorDescriptor],
        healsAutomatically: Bool
    ) {
        self.initialSelection = selection
        self.initialEngine = engine
        self.catalog = catalog
        self.healsAutomatically = healsAutomatically
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 900, height: 700),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Attractor Demo"
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        self.window = window
        startSimulation(selection: initialSelection, engine: initialEngine)
    }

    /// Install a scene for `selection` as the window content and start it.
    private func startSimulation(selection: AttractorSelection, engine: AttractorEngine) {
        let scene = SimulationScene(
            engine: engine,
            selection: selection,
            healthMonitor: healsAutomatically ? SimulationHealthMonitor() : nil
        )
        // Weak self: the scene is owned by this delegate, so a strong capture
        // here would form a retain cycle.
        scene.onNeedsRestart = { [weak self] in
            self?.restartRandomSimulation()
        }
        self.scene = scene

        scene.view.frame = window?.contentView?.bounds ?? NSRect(x: 0, y: 0, width: 900, height: 700)
        scene.view.autoresizingMask = [.width, .height]
        window?.contentView = scene.view
        scene.start()
    }

    /// Replace a bad run with a completely new random selection.
    private func restartRandomSimulation() {
        guard healsAutomatically else { return }
        do {
            let selection = Randomizer.select(from: catalog)
            let engine = AttractorEngine(
                model: try selection.buildModel(),
                initialState: selection.initialState,
                stepSize: selection.settings.stepSize,
                trailCapacity: selection.settings.trailLength
            )
            scene?.stop()
            scene?.view.removeFromSuperview()
            startSimulation(selection: selection, engine: engine)
        } catch {
            NSLog("AttractorDemo: failed to restart simulation: \(error)")
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
