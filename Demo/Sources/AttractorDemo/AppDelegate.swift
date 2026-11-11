import AppKit
import QuartzCore
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
    /// Non-nil when the launch should capture frames and exit (documentation).
    private let capture: CaptureController?
    /// Number of integration steps the engine was fast-forwarded before launch.
    /// Slow systems (Lorenz at small step sizes, Sprott family, Moore–Spiegel,
    /// Thomas) need well over 10k steps from a random start before the
    /// attractor's characteristic shape is fully traced out, and the stiff Chua
    /// double scroll (step 0.0004–0.0008) needs ~60k steps to converge from a
    /// random start. Warm-up runs before any rendering, so a few seconds of
    /// extra integration are free in capture mode. Overridable per run with
    /// `--warmup-steps` (e.g. 0 to record the trail growing from the start).
    private static let defaultWarmupSteps = 60000
    /// Capture-mode warm-up override from `--warmup-steps`, if given.
    private let warmupStepsOverride: Int?
    /// Pinned camera orbit angles (radians) from the command line, if given.
    /// `nil` keeps the random azimuth / default elevation the screensaver uses.
    private let cameraAzimuth: Double?
    private let cameraElevation: Double?
    /// Caps restart frequency when a run fails immediately.
    private var restartLimiter = RestartLimiter()
    private var scene: SimulationScene?
    private var window: NSWindow?

    init(
        selection: AttractorSelection,
        engine: AttractorEngine,
        catalog: [AttractorDescriptor],
        healsAutomatically: Bool,
        capture: CaptureController? = nil,
        warmupSteps: Int? = nil,
        cameraAzimuth: Double? = nil,
        cameraElevation: Double? = nil
    ) {
        self.initialSelection = selection
        self.initialEngine = engine
        self.catalog = catalog
        self.healsAutomatically = healsAutomatically
        self.capture = capture
        self.warmupStepsOverride = warmupSteps
        self.cameraAzimuth = cameraAzimuth
        self.cameraElevation = cameraElevation
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        buildCaptureWindowAndScene()
    }

    /// Create the window and start the simulation. Shared by the normal
    /// launch (via the delegate callback) and the capture launch (called
    /// from `main` before `app.run()`, see the note there).
    func buildCaptureWindowAndScene() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1200, height: 900),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Attractor Demo"
        // Capture windows must be placed on a real screen: a window that has
        // never been laid out on-screen gives SceneKit no valid render context,
        // and `snapshot()` segfaults inside SCNRenderer. Capture runs quit by
        // themselves within a few seconds, so a briefly visible window is fine.
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        self.window = window
        startSimulation(selection: initialSelection, engine: initialEngine)
    }

    /// Install a scene for `selection` as the window content and start it.
    private func startSimulation(selection: AttractorSelection, engine: AttractorEngine) {
        var engine = engine
        // Capture runs need the watchdog too: an unlucky Moore–Spiegel or Chua
        // start can settle into a periodic loop, and a capture of that run
        // would show a closed ring instead of the attractor.
        var monitor: SimulationHealthMonitor?
        if healsAutomatically || capture != nil {
            monitor = SimulationHealthMonitor()
        }
        if capture != nil {
            let warmupSteps = warmupStepsOverride ?? Self.defaultWarmupSteps
            // Capture launches fast-forward the engine so screenshots show a
            // fully developed trail instead of a just-born spiral. The same
            // monitor watches the warm-up: a degenerate run is usually already
            // periodic long before the scene starts, and catching it here (not
            // after 90 settle ticks) lets the capture script retry quickly.
            for _ in 0..<warmupSteps {
                do {
                    let advanced = try engine.advance()
                    if let issue = monitor?.observe(advanced) {
                        NSLog("AttractorDemo: capture warm-up failed health check (\(issue))")
                        exit(1)
                    }
                } catch {
                    NSLog("AttractorDemo: warm-up step failed: \(error)")
                    break
                }
            }
        }
        let scene = SimulationScene(
            engine: engine,
            selection: selection,
            healthMonitor: monitor,
            cameraAzimuth: cameraAzimuth,
            cameraElevation: cameraElevation
        )
        // Weak self: the scene is owned by this delegate, so a strong capture
        // here would form a retain cycle.
        scene.onNeedsRestart = { [weak self] in
            // In capture mode a failed run must not silently swap to a new
            // selection: exit non-zero so the calling script retries.
            if self?.capture != nil {
                NSLog("AttractorDemo: capture run failed health check")
                exit(1)
            }
            self?.restartRandomSimulation()
        }
        self.scene = scene

        scene.view.frame = window?.contentView?.bounds ?? NSRect(x: 0, y: 0, width: 1200, height: 900)
        scene.view.autoresizingMask = [.width, .height]
        window?.contentView = scene.view
        if let capture {
            capture.attach(to: scene)
        }
        scene.start()
    }

    /// Replace a bad run with a completely new random selection.
    private func restartRandomSimulation() {
        guard healsAutomatically else { return }
        guard restartLimiter.allowRestart(at: CACurrentMediaTime()) else {
            NSLog("AttractorDemo: too many restarts, keeping the last frame")
            return
        }
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
