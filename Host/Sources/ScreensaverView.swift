import AppKit
import ScreenSaver
import os
import AttractorCore
import AttractorRender

/// Lifecycle logger for the screensaver host (temporary diagnostic, remove
/// later): used to track whether the host actually calls
/// `startAnimation`/`stopAnimation` and whether our view/scene deallocate.
private let saverLog = Logger(subsystem: "com.example.AttractorSaver", category: "lifecycle")

/// The screensaver host view (M5). Emphasized as the modern `.appex` Screen
/// Saver: the system instantiates this class (via `NSExtensionPrincipalClass`)
/// and drives it through `startAnimation`/`stopAnimation`/`animateOneFrame`.
///
/// It reuses the shared render layer from `AttractorRender` — a random
/// attractor (from the bundled catalog) is selected at startup, the engine is
/// built and the `SimulationScene` runs its own per-frame timer that draws the
/// trace, orbits the camera and overlays the formulas/parameters in a corner.
@MainActor
final class ScreensaverView: ScreenSaver.ScreenSaverView {
    /// The running simulation scene, created in `startAnimation`.
    private var scene: SimulationScene?

    // `init(frame:)`/`init?(coder:)` are inherited from NSView (in this SDK
    // `initWithFrame:` imports as a convenience, `initWithCoder:` as required;
    // our stored property has a default, so no initializer needs overriding).

    override func startAnimation() {
        super.startAnimation()
        saverLog.info("ScreensaverView.startAnimation")
        // Reuse a still-running scene (re-entrant start). Otherwise tear down
        // and rebuild — including after the scene self-stopped because its
        // window went away, which the preview host may leave orphaned by never
        // calling `stopAnimation()`.
        if let running = scene, running.isRunning {
            return
        }
        tearDownScene()
        do {
            let catalog = try AttractorCatalog.bundled()
            // Random attractor + parameters + hyperparameters + initial state.
            let selection = Randomizer.select(from: catalog)
            let model = try selection.buildModel()
            // Reduced trail while filling in so the very first frames are not
            // almost empty; the renderer plots the full engine buffer anyway.
            let engine = AttractorEngine(
                model: model,
                initialState: selection.initialState,
                stepSize: selection.settings.stepSize,
                trailCapacity: selection.settings.trailLength
            )

            let newScene = SimulationScene(engine: engine, selection: selection)
            newScene.view.frame = bounds
            newScene.view.autoresizingMask = [.width, .height]
            addSubview(newScene.view)
            newScene.start()
            scene = newScene
        } catch {
            saverLog.error("AttractorScreensaver: failed to start simulation: \(String(describing: error))")
        }
    }

    override func stopAnimation() {
        saverLog.info("ScreensaverView.stopAnimation")
        tearDownScene()
        super.stopAnimation()
    }

    // Lifecycle observations (temporary diagnostic). Track when the host
    // attaches/detaches our view and whether it actually calls the animation
    // callbacks — on macOS 26 the saver can be driven through the Wallpaper
    // engine without ever reaching `startAnimation`.
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        saverLog.info("ScreensaverView.viewDidMoveToWindow window=\(self.window != nil)")
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        let attached = self.superview != nil
        saverLog.info("ScreensaverView.viewDidMoveToSuperview superview=\(attached)")
        // The preview host never calls `stopAnimation()` and keeps every
        // `ScreensaverView` alive across preview launches, so scenes would pile
        // up forever (≈ +1 live scene ≈ +130 MB per launch). The only signal we
        // get that a preview ended is the view being detached (superview → nil):
        // tear the scene down then, so the retained view stops holding its
        // whole SceneKit scene. The next `startAnimation` builds a fresh one.
        if !attached {
            tearDownScene()
        }
    }

    /// Stop the running scene and detach its SceneKit view from our hierarchy.
    ///
    /// The preview host commonly reuses this `ScreensaverView` across preview
    /// launches, so a scene we are told to stop must actually leave the view
    /// tree: an `SCNView` left as a subview keeps its whole SceneKit scene (and
    /// all the geometry built into it) alive forever, which is what made the
    /// host process grow by roughly one scene per launch (memory: perf M6).
    /// Dropping the reference lets the scene deallocate; `SimulationScene.deinit`
    /// invalidates its timer and releases the view's scene.
    private func tearDownScene() {
        scene?.stop()
        scene?.view.removeFromSuperview()
        scene = nil
    }

    deinit {
        saverLog.info("ScreensaverView dealloc")
    }

    /// The per-frame loop is driven by `SimulationScene`'s own timer, so the
    /// framework's callback stays empty.
    override func animateOneFrame() {}

    override func draw(_ dirtyRect: NSRect) {
        // The SceneKit view fills `bounds`; nothing further to draw here.
    }
}
