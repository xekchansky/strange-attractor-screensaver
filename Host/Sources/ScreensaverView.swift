import AppKit
import ScreenSaver
import AttractorCore
import AttractorRender

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
        // Reuse a still-running scene (re-entrant start). Otherwise tear down
        // and rebuild — including after the scene self-stopped because its
        // window went away (see `SimulationScene.tick`), which the preview host
        // may leave orphaned by never calling `stopAnimation()`.
        if let running = scene, running.isRunning {
            return
        }
        scene?.stop()
        scene = nil
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
            NSLog("AttractorScreensaver: failed to start simulation: \(error)")
        }
    }

    override func stopAnimation() {
        scene?.stop()
        scene = nil
        super.stopAnimation()
    }

    /// The per-frame loop is driven by `SimulationScene`'s own timer, so the
    /// framework's callback stays empty.
    override func animateOneFrame() {}

    override func draw(_ dirtyRect: NSRect) {
        // The SceneKit view fills `bounds`; nothing further to draw here.
    }
}
