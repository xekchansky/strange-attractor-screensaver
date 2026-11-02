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
        // window went away, which the preview host may leave orphaned by never
        // calling `stopAnimation()`.
        if let running = scene, running.isRunning {
            return
        }
        tearDownScene()
        startSimulation()
    }

    /// Build a fresh random simulation and start it. Used both on animation
    /// start and when the health monitor asks for a restart.
    private func startSimulation() {
        do {
            let validation = try AttractorCatalog.bundledValidation()
            // A single broken entry must not blank the screen saver: report it
            // and keep running with the descriptors that passed validation.
            for issue in validation.issues {
                NSLog("AttractorScreensaver: dropping attractor '\(issue.descriptorID)' (\(issue.field)): \(issue.message)")
            }
            guard !validation.valid.isEmpty else {
                NSLog("AttractorScreensaver: no runnable attractors in the bundled catalog")
                return
            }
            // Random attractor + parameters + hyperparameters + initial state.
            let selection = Randomizer.select(from: validation.valid)
            let model = try selection.buildModel()
            // Reduced trail while filling in so the very first frames are not
            // almost empty; the renderer plots the full engine buffer anyway.
            let engine = AttractorEngine(
                model: model,
                initialState: selection.initialState,
                stepSize: selection.settings.stepSize,
                trailCapacity: selection.settings.trailLength
            )

            let newScene = SimulationScene(
                engine: engine,
                selection: selection,
                healthMonitor: SimulationHealthMonitor()
            )
            // Weak self: the scene is owned by this view, so a strong capture
            // here would recreate the retain cycle fixed in the M6 memory work.
            newScene.onNeedsRestart = { [weak self] in
                self?.restartSimulation()
            }
            newScene.view.frame = bounds
            newScene.view.autoresizingMask = [.width, .height]
            addSubview(newScene.view)
            newScene.start()
            scene = newScene
        } catch {
            NSLog("AttractorScreensaver: failed to start simulation: \(error)")
        }
    }

    /// Discard the current run and pick a completely new random one.
    private func restartSimulation() {
        // If the view was already detached, the preview is over: do not build
        // a new scene that nobody will ever see.
        guard superview != nil else { return }
        tearDownScene()
        startSimulation()
    }

    override func stopAnimation() {
        tearDownScene()
        super.stopAnimation()
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        // The preview host never calls `stopAnimation()` and keeps every
        // `ScreensaverView` alive across preview launches, so scenes would pile
        // up forever (≈ +1 live scene ≈ +130 MB per launch). The only signal we
        // get that a preview ended is the view being detached (superview → nil):
        // tear the scene down then, so the retained view stops holding its
        // whole SceneKit scene. The next `startAnimation` builds a fresh one.
        if self.superview == nil {
            tearDownScene()
        }
    }

    /// Stop the running scene and detach its SceneKit view from our hierarchy.
    ///
    /// The preview host keeps every `ScreensaverView` it creates and never
    /// calls `stopAnimation()`, so a scene that is told to stop (or whose view
    /// is detached) must actually leave the view tree: a retained `SCNView`
    /// keeps its whole SceneKit scene alive forever, which is what made the
    /// host process grow by roughly one scene per launch (memory: perf M6).
    /// Dropping the reference lets the scene deallocate; `SimulationScene.deinit`
    /// invalidates its timer and releases the view's scene.
    private func tearDownScene() {
        scene?.stop()
        scene?.view.removeFromSuperview()
        scene = nil
    }

    /// The per-frame loop is driven by `SimulationScene`'s own timer, so the
    /// framework's callback stays empty.
    override func animateOneFrame() {}

    override func draw(_ dirtyRect: NSRect) {
        // The SceneKit view fills `bounds`; nothing further to draw here.
    }
}
